use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::engine::SyncEngine;
use super::oss::{OssClient, OssConfig};
use super::state::SyncStateManager;
use crate::services::notes::default_store;
use crate::sync::types::{SyncError, SyncResultDto};

const INITIAL_DELAY: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_secs(30);

static SYNC_RUNNING: AtomicBool = AtomicBool::new(false);
static SYNC_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn start_sync_scheduler(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(INITIAL_DELAY);

        // Check if startup sync is needed
        if let Err(e) = maybe_run_startup_sync(&app) {
            eprintln!("startup sync check failed: {e}");
        }

        loop {
            if let Err(e) = poll_scheduled_sync(&app) {
                eprintln!("scheduled sync error: {e}");
            }
            thread::sleep(POLL_INTERVAL);
        }
    });
}

pub fn request_sync() {
    SYNC_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn is_sync_running() -> bool {
    SYNC_RUNNING.load(Ordering::SeqCst)
}

fn maybe_run_startup_sync(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config = default_store()?.load_config()?;

    if !config.sync_on_startup {
        return Ok(());
    }

    if !is_oss_configured(&config) {
        return Ok(());
    }

    let _ = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run_sync_task(app, &config))?;
    Ok(())
}

fn poll_scheduled_sync(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config = default_store()?.load_config()?;

    // Check if manual sync was requested
    let manual_request = SYNC_REQUESTED.swap(false, Ordering::SeqCst);

    if !manual_request && !is_oss_configured(&config) {
        return Ok(());
    }

    if !manual_request && config.sync_interval == "off" {
        return Ok(());
    }

    if is_sync_running() {
        return Ok(());
    }

    // Check if interval has elapsed
    if !manual_request {
        let state_manager = SyncStateManager::new(&default_store()?.base_dir);
        let state = state_manager.get_state();

        if let Some(last_synced) = state.last_synced_at {
            let now = chrono::Utc::now();
            let elapsed = now - last_synced;
            let interval_seconds = get_interval_seconds(&config.sync_interval);

            if let Some(interval) = interval_seconds {
                if elapsed.num_seconds() < interval as i64 {
                    return Ok(());
                }
            }
        }
    }

    let _ = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run_sync_task(app, &config))?;
    Ok(())
}

fn is_oss_configured(config: &crate::services::notes::AppConfig) -> bool {
    !config.oss_provider.is_empty()
        && !config.oss_endpoint.is_empty()
        && !config.oss_bucket.is_empty()
        && !config.oss_access_key_id.is_empty()
}

fn get_interval_seconds(interval: &str) -> Option<u64> {
    match interval {
        "off" => None,
        "1min" => Some(60),
        "3min" => Some(180),
        "5min" => Some(300),
        "10min" => Some(600),
        "30min" => Some(1800),
        "1hour" => Some(3600),
        "daily" => {
            // Check if we've crossed midnight
            let now = chrono::Local::now();
            let today_start = now.date_naive().and_hms_opt(0, 0, 0)?;
            let today_start_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                today_start,
                chrono::Utc,
            );
            let now_utc = chrono::Utc::now();
            if now_utc > today_start_utc {
                Some(0) // Trigger sync if we haven't synced today
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Guard that clears the `SYNC_RUNNING` flag when dropped, so a panic inside
/// the sync task does not leave the flag set forever.
struct SyncRunningGuard;

impl SyncRunningGuard {
    fn acquire() -> Option<Self> {
        match SYNC_RUNNING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => Some(SyncRunningGuard),
            Err(_) => None,
        }
    }
}

impl Drop for SyncRunningGuard {
    fn drop(&mut self) {
        SYNC_RUNNING.store(false, Ordering::SeqCst);
    }
}

pub async fn run_sync_task(
    app: &AppHandle,
    config: &crate::services::notes::AppConfig,
) -> Result<SyncResultDto, Box<dyn std::error::Error>> {
    let _guard = SyncRunningGuard::acquire().ok_or("sync already running")?;

    eprintln!("sync task started");

    // Get AccessKeySecret from keyring
    let access_key_secret = get_oss_credential(&config.oss_access_key_id).unwrap_or_default();

    let oss_config = OssConfig {
        endpoint: config.oss_endpoint.clone(),
        bucket: config.oss_bucket.clone(),
        access_key_id: config.oss_access_key_id.clone(),
        access_key_secret,
    };

    let client = OssClient::new(oss_config).map_err(SyncError::from)?;

    let store = default_store().map_err(SyncError::from)?;

    let state_manager = SyncStateManager::new(&store.base_dir);

    let device_id = get_device_id();
    let strategy = config.sync_strategy.clone();
    let remote_prefix = config.oss_remote_prefix.clone();

    let engine = SyncEngine::new(
        &client,
        &store,
        &state_manager,
        strategy,
        device_id,
        remote_prefix,
    );

    let result = engine.sync().await;

    eprintln!("sync task finished: {result:?}");

    match result {
        Ok(sync_result) => {
            let _ = app.emit("sync-completed", &sync_result);
            Ok(sync_result)
        }
        Err(e) => {
            let _ = app.emit("sync-error", e.to_string());
            Err(Box::from(e))
        }
    }
}

fn get_device_id() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-device".to_string())
}

fn get_oss_credential(access_key_id: &str) -> Option<String> {
    if access_key_id.is_empty() {
        return None;
    }
    let entry = keyring::Entry::new("floral-notepaper-oss", access_key_id).ok()?;
    entry.get_password().ok()
}

pub fn save_oss_credential(access_key_id: &str, secret: &str) -> Result<(), String> {
    let entry =
        keyring::Entry::new("floral-notepaper-oss", access_key_id).map_err(|e| e.to_string())?;
    entry.set_password(secret).map_err(|e| e.to_string())
}

pub fn get_oss_credential_public(access_key_id: &str) -> Result<String, String> {
    if access_key_id.is_empty() {
        return Err("Access Key ID is empty".to_string());
    }
    let entry =
        keyring::Entry::new("floral-notepaper-oss", access_key_id).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}
