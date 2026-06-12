use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::oss::OssClient;
use super::state::SyncStateManager;
use super::types::{
    NoteSyncRecord, RemoteNoteEntry, SyncAction, SyncConflictDto, SyncError, SyncManifest,
    SyncResultDto,
};
use crate::services::notes::{NoteMetadata, NoteStore};

pub struct SyncEngine<'a> {
    client: &'a OssClient,
    store: &'a NoteStore,
    state_manager: &'a SyncStateManager,
    strategy: String,
    device_id: String,
    remote_prefix: String,
}

impl<'a> SyncEngine<'a> {
    pub fn new(
        client: &'a OssClient,
        store: &'a NoteStore,
        state_manager: &'a SyncStateManager,
        strategy: String,
        device_id: String,
        remote_prefix: String,
    ) -> Self {
        Self {
            client,
            store,
            state_manager,
            strategy,
            device_id,
            remote_prefix,
        }
    }

    fn manifest_key(&self) -> String {
        if self.remote_prefix.is_empty() {
            "manifest.json".to_string()
        } else {
            format!("{}/manifest.json", self.remote_prefix.trim_end_matches('/'))
        }
    }

    fn note_content_key(&self, note_id: &str) -> String {
        if self.remote_prefix.is_empty() {
            format!("notes/{}.md", note_id)
        } else {
            format!(
                "{}/notes/{}.md",
                self.remote_prefix.trim_end_matches('/'),
                note_id
            )
        }
    }

    fn note_meta_key(&self, note_id: &str) -> String {
        if self.remote_prefix.is_empty() {
            format!("notes/{}.meta.json", note_id)
        } else {
            format!(
                "{}/notes/{}.meta.json",
                self.remote_prefix.trim_end_matches('/'),
                note_id
            )
        }
    }

    fn compute_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn sync(&self) -> Result<SyncResultDto, SyncError> {
        let start_time = std::time::Instant::now();
        let mut uploaded = 0usize;
        let mut downloaded = 0usize;
        let mut _deleted_remote = 0usize;
        let mut conflicts = Vec::new();
        let mut errors = Vec::new();

        // 1. Fetch remote manifest
        let remote_manifest = self.fetch_remote_manifest().await?;

        // 2. Get local notes + trash/deletion info
        let local_notes = self
            .store
            .list_notes()
            .map_err(|e| SyncError::new("notes", e.to_string()))?;
        let trashed_ids: Vec<String> = self
            .store
            .list_trashed_notes()
            .map_err(|e| SyncError::new("notes", e.to_string()))?
            .iter()
            .map(|n| n.id.clone())
            .collect();
        let permanently_deleted_ids = self.state_manager.get_permanently_deleted_ids();

        // 3. Build sync plan
        let actions = self.build_sync_plan(
            &remote_manifest,
            &local_notes,
            &trashed_ids,
            &permanently_deleted_ids,
        );

        // 4. Execute actions
        for action in actions {
            match action {
                SyncAction::Upload { note_id } => {
                    if let Err(e) = self.upload_note(&note_id).await {
                        errors.push(format!("Upload {}: {}", note_id, e));
                    } else {
                        uploaded += 1;
                    }
                }
                SyncAction::Download { note_id } => {
                    if let Err(e) = self.download_note(&note_id, &remote_manifest).await {
                        errors.push(format!("Download {}: {}", note_id, e));
                    } else {
                        downloaded += 1;
                    }
                }
                SyncAction::Conflict {
                    note_id,
                    local_updated,
                    remote_updated,
                } => {
                    let title = local_notes
                        .iter()
                        .find(|n| n.id == note_id)
                        .map(|n| n.title.clone())
                        .unwrap_or_default();
                    conflicts.push(SyncConflictDto {
                        note_id,
                        note_title: title,
                        local_updated_at: local_updated.to_rfc3339(),
                        remote_updated_at: remote_updated.to_rfc3339(),
                    });
                }
                SyncAction::DeleteLocal { note_id } => {
                    // Remote deleted -> trash locally (soft delete)
                    if let Err(e) = self.store.trash_note(&note_id) {
                        errors.push(format!("DeleteLocal {}: {}", note_id, e));
                    }
                }
                SyncAction::DeleteRemote { note_id } => {
                    // Local permanent delete -> remove from OSS
                    if let Err(e) = self.delete_remote_note(&note_id).await {
                        errors.push(format!("DeleteRemote {}: {}", note_id, e));
                    } else {
                        self.state_manager.remove_permanently_deleted(&note_id);
                        _deleted_remote += 1;
                    }
                }
                SyncAction::Skip { .. } => {}
            }
        }

        // 5. Update remote manifest (merge strategy)
        let updated_local_notes = self
            .store
            .list_notes()
            .map_err(|e| SyncError::new("notes", e.to_string()))?;
        self.update_remote_manifest(&updated_local_notes, &remote_manifest)
            .await?;

        // 6. Update state
        self.state_manager.update_last_synced();
        self.state_manager.update_active_strategy(&self.strategy);

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(SyncResultDto {
            uploaded,
            downloaded,
            conflicts,
            errors,
            duration_ms,
            completed_at: Utc::now().to_rfc3339(),
        })
    }

    async fn fetch_remote_manifest(&self) -> Result<SyncManifest, SyncError> {
        let key = self.manifest_key();
        match self.client.get_object(&key).await {
            Ok(data) => {
                let manifest: SyncManifest = serde_json::from_slice(&data)?;
                Ok(manifest)
            }
            Err(e) if e.message.contains("404") || e.message.contains("Not Found") => {
                Ok(SyncManifest::default())
            }
            Err(e) => Err(e),
        }
    }

    async fn update_remote_manifest(
        &self,
        local_notes: &[NoteMetadata],
        old_manifest: &SyncManifest,
    ) -> Result<(), SyncError> {
        let local_ids: std::collections::HashSet<String> =
            local_notes.iter().map(|n| n.id.clone()).collect();

        let mut entries = Vec::new();

        // 1. Add all active local notes
        for note in local_notes {
            let content = self
                .store
                .read_note(&note.id)
                .map_err(|e| SyncError::new("notes", e.to_string()))?
                .content;
            let content_hash = Self::compute_content_hash(&content);

            entries.push(RemoteNoteEntry {
                id: note.id.clone(),
                title: note.title.clone(),
                file_name: note.file_name.clone(),
                category: note.category.clone(),
                created_at: note.created_at,
                updated_at: note.updated_at,
                word_count: note.word_count,
                content_hash,
                deleted: false,
            });
        }

        // 2. Preserve remote tombstones (deleted=true) that are not re-uploaded
        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
        for remote_entry in &old_manifest.notes {
            if remote_entry.deleted
                && !local_ids.contains(&remote_entry.id)
                && remote_entry.updated_at > thirty_days_ago
            {
                entries.push(remote_entry.clone());
            }
        }

        // 3. Add locally permanently-deleted notes as new tombstones
        for del_id in self.state_manager.get_permanently_deleted_ids() {
            if !entries.iter().any(|e| e.id == del_id) {
                entries.push(RemoteNoteEntry {
                    id: del_id,
                    title: String::new(),
                    file_name: String::new(),
                    category: String::new(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    word_count: 0,
                    content_hash: String::new(),
                    deleted: true,
                });
            }
        }

        let manifest = SyncManifest {
            version: 1,
            last_synced_at: Utc::now(),
            last_synced_by: self.device_id.clone(),
            notes: entries,
        };

        let data = serde_json::to_vec_pretty(&manifest)?;
        self.client
            .put_object(&self.manifest_key(), data, "application/json")
            .await?;

        Ok(())
    }

    fn build_sync_plan(
        &self,
        remote_manifest: &SyncManifest,
        local_notes: &[NoteMetadata],
        trashed_ids: &[String],
        permanently_deleted_ids: &[String],
    ) -> Vec<SyncAction> {
        let mut actions = Vec::new();

        // Separate remote entries into active and deleted
        let remote_active: HashMap<String, &RemoteNoteEntry> = remote_manifest
            .notes
            .iter()
            .filter(|e| !e.deleted)
            .map(|e| (e.id.clone(), e))
            .collect();

        let remote_deleted_ids: std::collections::HashSet<String> = remote_manifest
            .notes
            .iter()
            .filter(|e| e.deleted)
            .map(|e| e.id.clone())
            .collect();

        let local_map: HashMap<String, &NoteMetadata> =
            local_notes.iter().map(|n| (n.id.clone(), n)).collect();

        let trashed_set: std::collections::HashSet<&String> = trashed_ids.iter().collect();
        let perm_deleted_set: std::collections::HashSet<&String> =
            permanently_deleted_ids.iter().collect();

        // 1. Permanent delete propagation: local permanently deleted -> DeleteRemote
        // We also delete when the note is no longer in the remote manifest (e.g. manifest
        // was previously cleared) so that orphaned note files on OSS are cleaned up.
        for id in permanently_deleted_ids {
            if !remote_deleted_ids.contains(id) {
                actions.push(SyncAction::DeleteRemote {
                    note_id: id.clone(),
                });
            }
        }

        // 2. Remote delete propagation: remote deleted AND still exists locally → DeleteLocal (trash)
        for id in &remote_deleted_ids {
            if local_map.contains_key(id) {
                actions.push(SyncAction::DeleteLocal {
                    note_id: id.clone(),
                });
            }
        }

        // 3. Notes only in local (exclude remote_deleted) -> upload
        for (id, _local) in &local_map {
            if !remote_active.contains_key(id) && !remote_deleted_ids.contains(id) {
                actions.push(SyncAction::Upload {
                    note_id: id.clone(),
                });
            }
        }

        // 4. Notes only in remote (exclude trashed + permanently deleted) -> download
        for (id, _remote) in &remote_active {
            if !local_map.contains_key(id)
                && !trashed_set.contains(id)
                && !perm_deleted_set.contains(id)
            {
                actions.push(SyncAction::Download {
                    note_id: id.clone(),
                });
            }
        }

        // 5. Notes in both -> compare content + all metadata
        for (id, local) in &local_map {
            if let Some(remote) = remote_active.get(id) {
                let local_content = match self.store.read_note(id) {
                    Ok(note) => note.content,
                    Err(_) => continue,
                };
                let local_hash = Self::compute_content_hash(&local_content);

                // Conflict = same note exists on both sides but any field differs
                let content_match = local_hash == remote.content_hash;
                let category_match = local.category == remote.category;
                let title_match = local.title == remote.title;
                let file_name_match = local.file_name == remote.file_name;

                if content_match && category_match && title_match && file_name_match {
                    actions.push(SyncAction::Skip {
                        note_id: id.clone(),
                    });
                } else {
                    match self.strategy.as_str() {
                        "localWins" => {
                            actions.push(SyncAction::Upload {
                                note_id: id.clone(),
                            });
                        }
                        "remoteWins" => {
                            actions.push(SyncAction::Download {
                                note_id: id.clone(),
                            });
                        }
                        "manual" => {
                            actions.push(SyncAction::Conflict {
                                note_id: id.clone(),
                                local_updated: local.updated_at,
                                remote_updated: remote.updated_at,
                            });
                        }
                        _ => {
                            actions.push(SyncAction::Upload {
                                note_id: id.clone(),
                            });
                        }
                    }
                }
            }
        }

        actions
    }

    async fn delete_remote_note(&self, note_id: &str) -> Result<(), SyncError> {
        // Delete content file (ignore 404)
        match self
            .client
            .delete_object(&self.note_content_key(note_id))
            .await
        {
            Ok(_) => {}
            Err(e) if e.message.contains("404") || e.message.contains("Not Found") => {}
            Err(e) => return Err(e),
        }
        // Delete meta file (ignore 404)
        match self
            .client
            .delete_object(&self.note_meta_key(note_id))
            .await
        {
            Ok(_) => {}
            Err(e) if e.message.contains("404") || e.message.contains("Not Found") => {}
            Err(e) => return Err(e),
        }
        // Clean up sync state
        self.state_manager.remove_note_sync_record(note_id);
        Ok(())
    }

    async fn upload_note(&self, note_id: &str) -> Result<(), SyncError> {
        let note = self
            .store
            .read_note(note_id)
            .map_err(|e| SyncError::new("notes", e.to_string()))?;

        let content_hash = Self::compute_content_hash(&note.content);

        // Upload content
        self.client
            .put_object(
                &self.note_content_key(note_id),
                note.content.clone().into_bytes(),
                "text/markdown",
            )
            .await?;

        // Upload metadata
        let meta =
            serde_json::to_vec_pretty(&note).map_err(|e| SyncError::new("json", e.to_string()))?;
        self.client
            .put_object(&self.note_meta_key(note_id), meta, "application/json")
            .await?;

        // Update state
        self.state_manager.update_note_sync_record(
            note_id,
            NoteSyncRecord {
                synced: true,
                synced_at: Some(Utc::now()),
                content_hash: Some(content_hash),
            },
        );

        Ok(())
    }

    async fn download_note(
        &self,
        note_id: &str,
        remote_manifest: &SyncManifest,
    ) -> Result<(), SyncError> {
        // Get remote entry
        let remote_entry = remote_manifest
            .notes
            .iter()
            .find(|e| e.id == note_id)
            .ok_or_else(|| {
                SyncError::new("sync", format!("Note {} not found in remote", note_id))
            })?;

        // Download content
        let content_bytes = self
            .client
            .get_object(&self.note_content_key(note_id))
            .await?;
        let content = String::from_utf8(content_bytes)
            .map_err(|e| SyncError::new("encoding", e.to_string()))?;

        // Download metadata
        let meta_bytes = self.client.get_object(&self.note_meta_key(note_id)).await?;
        let meta: NoteMetadata = serde_json::from_slice(&meta_bytes)?;

        // Create or update local note
        let save_request = crate::services::notes::SaveNoteRequest {
            title: meta.title.clone(),
            content: content.clone(),
            category: meta.category.clone(),
        };

        // Try to update existing note, or create new one
        match self.store.update_note(note_id, save_request.clone()) {
            Ok(_) => {
                // Update successful
            }
            Err(_) => {
                // Note not in active notes — restore from trash if present, preserving original ID
                if self
                    .store
                    .prepare_trashed_for_download(note_id)
                    .unwrap_or(false)
                {
                    let _ = self.store.update_note(note_id, save_request.clone());
                } else {
                    let _note = self
                        .store
                        .create_note(save_request)
                        .map_err(|e| SyncError::new("notes", e.to_string()))?;
                }
            }
        }

        // Update state
        self.state_manager.update_note_sync_record(
            note_id,
            NoteSyncRecord {
                synced: true,
                synced_at: Some(Utc::now()),
                content_hash: Some(remote_entry.content_hash.clone()),
            },
        );

        Ok(())
    }
}
