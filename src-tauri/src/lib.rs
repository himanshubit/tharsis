// src-tauri/src/lib.rs
// Library entry-point — re-exported by main.rs.
// Tauri v2 uses a lib crate so the same code works on all platforms
// (iOS/Android need a `cdylib` entry point, not a binary).

mod db;
pub mod models;
pub mod queries;
pub mod engine;

use sqlx::SqlitePool;
use tauri::Manager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// ─── Application State ────────────────────────────────────────────────────────
pub struct AppState {
    pub pool: SqlitePool,
    pub active_tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

// ─── IPC Commands ─────────────────────────────────────────────────────────────

/// Smoke-test — confirms the Tauri IPC bridge is alive.
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

/// Returns the absolute path to the SQLite database file.
#[tauri::command]
async fn get_db_path() -> Result<String, String> {
    db::db_path().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_downloads(state: tauri::State<'_, AppState>) -> Result<Vec<models::Download>, String> {
    // Returns ALL downloads (active + completed) so history persists
    queries::get_all_downloads(&state.pool).await
}

#[tauri::command]
async fn get_download_dir() -> Result<String, String> {
    // Use the dirs crate (already a transitive dep) to get the OS Downloads folder.
    // Falls back to the user's home dir, then finally to the current working directory.
    let path = dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn start_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
    save_dir: String,
) -> Result<String, String> {
    let download_id = Uuid::new_v4().to_string();
    let pool = state.pool.clone();
    let tasks = state.active_tasks.clone();
    let id_c = download_id.clone();
    
    // Spawn the download in the background so the command returns immediately
    let handle = tokio::spawn(async move {
        let result = engine::download_file(app, pool, url, save_dir, id_c.clone()).await;
        if let Err(e) = result {
            tracing::error!("Download {} failed: {}", id_c, e);
        }
        // Cleanup handle when finished
        let mut tasks_lock = tasks.lock().await;
        tasks_lock.remove(&id_c);
    });

    state.active_tasks.lock().await.insert(download_id.clone(), handle);
    Ok(download_id)
}

#[tauri::command]
async fn delete_download(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // 1. Abort background task if it's still running
    let mut tasks = state.active_tasks.lock().await;
    if let Some(handle) = tasks.remove(&id) {
        handle.abort();
        tracing::info!("Aborted active download task: {}", id);
    }

    // 2. Remove from database
    sqlx::query("DELETE FROM downloads WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Library Entry Point ──────────────────────────────────────────────────────
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Structured logging — reads RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tharsis=debug,sqlx=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Block on async DB init inside the synchronous setup hook.
            let pool = tauri::async_runtime::block_on(async {
                db::init_pool()
                    .await
                    .expect("Fatal: could not initialise SQLite database")
            });

            app.manage(AppState { 
                pool, 
                active_tasks: Arc::new(Mutex::new(HashMap::new())) 
            });
            tracing::info!("Tharsis backend ready — Phase 1 complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ping, get_db_path, get_downloads, get_download_dir, start_download, delete_download])
        .run(tauri::generate_context!())
        .expect("error while running Tharsis");
}
