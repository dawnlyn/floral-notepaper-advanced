use crate::json_io::{read_json, write_json_atomic};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::types::{
    DeletedNoteRecord, NoteCloudStatus, NoteSyncRecord, PendingConflict, SyncState, SyncStatusDto,
};

pub struct SyncStateManager {
    state_path: PathBuf,
    state: Mutex<SyncState>,
}

impl SyncStateManager {
    pub fn new(base_dir: &Path) -> Self {
        let state_path = base_dir.join("sync-state.json");
        let state = read_json(&state_path).unwrap_or_default();
        Self {
            state_path,
            state: Mutex::new(state),
        }
    }

    pub fn get_state(&self) -> SyncState {
        self.state.lock().unwrap().clone()
    }

    pub fn save_state(&self) -> Result<(), std::io::Error> {
        let state = self.state.lock().unwrap();
        write_json_atomic(&self.state_path, &*state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    pub fn update_last_synced(&self) {
        let mut state = self.state.lock().unwrap();
        state.last_synced_at = Some(chrono::Utc::now());
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn update_active_strategy(&self, strategy: &str) {
        let mut state = self.state.lock().unwrap();
        state.active_strategy = strategy.to_string();
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn update_note_sync_record(&self, note_id: &str, record: NoteSyncRecord) {
        let mut state = self.state.lock().unwrap();
        state.note_sync_records.insert(note_id.to_string(), record);
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn remove_note_sync_record(&self, note_id: &str) {
        let mut state = self.state.lock().unwrap();
        state.note_sync_records.remove(note_id);
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn get_note_cloud_status(&self, note_id: &str) -> NoteCloudStatus {
        let state = self.state.lock().unwrap();
        match state.note_sync_records.get(note_id) {
            Some(record) => NoteCloudStatus {
                note_id: note_id.to_string(),
                synced: record.synced,
                synced_at: record.synced_at.map(|dt| dt.to_rfc3339()),
                content_hash: record.content_hash.clone(),
            },
            None => NoteCloudStatus {
                note_id: note_id.to_string(),
                synced: false,
                synced_at: None,
                content_hash: None,
            },
        }
    }

    pub fn get_all_note_cloud_status(&self) -> Vec<NoteCloudStatus> {
        let state = self.state.lock().unwrap();
        state
            .note_sync_records
            .iter()
            .map(|(id, record)| NoteCloudStatus {
                note_id: id.clone(),
                synced: record.synced,
                synced_at: record.synced_at.map(|dt| dt.to_rfc3339()),
                content_hash: record.content_hash.clone(),
            })
            .collect()
    }

    pub fn mark_permanently_deleted(&self, note_id: &str) {
        let mut state = self.state.lock().unwrap();
        // Remove from sync records
        state.note_sync_records.remove(note_id);
        // Add to permanently deleted if not already there
        if !state
            .permanently_deleted
            .iter()
            .any(|r| r.note_id == note_id)
        {
            state.permanently_deleted.push(DeletedNoteRecord {
                note_id: note_id.to_string(),
                deleted_at: chrono::Utc::now(),
            });
        }
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn get_permanently_deleted_ids(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        state
            .permanently_deleted
            .iter()
            .map(|r| r.note_id.clone())
            .collect()
    }

    pub fn remove_permanently_deleted(&self, note_id: &str) {
        let mut state = self.state.lock().unwrap();
        state.permanently_deleted.retain(|r| r.note_id != note_id);
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn get_status_dto(&self) -> SyncStatusDto {
        let state = self.state.lock().unwrap();
        SyncStatusDto {
            state: "idle".into(),
            last_synced_at: state.last_synced_at.map(|dt| dt.to_rfc3339()),
            last_result: None,
            next_sync_at: None,
        }
    }

    pub fn set_pending_conflicts(&self, conflicts: Vec<PendingConflict>) {
        let mut state = self.state.lock().unwrap();
        state.pending_conflicts = conflicts;
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn add_or_update_pending_conflicts(&self, new_conflicts: Vec<PendingConflict>) {
        let mut state = self.state.lock().unwrap();
        for conflict in new_conflicts {
            if let Some(existing) = state
                .pending_conflicts
                .iter_mut()
                .find(|c| c.note_id == conflict.note_id)
            {
                if !existing.resolved {
                    *existing = conflict;
                }
            } else {
                state.pending_conflicts.push(conflict);
            }
        }
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn get_pending_conflicts(&self) -> Vec<PendingConflict> {
        let state = self.state.lock().unwrap();
        state
            .pending_conflicts
            .iter()
            .filter(|c| !c.resolved)
            .cloned()
            .collect()
    }

    pub fn get_pending_conflict(&self, note_id: &str) -> Option<PendingConflict> {
        let state = self.state.lock().unwrap();
        state
            .pending_conflicts
            .iter()
            .find(|c| c.note_id == note_id && !c.resolved)
            .cloned()
    }

    pub fn mark_conflict_resolved(&self, note_id: &str) {
        let mut state = self.state.lock().unwrap();
        if let Some(conflict) = state
            .pending_conflicts
            .iter_mut()
            .find(|c| c.note_id == note_id)
        {
            conflict.resolved = true;
        }
        let _ = write_json_atomic(&self.state_path, &*state);
    }

    pub fn clear_resolved_conflicts(&self) {
        let mut state = self.state.lock().unwrap();
        state.pending_conflicts.retain(|c| !c.resolved);
        let _ = write_json_atomic(&self.state_path, &*state);
    }
}
