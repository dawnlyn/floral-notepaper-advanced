pub mod engine;
pub mod oss;
pub mod scheduler;
pub mod state;
pub mod types;

use tauri::{AppHandle, Emitter};

use crate::services::notes::{default_store, AppError};
use oss::{OssClient, OssConfig};
use state::SyncStateManager;
use types::{NoteCloudStatus, SyncResultDto, SyncStatusDto};

pub fn start_sync_scheduler(app: AppHandle) {
    scheduler::start_sync_scheduler(app);
}

pub fn request_sync() {
    scheduler::request_sync();
}

pub async fn sync_now(app: AppHandle) -> Result<SyncResultDto, AppError> {
    let config = default_store()?.load_config()?;

    if config.oss_provider.is_empty()
        || config.oss_endpoint.is_empty()
        || config.oss_bucket.is_empty()
        || config.oss_access_key_id.is_empty()
    {
        return Err(AppError {
            code: "syncConfigIncomplete".into(),
            message: "OSS configuration is incomplete".into(),
            details: Default::default(),
        });
    }

    // Get AccessKeySecret from keyring
    let access_key_secret =
        scheduler::get_oss_credential_public(&config.oss_access_key_id).unwrap_or_default();

    let oss_config = OssConfig {
        endpoint: config.oss_endpoint.clone(),
        bucket: config.oss_bucket.clone(),
        access_key_id: config.oss_access_key_id.clone(),
        access_key_secret,
    };

    let client = OssClient::new(oss_config).map_err(|e| AppError {
        code: "ossClient".into(),
        message: e.to_string(),
        details: Default::default(),
    })?;

    let store = default_store()?;
    let state_manager = SyncStateManager::new(&store.base_dir);

    let device_id = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-device".to_string());

    let sync_engine = engine::SyncEngine::new(
        &client,
        &store,
        &state_manager,
        config.sync_strategy.clone(),
        device_id,
        config.oss_remote_prefix.clone(),
    );

    let result = sync_engine.sync().await.map_err(|e| AppError {
        code: "sync".into(),
        message: e.to_string(),
        details: Default::default(),
    })?;

    let _ = app.emit("sync-completed", &result);
    Ok(result)
}

pub fn get_sync_status() -> Result<SyncStatusDto, AppError> {
    let store = default_store()?;
    let state_manager = SyncStateManager::new(&store.base_dir);
    Ok(state_manager.get_status_dto())
}

pub async fn test_oss_connection(
    endpoint: String,
    bucket: String,
    access_key_id: String,
    access_key_secret: String,
) -> Result<(), AppError> {
    let oss_config = OssConfig {
        endpoint,
        bucket,
        access_key_id,
        access_key_secret,
    };

    let client = OssClient::new(oss_config).map_err(|e| AppError {
        code: "ossClient".into(),
        message: e.to_string(),
        details: Default::default(),
    })?;

    client.test_connection().await.map_err(|e| AppError {
        code: "ossConnection".into(),
        message: e.to_string(),
        details: Default::default(),
    })
}

pub fn save_oss_credential(access_key_id: String, secret: String) -> Result<(), AppError> {
    scheduler::save_oss_credential(&access_key_id, &secret).map_err(|e| AppError {
        code: "keyring".into(),
        message: e,
        details: Default::default(),
    })
}

pub fn get_oss_credential(access_key_id: String) -> Result<String, AppError> {
    scheduler::get_oss_credential_public(&access_key_id).map_err(|e| AppError {
        code: "keyring".into(),
        message: e,
        details: Default::default(),
    })
}

pub fn get_notes_cloud_status() -> Result<Vec<NoteCloudStatus>, AppError> {
    let store = default_store()?;
    let state_manager = SyncStateManager::new(&store.base_dir);
    Ok(state_manager.get_all_note_cloud_status())
}

#[tauri::command]
pub async fn sync_now_command(app: AppHandle) -> Result<SyncResultDto, AppError> {
    sync_now(app).await
}

#[tauri::command]
pub fn sync_status_command() -> Result<SyncStatusDto, AppError> {
    get_sync_status()
}

#[tauri::command]
pub async fn oss_test_connection_command(
    endpoint: String,
    bucket: String,
    access_key_id: String,
    access_key_secret: String,
) -> Result<(), AppError> {
    test_oss_connection(endpoint, bucket, access_key_id, access_key_secret).await
}

#[tauri::command]
pub fn oss_save_credential_command(access_key_id: String, secret: String) -> Result<(), AppError> {
    save_oss_credential(access_key_id, secret)
}

#[tauri::command]
pub fn oss_get_credential_command(access_key_id: String) -> Result<String, AppError> {
    get_oss_credential(access_key_id)
}

#[tauri::command]
pub fn sync_get_cloud_status_command() -> Result<Vec<NoteCloudStatus>, AppError> {
    get_notes_cloud_status()
}
