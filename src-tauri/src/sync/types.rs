use crate::services::notes::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncManifest {
    pub version: u32,
    pub last_synced_at: DateTime<Utc>,
    pub last_synced_by: String,
    pub notes: Vec<RemoteNoteEntry>,
}

impl Default for SyncManifest {
    fn default() -> Self {
        Self {
            version: 1,
            last_synced_at: Utc::now(),
            last_synced_by: String::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteNoteEntry {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub word_count: usize,
    pub content_hash: String,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResultDto {
    pub uploaded: usize,
    pub downloaded: usize,
    pub conflicts: Vec<SyncConflictDto>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflictDto {
    pub note_id: String,
    pub note_title: String,
    pub local_updated_at: String,
    pub remote_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    pub state: String,
    pub last_synced_at: Option<String>,
    pub last_result: Option<SyncResultDto>,
    pub next_sync_at: Option<String>,
}

impl Default for SyncStatusDto {
    fn default() -> Self {
        Self {
            state: "idle".into(),
            last_synced_at: None,
            last_result: None,
            next_sync_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteCloudStatus {
    pub note_id: String,
    pub synced: bool,
    pub synced_at: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub last_synced_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub active_strategy: String,
    #[serde(default)]
    pub note_sync_records: HashMap<String, NoteSyncRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSyncRecord {
    pub synced: bool,
    pub synced_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
}

pub enum SyncAction {
    Upload {
        note_id: String,
    },
    Download {
        note_id: String,
    },
    Conflict {
        note_id: String,
        local_updated: DateTime<Utc>,
        remote_updated: DateTime<Utc>,
    },
    DeleteLocal {
        note_id: String,
    },
    Skip {
        note_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct SyncError {
    pub code: String,
    pub message: String,
}

impl SyncError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SyncError {}

impl From<reqwest::Error> for SyncError {
    fn from(error: reqwest::Error) -> Self {
        Self::new("http", error.to_string())
    }
}

impl From<std::io::Error> for SyncError {
    fn from(error: std::io::Error) -> Self {
        Self::new("io", error.to_string())
    }
}

impl From<serde_json::Error> for SyncError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("json", error.to_string())
    }
}

impl From<quick_xml::DeError> for SyncError {
    fn from(error: quick_xml::DeError) -> Self {
        Self::new("xml", error.to_string())
    }
}

impl From<AppError> for SyncError {
    fn from(err: AppError) -> Self {
        // 手动转换，比如转成字符串包装
        SyncError::new(err.code, err.message)
        // 或者 Box::new(err) 如果你确定这里可以
    }
}
