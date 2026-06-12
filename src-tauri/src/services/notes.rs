use crate::json_io::write_json_atomic;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fmt, fs, io,
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

#[cfg(target_os = "macos")]
const DEFAULT_MACOS_GLOBAL_SHORTCUT: &str = "Command+Option+N";
#[cfg(target_os = "macos")]
const LEGACY_MACOS_GLOBAL_SHORTCUTS: [&str; 5] = [
    "Option+Space",
    "Alt+Space",
    "Ctrl+Option+Space",
    "Control+Option+Space",
    "Ctrl+Alt+Space",
];
const MACOS_SHORTCUT_MIGRATION_MARKER: &str = ".macos-shortcut-default-v3";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_locale")]
    pub locale: String,
    pub notes_dir: String,
    pub global_shortcut: String,
    pub close_to_tray: bool,
    pub autostart: bool,
    pub default_view_mode: String,
    #[serde(default = "default_note_auto_save")]
    pub note_auto_save: bool,
    #[serde(default = "default_note_surface_auto_save")]
    pub note_surface_auto_save: bool,
    #[serde(default = "default_tile_color")]
    pub tile_color: String,
    #[serde(default = "default_tile_color_mode")]
    pub tile_color_mode: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_surface_font_size")]
    pub surface_font_size: u32,
    #[serde(default = "default_tab_indent_size")]
    pub tab_indent_size: u32,
    #[serde(default = "default_external_file_auto_save")]
    pub external_file_auto_save: bool,
    #[serde(default)]
    pub background_image_path: String,
    #[serde(default = "default_background_fit")]
    pub background_fit: String,
    #[serde(default = "default_background_dim")]
    pub background_dim: f64,
    #[serde(default = "default_background_blur")]
    pub background_blur: f64,
    #[serde(default = "default_background_scale")]
    pub background_scale: f64,
    #[serde(default = "default_background_position")]
    pub background_position_x: f64,
    #[serde(default = "default_background_position")]
    pub background_position_y: f64,
    #[serde(default = "default_remember_surface_size")]
    pub remember_surface_size: bool,
    #[serde(default = "default_tile_ctrl_close")]
    pub tile_ctrl_close: bool,
    #[serde(default)]
    pub tile_render_markdown: bool,
    #[serde(default)]
    pub render_html_markdown: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_height: Option<u32>,
    #[serde(default = "default_toggle_visibility_shortcut")]
    pub toggle_visibility_shortcut: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_base_dir: Option<String>,
    #[serde(default = "default_open_at_cursor")]
    pub open_at_cursor: bool,

    // OSS 云同步配置
    #[serde(default)]
    pub oss_provider: String,
    #[serde(default)]
    pub oss_endpoint: String,
    #[serde(default)]
    pub oss_bucket: String,
    #[serde(default)]
    pub oss_access_key_id: String,
    #[serde(default)]
    pub oss_remote_prefix: String,

    // 同步设置
    #[serde(default)]
    pub sync_on_startup: bool,
    #[serde(default)]
    pub sync_interval: String,
    #[serde(default = "default_sync_strategy")]
    pub sync_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveNoteRequest {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetadata {
    pub id: String,
    pub title: String,
    pub file_name: String,
    #[serde(default)]
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub word_count: usize,
    pub preview: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trashed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub file_name: String,
    #[serde(default)]
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub word_count: usize,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, String>,
}

impl AppError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: BTreeMap::new(),
        }
    }

    fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    fn note_not_found(id: &str) -> Self {
        Self::new("noteNotFound", format!("Note {id} was not found")).with_detail("noteId", id)
    }

    fn unsupported_file() -> Self {
        Self::new("unsupportedFile", "只支持导入 .md 文件")
    }

    fn category_name_empty() -> Self {
        Self::new("categoryNameEmpty", "分类名不能为空")
    }

    fn category_name_invalid_chars() -> Self {
        Self::new("categoryNameInvalidChars", "分类名不能包含特殊字符")
    }

    fn category_not_found(name: &str) -> Self {
        Self::new("categoryNotFound", format!("分类「{name}」不存在")).with_detail("category", name)
    }

    fn category_already_exists(name: &str) -> Self {
        Self::new("categoryAlreadyExists", format!("分类「{name}」已存在"))
            .with_detail("category", name)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::new("io", error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("json", error.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(error: tauri::Error) -> Self {
        Self::new("tauri", error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MetadataFile {
    notes: Vec<NoteMetadata>,
    #[serde(default)]
    trashed_notes: Vec<NoteMetadata>,
}

#[derive(Debug, Clone)]
pub struct NoteStore {
    pub(crate) base_dir: PathBuf,
}

pub fn default_store() -> Result<NoteStore, AppError> {
    Ok(NoteStore::new(default_base_dir()?))
}

fn default_base_dir() -> Result<PathBuf, AppError> {
    if let Ok(path) = env::var("FLORAL_NOTEPAPER_DATA_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(dir) = dirs::data_dir() {
        return Ok(dir.join("花笺"));
    }

    if let Some(dir) = dirs::document_dir() {
        return Ok(dir.join("花笺"));
    }

    Ok(env::current_dir()?.join("data"))
}

fn known_data_migration_candidates() -> Vec<PathBuf> {
    known_data_migration_candidates_for(env::var("HOME").ok(), env::var("USERPROFILE").ok())
}

fn known_data_migration_candidates_for(
    home: Option<String>,
    userprofile: Option<String>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(home) = home {
        let home = PathBuf::from(home);
        candidates.push(home.join("Documents").join("花笺"));
        candidates.push(
            home.join("Library")
                .join("Application Support")
                .join("花笺"),
        );
    }
    if let Some(profile) = userprofile {
        let profile = PathBuf::from(profile);
        candidates.push(profile.join("Documents").join("花笺"));
    }

    candidates
}

fn move_or_copy_dir(from: &Path, to: &Path) -> Result<(), AppError> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // cross-filesystem fallback
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    copy_dir_recursive(from, to)?;
    fs::remove_dir_all(from)?;
    Ok(())
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn is_filesystem_root(path: &Path) -> bool {
    let path = path.to_string_lossy();
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return true;
    }
    // Windows drive root: "C:" or "D:" etc.
    if trimmed.len() == 2 {
        let bytes = trimmed.as_bytes();
        if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return true;
        }
    }
    false
}

fn ensure_notes_suffix(dir: &str) -> String {
    let path = Path::new(dir);
    if path.file_name().and_then(|n| n.to_str()) == Some("notes") {
        return dir.to_string();
    }
    path.join("notes").to_string_lossy().to_string()
}

fn is_safe_notes_dir(path: &Path) -> Result<(), AppError> {
    if is_filesystem_root(path) {
        return Err(AppError::new(
            "unsafePath",
            "不能将磁盘根目录设为笔记目录，请选择一个子文件夹",
        ));
    }

    let normalized = path.to_string_lossy().to_lowercase();
    let blocked = [
        "\\windows",
        "\\program files",
        "\\program files (x86)",
        "\\system32",
        "\\syswow64",
    ];
    for suffix in &blocked {
        if normalized.ends_with(suffix) {
            return Err(AppError::new(
                "unsafePath",
                format!("不能将系统目录「{}」设为笔记目录", path.display()),
            ));
        }
    }

    // Must have at least 2 real path components (e.g. D:\Something, not just D:\)
    let real_components = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    if real_components == 0 {
        return Err(AppError::new(
            "unsafePath",
            "笔记目录路径不合法，请选择一个具体的文件夹",
        ));
    }

    Ok(())
}

/// Normalize a category path: use `/` as the canonical separator, trim
/// whitespace from each segment, and collapse empty segments.
fn normalize_category_path(category: &str) -> String {
    category
        .replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_valid_category_segment(segment: &str) -> bool {
    if segment.is_empty() || segment == "." || segment == ".." {
        return false;
    }
    if segment.contains('/') || segment.contains('\\') || segment.contains(':') {
        return false;
    }
    if segment.chars().any(|c| c.is_control()) {
        return false;
    }
    true
}

fn validate_category_path(name: &str) -> Result<String, AppError> {
    let normalized_input = name.replace('\\', "/");
    let raw_segments: Vec<&str> = normalized_input.split('/').collect();
    if raw_segments.iter().map(|s| s.trim()).any(|s| s.is_empty()) {
        return Err(AppError::category_name_invalid_chars());
    }

    let normalized = normalize_category_path(name);
    if normalized.is_empty() {
        return Err(AppError::category_name_empty());
    }
    for segment in normalized.split('/') {
        if !is_valid_category_segment(segment) {
            return Err(AppError::category_name_invalid_chars());
        }
    }
    Ok(normalized)
}

impl NoteStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.base_dir.join("metadata.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    #[cfg(target_os = "macos")]
    fn macos_shortcut_migration_path(&self) -> PathBuf {
        self.base_dir.join(MACOS_SHORTCUT_MIGRATION_MARKER)
    }

    pub fn load_config(&self) -> Result<AppConfig, AppError> {
        self.ensure_base_dir()?;
        let path = self.config_path();
        if !path.exists() && self.is_default_base_dir() {
            self.migrate_from_known_locations()?;
        }
        if !path.exists() {
            let config = self.default_config();
            self.save_config(config.clone())?;
            self.mark_macos_shortcut_migration_handled()?;
            return Ok(config);
        }

        let mut config: AppConfig = serde_json::from_str(&fs::read_to_string(&path)?)?;
        self.migrate_base_dir_if_changed(&mut config)?;
        if is_safe_notes_dir(Path::new(&config.notes_dir)).is_err() {
            config.notes_dir = self.default_config().notes_dir;
        }
        config.last_known_base_dir = Some(self.base_dir.to_string_lossy().to_string());
        write_json_atomic(&path, &config)?;
        fs::create_dir_all(&config.notes_dir)?;
        if self.migrate_macos_shortcut_default(&mut config)? {
            write_json_atomic(&path, &config)?;
        }
        Ok(config)
    }

    pub fn save_config(&self, mut config: AppConfig) -> Result<AppConfig, AppError> {
        self.ensure_base_dir()?;
        config.notes_dir = ensure_notes_suffix(&config.notes_dir);
        config.tab_indent_size = config.tab_indent_size.clamp(1, 8);
        is_safe_notes_dir(Path::new(&config.notes_dir))?;
        fs::create_dir_all(&config.notes_dir)?;
        write_json_atomic(&self.config_path(), &config)?;
        Ok(config)
    }

    pub fn list_notes(&self) -> Result<Vec<NoteMetadata>, AppError> {
        self.ensure_storage()?;
        let mut metadata = self.load_metadata()?.notes;
        metadata.retain(|note| {
            self.note_path_in_category(&note.file_name, &note.category)
                .exists()
        });
        metadata.sort_by_key(|note| std::cmp::Reverse(note.updated_at));
        Ok(metadata)
    }

    pub fn read_note(&self, id: &str) -> Result<Note, AppError> {
        self.ensure_storage()?;
        let metadata = self.find_metadata(id)?;
        let content = fs::read_to_string(
            self.note_path_in_category(&metadata.file_name, &metadata.category),
        )?;
        Ok(Note {
            id: metadata.id,
            title: metadata.title,
            file_name: metadata.file_name,
            category: metadata.category,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            word_count: metadata.word_count,
            content,
        })
    }

    pub fn create_note(&self, request: SaveNoteRequest) -> Result<Note, AppError> {
        self.ensure_storage()?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let file_name = self.file_name_for(&id, &request.title);
        let word_count = count_words(&request.content);
        let category = normalize_category_path(&request.category);
        let note_path = self.note_path_in_category(&file_name, &category);
        if let Some(parent) = note_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let metadata = NoteMetadata {
            id: id.clone(),
            title: request.title,
            file_name: file_name.clone(),
            category: category.clone(),
            created_at: now,
            updated_at: now,
            word_count,
            preview: preview(&request.content),
            trashed_at: None,
        };

        fs::write(&note_path, &request.content)?;
        let mut metadata_file = self.load_metadata()?;
        metadata_file.notes.push(metadata.clone());
        self.save_metadata(&metadata_file)?;

        Ok(Note {
            id,
            title: metadata.title,
            file_name,
            category,
            created_at: now,
            updated_at: now,
            word_count,
            content: request.content,
        })
    }

    pub fn update_note(&self, id: &str, request: SaveNoteRequest) -> Result<Note, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let note = metadata_file
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::note_not_found(id))?;

        let old_file_name = note.file_name.clone();
        let old_category = note.category.clone();
        let new_file_name = self.file_name_for(id, &request.title);
        let new_category = normalize_category_path(&request.category);
        let now = Utc::now();
        let word_count = count_words(&request.content);

        let new_path = self.note_path_in_category(&new_file_name, &new_category);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&new_path, &request.content)?;

        if old_file_name != new_file_name || old_category != new_category {
            let old_path = self.note_path_in_category(&old_file_name, &old_category);
            if old_path.exists() && old_path != new_path {
                trash::delete(&old_path)
                    .map_err(|e| AppError::new("trash", format!("移入回收站失败: {e}")))?;
            }
        }

        note.title = request.title;
        note.file_name = new_file_name.clone();
        note.category = new_category.clone();
        note.updated_at = now;
        note.word_count = word_count;
        note.preview = preview(&request.content);

        let result = Note {
            id: note.id.clone(),
            title: note.title.clone(),
            file_name: note.file_name.clone(),
            category: new_category,
            created_at: note.created_at,
            updated_at: note.updated_at,
            word_count: note.word_count,
            content: request.content,
        };

        self.save_metadata(&metadata_file)?;
        Ok(result)
    }

    pub fn delete_note(&self, id: &str) -> Result<(), AppError> {
        self.trash_note(id)
    }

    fn trash_dir(&self, note_id: &str) -> PathBuf {
        self.base_dir.join(".trash").join(note_id)
    }

    pub fn trash_note(&self, id: &str) -> Result<(), AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let index = metadata_file
            .notes
            .iter()
            .position(|note| note.id == id)
            .ok_or_else(|| AppError::note_not_found(id))?;
        let mut metadata = metadata_file.notes.remove(index);
        metadata.trashed_at = Some(Utc::now());

        // Move note file to .trash/{id}/
        let src_path = self.note_path_in_category(&metadata.file_name, &metadata.category);
        let trash_dir = self.trash_dir(id);
        fs::create_dir_all(&trash_dir)?;
        if src_path.exists() {
            let dst_path = trash_dir.join(&metadata.file_name);
            if fs::rename(&src_path, &dst_path).is_err() {
                // cross-filesystem fallback
                fs::copy(&src_path, &dst_path)?;
                let _ = fs::remove_file(&src_path);
            }
        }

        // Move images to .trash/{id}/images/
        let images_src = self.images_dir(id);
        if images_src.exists() {
            let images_dst = trash_dir.join("images");
            if fs::rename(&images_src, &images_dst).is_err() {
                copy_dir_recursive(&images_src, &images_dst)?;
                let _ = fs::remove_dir_all(&images_src);
            }
        }

        metadata_file.trashed_notes.push(metadata);
        self.save_metadata(&metadata_file)?;
        Ok(())
    }

    pub fn list_trashed_notes(&self) -> Result<Vec<NoteMetadata>, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let mut metadata = metadata_file.trashed_notes.clone();
        let original_len = metadata.len();
        // Only return entries whose .trash/{id}/ directory actually exists on disk
        metadata.retain(|note| self.trash_dir(&note.id).exists());
        // Auto-clean stale entries from metadata.json
        if metadata.len() != original_len {
            metadata_file
                .trashed_notes
                .retain(|n| self.trash_dir(&n.id).exists());
            let _ = self.save_metadata(&metadata_file);
        }
        metadata.sort_by_key(|note| std::cmp::Reverse(note.trashed_at));
        Ok(metadata)
    }

    pub fn restore_note(&self, id: &str) -> Result<NoteMetadata, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let index = metadata_file
            .trashed_notes
            .iter()
            .position(|note| note.id == id)
            .ok_or_else(|| AppError::note_not_found(id))?;
        let mut metadata = metadata_file.trashed_notes.remove(index);

        let trash_dir = self.trash_dir(id);

        // Move note file back
        let trash_file = trash_dir.join(&metadata.file_name);
        if trash_file.exists() {
            let dst_path = self.note_path_in_category(&metadata.file_name, &metadata.category);
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if fs::rename(&trash_file, &dst_path).is_err() {
                fs::copy(&trash_file, &dst_path)?;
                let _ = fs::remove_file(&trash_file);
            }
        }

        // Move images back
        let trash_images = trash_dir.join("images");
        if trash_images.exists() {
            let dst_images = self.images_dir(id);
            if fs::rename(&trash_images, &dst_images).is_err() {
                copy_dir_recursive(&trash_images, &dst_images)?;
                let _ = fs::remove_dir_all(&trash_images);
            }
        }

        // Clean up trash directory
        let _ = fs::remove_dir_all(&trash_dir);

        metadata.trashed_at = None;
        let result = metadata.clone();
        metadata_file.notes.push(metadata);
        self.save_metadata(&metadata_file)?;
        Ok(result)
    }

    /// Move a trashed note back to active notes for sync download.
    /// Returns true if the note was found in trashed_notes and restored.
    pub fn prepare_trashed_for_download(&self, note_id: &str) -> Result<bool, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let index = match metadata_file
            .trashed_notes
            .iter()
            .position(|note| note.id == note_id)
        {
            Some(i) => i,
            None => return Ok(false),
        };
        let mut note = metadata_file.trashed_notes.remove(index);
        note.trashed_at = None;
        metadata_file.notes.push(note);
        // Clean up any leftover trash directory
        let trash_dir = self.trash_dir(note_id);
        if trash_dir.exists() {
            let _ = fs::remove_dir_all(&trash_dir);
        }
        self.save_metadata(&metadata_file)?;
        Ok(true)
    }

    pub fn permanent_delete_note(&self, id: &str) -> Result<(), AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let index = metadata_file
            .trashed_notes
            .iter()
            .position(|note| note.id == id)
            .ok_or_else(|| AppError::note_not_found(id))?;
        metadata_file.trashed_notes.remove(index);

        let trash_dir = self.trash_dir(id);
        if trash_dir.exists() {
            fs::remove_dir_all(&trash_dir)?;
        }

        self.save_metadata(&metadata_file)?;
        Ok(())
    }

    pub fn empty_trash(&self) -> Result<Vec<String>, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let ids: Vec<String> = metadata_file
            .trashed_notes
            .iter()
            .map(|n| n.id.clone())
            .collect();
        for id in &ids {
            let trash_dir = self.trash_dir(id);
            if trash_dir.exists() {
                let _ = fs::remove_dir_all(&trash_dir);
            }
        }
        metadata_file.trashed_notes.clear();
        self.save_metadata(&metadata_file)?;
        Ok(ids)
    }

    pub fn images_dir(&self, note_id: &str) -> PathBuf {
        self.base_dir.join("images").join(note_id)
    }

    pub fn save_image(
        &self,
        note_id: &str,
        data: &[u8],
        extension: &str,
    ) -> Result<String, AppError> {
        self.ensure_storage()?;
        self.find_metadata(note_id)?;

        const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];
        let ext = extension.to_ascii_lowercase();
        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            return Err(AppError::new(
                "unsupportedImageFormat",
                format!("不支持的图片格式: {ext}"),
            ));
        }

        let dir = self.images_dir(note_id);
        fs::create_dir_all(&dir)?;

        let file_name = format!("{}.{}", Uuid::new_v4(), ext);
        fs::write(dir.join(&file_name), data)?;

        Ok(format!("images/{note_id}/{file_name}"))
    }

    pub fn delete_note_images(&self, note_id: &str) -> Result<(), AppError> {
        let dir = self.images_dir(note_id);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn clean_unused_images(
        &self,
        note_id: &str,
        content: &str,
    ) -> Result<Vec<String>, AppError> {
        let dir = self.images_dir(note_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut removed = Vec::new();
        let mut remaining = 0usize;
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let relative = format!("images/{note_id}/{file_name}");
            if !content.contains(&relative) {
                fs::remove_file(&path)?;
                removed.push(file_name);
            } else {
                remaining += 1;
            }
        }

        if remaining == 0 {
            let _ = fs::remove_dir(&dir);
        }

        Ok(removed)
    }

    pub fn import_markdown_file(&self, path: &Path, category: &str) -> Result<Note, AppError> {
        if !is_markdown_path(path) {
            return Err(AppError::unsupported_file());
        }

        let content = fs::read_to_string(path)?;
        let title = imported_markdown_title(path, &content);
        self.create_note(SaveNoteRequest {
            title,
            content,
            category: category.to_string(),
        })
    }

    pub fn export_markdown_file(&self, id: &str, path: &Path) -> Result<(), AppError> {
        let note = self.read_note(id)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, note.content)?;
        Ok(())
    }

    pub fn list_categories(&self) -> Result<Vec<String>, AppError> {
        let notes_dir = self.notes_dir()?;
        fs::create_dir_all(&notes_dir)?;
        let mut categories = Vec::new();
        self.collect_categories(&notes_dir, "", &mut categories)?;
        categories.sort();
        Ok(categories)
    }

    fn collect_categories(
        &self,
        dir: &Path,
        prefix: &str,
        out: &mut Vec<String>,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let category = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", prefix, name)
                };
                out.push(category.clone());
                self.collect_categories(&path, &category, out)?;
            }
        }
        Ok(())
    }

    pub fn create_category(&self, name: &str) -> Result<(), AppError> {
        let category = validate_category_path(name)?;
        let path = self.category_path(&category);
        fs::create_dir_all(&path)?;
        Ok(())
    }

    pub fn rename_category(&self, old_name: &str, new_name: &str) -> Result<(), AppError> {
        let new_category = validate_category_path(new_name)?;
        let old_category = normalize_category_path(old_name);
        if old_category.is_empty() {
            return Err(AppError::category_name_empty());
        }
        if new_category == old_category {
            return Ok(());
        }

        let old_path = self.category_path(&old_category);
        let new_path = self.category_path(&new_category);
        if !old_path.exists() {
            return Err(AppError::category_not_found(old_name));
        }
        if new_path.exists() {
            return Err(AppError::category_already_exists(new_name));
        }
        fs::rename(&old_path, &new_path)?;

        let old_prefix = format!("{}/", old_category);
        let mut metadata_file = self.load_metadata()?;
        for note in &mut metadata_file.notes {
            if note.category == old_category {
                note.category = new_category.clone();
            } else if note.category.starts_with(&old_prefix) {
                note.category = format!("{}{}", new_category, &note.category[old_category.len()..]);
            }
        }
        self.save_metadata(&metadata_file)?;
        Ok(())
    }

    pub fn delete_category(&self, name: &str) -> Result<(), AppError> {
        let category = normalize_category_path(name);
        if category.is_empty() {
            return Err(AppError::category_name_empty());
        }
        let notes_dir = self.notes_dir()?;
        let category_path = self.category_path(&category);
        let dir_exists = category_path.exists();

        if dir_exists {
            // Safety: ensure the category path is actually inside notes_dir
            let canon_notes = fs::canonicalize(&notes_dir).unwrap_or_else(|_| notes_dir.clone());
            let canon_cat =
                fs::canonicalize(&category_path).unwrap_or_else(|_| category_path.clone());
            if !canon_cat.starts_with(&canon_notes) || canon_cat == canon_notes {
                return Err(AppError::new(
                    "unsafePath",
                    format!(
                        "拒绝删除「{}」：路径不在笔记目录内",
                        category_path.display()
                    ),
                ));
            }

            // Move all notes in this category tree to uncategorized (root)
            let mut metadata_file = self.load_metadata()?;
            let prefix = format!("{}/", category);
            for note in &mut metadata_file.notes {
                if note.category == category || note.category.starts_with(&prefix) {
                    let old_path = self.note_path_in_category(&note.file_name, &note.category);
                    let new_path = notes_dir.join(&note.file_name);
                    if old_path.exists() {
                        fs::rename(&old_path, &new_path)?;
                    }
                    note.category = String::new();
                }
            }
            self.save_metadata(&metadata_file)?;

            // Move to recycle bin instead of permanent deletion
            trash::delete(&category_path)
                .map_err(|e| AppError::new("trash", format!("移入回收站失败: {e}")))?;
        } else {
            // Directory already gone (manually deleted outside the app);
            // clean up any stale metadata references.
            let mut metadata_file = self.load_metadata()?;
            let mut changed = false;
            let prefix = format!("{}/", category);
            for note in &mut metadata_file.notes {
                if note.category == category || note.category.starts_with(&prefix) {
                    note.category = String::new();
                    changed = true;
                }
            }
            if changed {
                self.save_metadata(&metadata_file)?;
            }
        }
        Ok(())
    }

    pub fn move_note_to_category(
        &self,
        id: &str,
        new_category: &str,
    ) -> Result<NoteMetadata, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let note = metadata_file
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::note_not_found(id))?;

        let old_category = note.category.clone();
        let new_category = normalize_category_path(new_category);
        if old_category == new_category {
            return Ok(note.clone());
        }

        let old_path = self.note_path_in_category(&note.file_name, &old_category);
        let new_path = self.note_path_in_category(&note.file_name, &new_category);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if old_path.exists() {
            fs::rename(&old_path, &new_path)?;
        }

        note.category = new_category;
        let result = note.clone();
        self.save_metadata(&metadata_file)?;
        Ok(result)
    }

    fn default_config(&self) -> AppConfig {
        AppConfig {
            locale: default_locale(),
            notes_dir: self.base_dir.join("notes").to_string_lossy().to_string(),
            #[cfg(target_os = "macos")]
            global_shortcut: DEFAULT_MACOS_GLOBAL_SHORTCUT.into(),
            #[cfg(not(target_os = "macos"))]
            global_shortcut: "Ctrl+Space".into(),
            close_to_tray: true,
            autostart: false,
            default_view_mode: "split".into(),
            note_auto_save: true,
            note_surface_auto_save: true,
            tile_color: default_tile_color(),
            tile_color_mode: default_tile_color_mode(),
            theme: default_theme(),
            font_size: default_font_size(),
            surface_font_size: default_surface_font_size(),
            tab_indent_size: default_tab_indent_size(),
            external_file_auto_save: default_external_file_auto_save(),
            background_image_path: String::new(),
            background_fit: default_background_fit(),
            background_dim: default_background_dim(),
            background_blur: default_background_blur(),
            background_scale: default_background_scale(),
            background_position_x: default_background_position(),
            background_position_y: default_background_position(),
            remember_surface_size: default_remember_surface_size(),
            tile_ctrl_close: default_tile_ctrl_close(),
            tile_render_markdown: false,
            render_html_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: default_toggle_visibility_shortcut(),
            last_known_base_dir: Some(self.base_dir.to_string_lossy().to_string()),
            open_at_cursor: default_open_at_cursor(),
            oss_provider: String::new(),
            oss_endpoint: String::new(),
            oss_bucket: String::new(),
            oss_access_key_id: String::new(),
            oss_remote_prefix: String::new(),
            sync_on_startup: false,
            sync_interval: "off".into(),
            sync_strategy: default_sync_strategy(),
        }
    }

    fn migrate_from_known_locations(&self) -> Result<(), AppError> {
        let current = &self.base_dir;
        if current.join("config.json").exists() {
            return Ok(());
        }
        for old_dir in known_data_migration_candidates() {
            if old_dir != *current && old_dir.join("config.json").exists() {
                eprintln!(
                    "migrating data from {} to {}",
                    old_dir.display(),
                    current.display()
                );
                return move_or_copy_dir(&old_dir, current);
            }
        }
        Ok(())
    }

    fn is_default_base_dir(&self) -> bool {
        default_base_dir()
            .map(|default_dir| default_dir == self.base_dir)
            .unwrap_or(false)
    }

    fn migrate_base_dir_if_changed(&self, config: &mut AppConfig) -> Result<(), AppError> {
        let Some(ref last_known) = config.last_known_base_dir else {
            return Ok(());
        };
        if Path::new(last_known.as_str()) == self.base_dir.as_path() {
            return Ok(());
        }
        let old_dir = Path::new(last_known);
        if !old_dir.exists() {
            return Ok(());
        }
        if self.base_dir_has_user_data()? {
            return Ok(());
        }
        eprintln!(
            "migrating data from {last_known} to {}",
            self.base_dir.display()
        );
        move_or_copy_dir(old_dir, &self.base_dir)
    }

    fn base_dir_has_user_data(&self) -> Result<bool, AppError> {
        if !self.base_dir.exists() {
            return Ok(false);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                return Ok(true);
            };
            if name == "config.json" || name == MACOS_SHORTCUT_MIGRATION_MARKER {
                continue;
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn ensure_base_dir(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn migrate_macos_shortcut_default(&self, config: &mut AppConfig) -> Result<bool, AppError> {
        let migration_path = self.macos_shortcut_migration_path();
        if migration_path.exists() {
            return Ok(false);
        }

        let should_migrate = LEGACY_MACOS_GLOBAL_SHORTCUTS
            .iter()
            .any(|shortcut| shortcuts_equal(shortcut, &config.global_shortcut));
        if should_migrate {
            config.global_shortcut = DEFAULT_MACOS_GLOBAL_SHORTCUT.into();
        }

        self.mark_macos_shortcut_migration_handled()?;
        Ok(should_migrate)
    }

    #[cfg(not(target_os = "macos"))]
    fn migrate_macos_shortcut_default(&self, _config: &mut AppConfig) -> Result<bool, AppError> {
        Ok(false)
    }

    #[cfg(target_os = "macos")]
    fn mark_macos_shortcut_migration_handled(&self) -> Result<(), AppError> {
        fs::write(self.macos_shortcut_migration_path(), "done")?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    fn mark_macos_shortcut_migration_handled(&self) -> Result<(), AppError> {
        Ok(())
    }

    fn ensure_storage(&self) -> Result<(), AppError> {
        self.ensure_base_dir()?;
        let config = self.load_config()?;
        fs::create_dir_all(&config.notes_dir)?;
        if !self.metadata_path().exists() {
            self.save_metadata(&MetadataFile::default())?;
        }
        Ok(())
    }

    fn notes_dir(&self) -> Result<PathBuf, AppError> {
        Ok(PathBuf::from(self.load_config()?.notes_dir))
    }

    fn category_path(&self, category: &str) -> PathBuf {
        let notes_dir = self
            .notes_dir()
            .unwrap_or_else(|_| self.base_dir.join("notes"));
        if category.is_empty() {
            notes_dir
        } else {
            category
                .split('/')
                .fold(notes_dir, |path, segment| path.join(segment))
        }
    }

    fn note_path_in_category(&self, file_name: &str, category: &str) -> PathBuf {
        self.category_path(category).join(file_name)
    }

    fn find_metadata(&self, id: &str) -> Result<NoteMetadata, AppError> {
        self.load_metadata()?
            .notes
            .into_iter()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::note_not_found(id))
    }

    fn file_name_for(&self, id: &str, title: &str) -> String {
        let safe_title = safe_file_stem(title);
        if safe_title.is_empty() {
            format!("{id}.md")
        } else {
            format!("{id}_{safe_title}.md")
        }
    }

    fn load_metadata(&self) -> Result<MetadataFile, AppError> {
        self.ensure_base_dir()?;
        let path = self.metadata_path();
        if !path.exists() {
            let rebuilt = self.rebuild_metadata()?;
            self.save_metadata(&rebuilt)?;
            return Ok(rebuilt);
        }

        match serde_json::from_str(&fs::read_to_string(&path)?) {
            Ok(metadata) => Ok(metadata),
            Err(error) => {
                let corrupt_name = format!(
                    "metadata.corrupt-{}.json",
                    Utc::now().format("%Y%m%d%H%M%S")
                );
                fs::rename(&path, self.base_dir.join(corrupt_name))?;
                let rebuilt = self.rebuild_metadata()?;
                self.save_metadata(&rebuilt)?;
                let _ = error;
                Ok(rebuilt)
            }
        }
    }

    fn save_metadata(&self, metadata: &MetadataFile) -> Result<(), AppError> {
        self.ensure_base_dir()?;
        write_json_atomic(&self.metadata_path(), metadata)
    }

    fn rebuild_metadata(&self) -> Result<MetadataFile, AppError> {
        let notes_dir = self.notes_dir()?;
        fs::create_dir_all(&notes_dir)?;
        let mut notes = Vec::new();
        self.scan_dir_recursive(&notes_dir, "", &mut notes)?;
        Ok(MetadataFile {
            notes,
            trashed_notes: Vec::new(),
        })
    }

    fn scan_dir_recursive(
        &self,
        dir: &Path,
        category: &str,
        notes: &mut Vec<NoteMetadata>,
    ) -> Result<(), AppError> {
        self.scan_md_files(dir, category, notes)?;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let child_category = if category.is_empty() {
                    name
                } else {
                    format!("{}/{}", category, name)
                };
                self.scan_dir_recursive(&path, &child_category, notes)?;
            }
        }
        Ok(())
    }

    fn scan_md_files(
        &self,
        dir: &Path,
        category: &str,
        notes: &mut Vec<NoteMetadata>,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let Some(id) = id_from_file_name(&file_name) else {
                continue;
            };
            let content = fs::read_to_string(&path).unwrap_or_default();
            let title = infer_title(&file_name, &content);
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());

            notes.push(NoteMetadata {
                id,
                title,
                file_name,
                category: category.to_string(),
                created_at: modified,
                updated_at: modified,
                word_count: count_words(&content),
                preview: preview(&content),
                trashed_at: None,
            });
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn shortcuts_equal(left: &str, right: &str) -> bool {
    fn normalize(value: &str) -> String {
        value
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .flat_map(|ch| ch.to_lowercase())
            .collect()
    }

    normalize(left) == normalize(right)
}

fn safe_file_stem(title: &str) -> String {
    let mut stem = String::new();
    let mut last_was_separator = false;

    for ch in title.trim().chars() {
        let should_separate = ch.is_whitespace()
            || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            || ch.is_control();

        if should_separate {
            if !stem.is_empty() && !last_was_separator {
                stem.push('_');
                last_was_separator = true;
            }
            continue;
        }

        stem.push(ch);
        last_was_separator = false;
        if stem.chars().count() >= 48 {
            break;
        }
    }

    stem.trim_matches('_').to_string()
}

fn count_words(content: &str) -> usize {
    content.chars().filter(|ch| !ch.is_whitespace()).count()
}

fn preview(content: &str) -> String {
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect()
}

fn id_from_file_name(file_name: &str) -> Option<String> {
    let stem = file_name.strip_suffix(".md")?;
    Some(
        stem.split_once('_')
            .map(|(id, _)| id.to_string())
            .unwrap_or_else(|| stem.to_string()),
    )
}

fn infer_title(file_name: &str, content: &str) -> String {
    if let Some(title) = content
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
    {
        return title.to_string();
    }

    let stem = file_name.strip_suffix(".md").unwrap_or(file_name);
    stem.split_once('_')
        .map(|(_, title)| title.replace('_', " "))
        .unwrap_or_default()
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

fn imported_markdown_title(path: &Path, content: &str) -> String {
    let first_line = content.lines().next().unwrap_or_default();
    let first_line = first_line.trim_start_matches('\u{feff}').trim_start();

    if let Some(title) = first_line
        .strip_prefix("# ")
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        return title.to_string();
    }

    path.file_stem()
        .and_then(|file_stem| file_stem.to_str())
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or("导入笔记")
        .to_string()
}

fn default_note_auto_save() -> bool {
    true
}

fn default_note_surface_auto_save() -> bool {
    true
}

fn default_tile_color() -> String {
    "#f6f3ec".into()
}

fn default_tile_color_mode() -> String {
    "system".into()
}

fn default_theme() -> String {
    "system".into()
}

fn default_font_size() -> u32 {
    14
}

fn default_surface_font_size() -> u32 {
    14
}

fn default_tab_indent_size() -> u32 {
    2
}

fn default_external_file_auto_save() -> bool {
    true
}

fn default_background_fit() -> String {
    "cover".into()
}

fn default_background_dim() -> f64 {
    0.25
}

fn default_background_blur() -> f64 {
    0.0
}

fn default_background_scale() -> f64 {
    1.0
}

fn default_background_position() -> f64 {
    50.0
}

fn default_remember_surface_size() -> bool {
    true
}

fn default_tile_ctrl_close() -> bool {
    true
}

fn default_toggle_visibility_shortcut() -> String {
    String::new()
}

fn default_open_at_cursor() -> bool {
    true
}

fn default_sync_strategy() -> String {
    "localWins".into()
}

fn default_locale() -> String {
    "zh-CN".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn test_root(name: &str) -> PathBuf {
        let base = std::env::var_os("FLORAL_NOTEPAPER_TEST_TEMP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("floral-notepaper-rust-tests"));
        let root = base.join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale test root");
        }
        fs::create_dir_all(&root).expect("create test root");
        root
    }

    #[test]
    fn creates_updates_reads_and_deletes_markdown_notes() {
        let store = NoteStore::new(test_root("crud"));

        let created = store
            .create_note(SaveNoteRequest {
                title: "A/B:Test".into(),
                content: "hello\nworld".into(),
                category: String::new(),
            })
            .expect("create note");

        assert_eq!(created.title, "A/B:Test");
        assert_eq!(created.content, "hello\nworld");
        assert_eq!(created.word_count, 10);
        assert!(created.file_name.ends_with(".md"));
        assert!(created.file_name.contains("A_B_Test"));

        let loaded = store.read_note(&created.id).expect("read note");
        assert_eq!(loaded, created);

        let listed = store.list_notes().expect("list notes");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].preview, "hello world");

        let updated = store
            .update_note(
                &created.id,
                SaveNoteRequest {
                    title: "".into(),
                    content: "# 新标题\nsecond line".into(),
                    category: String::new(),
                },
            )
            .expect("update note");

        assert_eq!(updated.title, "");
        assert_eq!(updated.content, "# 新标题\nsecond line");
        assert_ne!(updated.file_name, created.file_name);

        store.delete_note(&created.id).expect("delete note");
        assert!(store.read_note(&created.id).is_err());
        assert!(store.list_notes().expect("list after delete").is_empty());
    }

    #[test]
    fn rebuilds_metadata_when_metadata_json_is_corrupt() {
        let store = NoteStore::new(test_root("repair"));
        let first = store
            .create_note(SaveNoteRequest {
                title: "第一条".into(),
                content: "# 第一条\n正文".into(),
                category: String::new(),
            })
            .expect("create first");
        let second = store
            .create_note(SaveNoteRequest {
                title: "第二条".into(),
                content: "第二条正文".into(),
                category: String::new(),
            })
            .expect("create second");

        fs::write(store.metadata_path(), "{ broken json").expect("corrupt metadata");

        let repaired = store.list_notes().expect("repair metadata");
        let ids: Vec<_> = repaired.iter().map(|note| note.id.as_str()).collect();

        assert_eq!(repaired.len(), 2);
        assert!(ids.contains(&first.id.as_str()));
        assert!(ids.contains(&second.id.as_str()));
        assert!(store
            .base_dir()
            .read_dir()
            .expect("read base dir")
            .any(|entry| entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with("metadata.corrupt-")));
    }

    #[test]
    fn reads_and_writes_config_json() {
        let store = NoteStore::new(test_root("config"));

        let default_config = store.load_config().expect("load default config");
        #[cfg(target_os = "macos")]
        assert_eq!(default_config.global_shortcut, "Command+Option+N");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(default_config.global_shortcut, "Ctrl+Space");
        assert!(default_config.note_auto_save);
        assert!(default_config.note_surface_auto_save);
        assert_eq!(default_config.tile_color, "#f6f3ec");
        assert_eq!(default_config.tile_color_mode, "system");
        assert_eq!(default_config.theme, "system");
        assert_eq!(default_config.locale, "zh-CN");
        assert!(default_config.notes_dir.ends_with("notes"));

        let custom_notes_dir = store.base_dir().join("custom-notes");
        let mut saved = AppConfig {
            locale: "en-US".into(),
            notes_dir: custom_notes_dir.join("notes").to_string_lossy().to_string(),
            global_shortcut: "Alt+Space".into(),
            close_to_tray: false,
            autostart: true,
            default_view_mode: "preview".into(),
            note_auto_save: false,
            note_surface_auto_save: false,
            tile_color: "#efe8dc".into(),
            tile_color_mode: "custom".into(),
            theme: "dark".into(),
            font_size: 16,
            surface_font_size: 16,
            tab_indent_size: 2,
            external_file_auto_save: true,
            background_image_path: String::new(),
            background_fit: "cover".into(),
            background_dim: 0.25,
            background_blur: 0.0,
            background_scale: 1.0,
            background_position_x: 50.0,
            background_position_y: 50.0,
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            render_html_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: String::new(),
            last_known_base_dir: None,
            open_at_cursor: true,

            oss_provider: "".to_string(),
            oss_endpoint: "".to_string(),
            oss_bucket: "".to_string(),
            oss_access_key_id: "".to_string(),
            oss_remote_prefix: "".to_string(),
            sync_on_startup: false,
            sync_interval: "".to_string(),
            sync_strategy: "".to_string(),
        };

        store.save_config(saved.clone()).expect("save config");

        let loaded = store.load_config().expect("reload config");
        saved.last_known_base_dir = Some(store.base_dir().to_string_lossy().to_string());
        assert_eq!(loaded, saved);
        assert!(custom_notes_dir.exists());
    }

    #[test]
    fn data_migration_candidates_include_legacy_chinese_dirs() {
        let candidates = known_data_migration_candidates_for(
            Some("/Users/alice".into()),
            Some(r"C:\Users\Alice".into()),
        );

        assert!(candidates.contains(&PathBuf::from("/Users/alice").join("Documents").join("花笺")));
        assert!(candidates.contains(
            &PathBuf::from("/Users/alice")
                .join("Library")
                .join("Application Support")
                .join("花笺")
        ));
        assert!(candidates.contains(
            &PathBuf::from(r"C:\Users\Alice")
                .join("Documents")
                .join("花笺")
        ));
    }

    #[test]
    fn migrates_last_known_base_dir_when_current_dir_only_has_config() {
        let root = test_root("last-known-base-dir-migration");
        let old_store = NoteStore::new(root.join("old"));
        fs::create_dir_all(old_store.base_dir()).expect("create old base");
        fs::write(old_store.metadata_path(), r#"{"notes":[]}"#).expect("write old metadata");
        fs::write(old_store.base_dir().join("legacy.txt"), "legacy").expect("write legacy file");

        let new_store = NoteStore::new(root.join("new"));
        fs::create_dir_all(new_store.base_dir()).expect("create new base");
        let mut config = new_store.default_config();
        config.last_known_base_dir = Some(old_store.base_dir().to_string_lossy().to_string());
        write_json_atomic(&new_store.config_path(), &config).expect("write copied config");

        let loaded = new_store.load_config().expect("load copied config");

        assert_eq!(
            loaded.last_known_base_dir.as_deref(),
            Some(new_store.base_dir().to_string_lossy().as_ref())
        );
        assert_eq!(
            fs::read_to_string(new_store.base_dir().join("legacy.txt"))
                .expect("read migrated legacy file"),
            "legacy"
        );
        assert!(new_store.metadata_path().exists());
        assert!(!old_store.base_dir().exists());
    }

    #[test]
    fn does_not_merge_last_known_base_dir_into_non_empty_current_dir() {
        let root = test_root("last-known-base-dir-non-empty");
        let old_store = NoteStore::new(root.join("old"));
        fs::create_dir_all(old_store.base_dir()).expect("create old base");
        fs::write(old_store.base_dir().join("legacy.txt"), "legacy").expect("write legacy file");

        let new_store = NoteStore::new(root.join("new"));
        fs::create_dir_all(new_store.base_dir()).expect("create new base");
        fs::write(new_store.metadata_path(), r#"{"notes":[]}"#).expect("write current metadata");
        let mut config = new_store.default_config();
        config.last_known_base_dir = Some(old_store.base_dir().to_string_lossy().to_string());
        write_json_atomic(&new_store.config_path(), &config).expect("write copied config");

        new_store.load_config().expect("load copied config");

        assert!(old_store.base_dir().exists());
        assert!(!new_store.base_dir().join("legacy.txt").exists());
        assert!(new_store.metadata_path().exists());
    }

    #[test]
    fn loads_legacy_config_with_note_surface_auto_save_enabled() {
        let store = NoteStore::new(test_root("legacy-config"));
        let notes_dir = store.base_dir().join("notes");
        fs::create_dir_all(&notes_dir).expect("create notes dir");
        fs::write(
            store.config_path(),
            format!(
                r#"{{
  "notesDir": "{}",
  "globalShortcut": "Ctrl+Space",
  "closeToTray": true,
  "autostart": false,
  "defaultViewMode": "split"
}}"#,
                notes_dir.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write legacy config");

        let loaded = store.load_config().expect("load legacy config");

        assert!(loaded.note_auto_save);
        assert!(loaded.note_surface_auto_save);
        assert_eq!(loaded.tile_color, "#f6f3ec");
        assert_eq!(loaded.tile_color_mode, "system");
        assert_eq!(loaded.theme, "system");
        assert_eq!(loaded.locale, "zh-CN");
        assert_eq!(loaded.font_size, 14);
        assert_eq!(loaded.surface_font_size, 14);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn migrates_legacy_macos_shortcut_default_once() {
        let store = NoteStore::new(test_root("legacy-macos-shortcut"));
        let notes_dir = store.base_dir().join("notes");
        fs::create_dir_all(store.base_dir()).expect("create base dir");
        fs::create_dir_all(&notes_dir).expect("create notes dir");
        fs::write(
            store.config_path(),
            format!(
                r#"{{
  "notesDir": "{}",
  "globalShortcut": "Option+Space",
  "closeToTray": true,
  "autostart": false,
  "defaultViewMode": "split"
}}"#,
                notes_dir.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write legacy config");

        let migrated = store.load_config().expect("load legacy config");

        assert_eq!(migrated.global_shortcut, "Command+Option+N");
        assert!(store.macos_shortcut_migration_path().exists());

        let mut manual = migrated;
        manual.global_shortcut = "Option+Space".into();
        store
            .save_config(manual.clone())
            .expect("save manual config");

        let loaded = store.load_config().expect("reload manual config");
        assert_eq!(loaded.global_shortcut, "Option+Space");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn migrates_previous_macos_shortcut_default() {
        let store = NoteStore::new(test_root("previous-macos-shortcut"));
        let notes_dir = store.base_dir().join("notes");
        fs::create_dir_all(store.base_dir()).expect("create base dir");
        fs::create_dir_all(&notes_dir).expect("create notes dir");
        fs::write(
            store.config_path(),
            format!(
                r#"{{
  "notesDir": "{}",
  "globalShortcut": "Ctrl+Option+Space",
  "closeToTray": true,
  "autostart": false,
  "defaultViewMode": "split"
}}"#,
                notes_dir.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write previous config");

        let migrated = store.load_config().expect("load previous config");

        assert_eq!(migrated.global_shortcut, "Command+Option+N");
        assert!(store.macos_shortcut_migration_path().exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn leaves_custom_macos_shortcut_unchanged() {
        let store = NoteStore::new(test_root("custom-macos-shortcut"));
        let notes_dir = store.base_dir().join("notes");
        fs::create_dir_all(store.base_dir()).expect("create base dir");
        fs::create_dir_all(&notes_dir).expect("create notes dir");
        fs::write(
            store.config_path(),
            format!(
                r#"{{
  "notesDir": "{}",
  "globalShortcut": "Command+K",
  "closeToTray": true,
  "autostart": false,
  "defaultViewMode": "split"
}}"#,
                notes_dir.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write custom config");

        let loaded = store.load_config().expect("load custom config");

        assert_eq!(loaded.global_shortcut, "Command+K");
        assert!(store.macos_shortcut_migration_path().exists());
    }

    #[test]
    fn imports_markdown_heading_title_without_stripping_content() {
        let root = test_root("import-heading-title");
        let source_path = root.join("外部文件.md");
        let source_content = "# 导入标题\n正文第一行\n正文第二行";
        fs::write(&source_path, source_content).expect("write source markdown");
        let store = NoteStore::new(root.join("store"));

        let imported = store
            .import_markdown_file(&source_path, "")
            .expect("import markdown");

        assert_eq!(imported.title, "导入标题");
        assert_eq!(imported.content, source_content);
        assert_eq!(
            store
                .read_note(&imported.id)
                .expect("read imported")
                .content,
            source_content
        );
    }

    #[test]
    fn imports_markdown_title_from_file_name_without_heading() {
        let root = test_root("import-file-title");
        let source_path = root.join("会议记录.md");
        let source_content = "正文第一行\n# 不是第一行标题";
        fs::write(&source_path, source_content).expect("write source markdown");
        let store = NoteStore::new(root.join("store"));

        let imported = store
            .import_markdown_file(&source_path, "")
            .expect("import markdown");

        assert_eq!(imported.title, "会议记录");
        assert_eq!(imported.content, source_content);
    }

    #[test]
    fn exports_markdown_file_without_rewriting_content() {
        let root = test_root("export-markdown");
        let store = NoteStore::new(root.join("store"));
        let content = "# 原始标题\n正文\n- 列表";
        let note = store
            .create_note(SaveNoteRequest {
                title: "导出标题".into(),
                content: content.into(),
                category: String::new(),
            })
            .expect("create note");
        let export_path = root.join("exports").join("导出.md");

        store
            .export_markdown_file(&note.id, &export_path)
            .expect("export markdown");

        assert_eq!(
            fs::read_to_string(export_path).expect("read exported markdown"),
            content
        );
    }

    #[test]
    fn creates_and_lists_nested_categories() {
        let store = NoteStore::new(test_root("nested-categories"));
        store
            .create_category("产品文档/产品文档1")
            .expect("create nested category");

        let categories = store.list_categories().expect("list categories");
        assert!(categories.contains(&"产品文档".to_string()));
        assert!(categories.contains(&"产品文档/产品文档1".to_string()));
    }

    #[test]
    fn moves_note_to_nested_category() {
        let store = NoteStore::new(test_root("move-nested"));
        let note = store
            .create_note(SaveNoteRequest {
                title: "a".into(),
                content: "content".into(),
                category: String::new(),
            })
            .expect("create note");

        store
            .move_note_to_category(&note.id, "产品文档/产品文档1")
            .expect("move note");

        let listed = store.list_notes().expect("list notes");
        assert_eq!(listed[0].category, "产品文档/产品文档1");
        assert!(store
            .note_path_in_category(&listed[0].file_name, "产品文档/产品文档1")
            .exists());
    }

    #[test]
    fn renames_nested_category() {
        let store = NoteStore::new(test_root("rename-nested"));
        let note = store
            .create_note(SaveNoteRequest {
                title: "a".into(),
                content: "content".into(),
                category: "产品文档/产品文档1".into(),
            })
            .expect("create note");

        store
            .rename_category("产品文档/产品文档1", "产品文档/产品文档2")
            .expect("rename category");

        let listed = store.list_notes().expect("list notes");
        assert_eq!(listed[0].category, "产品文档/产品文档2");
        assert!(store.read_note(&note.id).is_ok());
    }

    #[test]
    fn deletes_nested_category_and_moves_notes_to_root() {
        let store = NoteStore::new(test_root("delete-nested"));
        let note = store
            .create_note(SaveNoteRequest {
                title: "a".into(),
                content: "content".into(),
                category: "产品文档/产品文档1".into(),
            })
            .expect("create note");

        store.delete_category("产品文档").expect("delete category");

        let listed = store.list_notes().expect("list notes");
        assert_eq!(listed[0].category, "");
        assert!(store.read_note(&note.id).is_ok());
    }

    #[test]
    fn rebuilds_metadata_for_nested_categories() {
        let store = NoteStore::new(test_root("rebuild-nested"));
        let note = store
            .create_note(SaveNoteRequest {
                title: "a".into(),
                content: "content".into(),
                category: "产品文档/产品文档1".into(),
            })
            .expect("create note");

        fs::write(store.metadata_path(), "{ broken json").expect("corrupt metadata");

        let rebuilt = store.list_notes().expect("rebuild and list");
        assert_eq!(rebuilt.len(), 1);
        assert_eq!(rebuilt[0].category, "产品文档/产品文档1");
        assert_eq!(rebuilt[0].id, note.id);
    }

    #[test]
    fn rejects_invalid_category_paths() {
        let store = NoteStore::new(test_root("invalid-categories"));
        assert!(store.create_category("").is_err());
        assert!(store.create_category("a/../b").is_err());
        assert!(store.create_category("a/ /b").is_err());
        assert!(store.create_category("a/b:c").is_err());
        assert!(store.create_category("/a/b").is_err());
    }
}
