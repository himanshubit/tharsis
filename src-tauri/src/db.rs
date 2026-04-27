// src-tauri/src/db.rs
//
// Database initialisation module.
// Enforces WAL mode + synchronous=NORMAL + busy_timeout to prevent
// SQLITE_BUSY lock contention during concurrent chunk progress writes.

use anyhow::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    SqlitePool,
};
use std::{str::FromStr, time::Duration};

/// Returns the absolute path to `tharsis.db` inside the platform's
/// application-data directory, creating the containing directory if needed.
pub fn db_path() -> Result<String> {
    // Windows  → %APPDATA%\tharsis\tharsis.db
    // macOS    → $HOME/Library/Application Support/tharsis/tharsis.db (via HOME)
    // Linux    → $HOME/.local/share/tharsis/tharsis.db  (via HOME)
    let base = std::env::var("APPDATA")
        .or_else(|_| {
            // macOS / Linux: build XDG-ish path from $HOME
            std::env::var("HOME").map(|h| format!("{}/Library/Application Support", h))
        })
        .unwrap_or_else(|_| ".".to_string());

    let dir = std::path::PathBuf::from(base).join("tharsis");
    std::fs::create_dir_all(&dir)?;

    let db_file = dir.join("tharsis.db");
    Ok(format!("sqlite:{}", db_file.display()))
}

/// Initialises the SQLite connection pool with concurrency-safe PRAGMAs:
///
/// | PRAGMA               | Value   | Reason                                      |
/// |----------------------|---------|---------------------------------------------|
/// | journal_mode         | WAL     | Readers don't block writers; multi-thread ok |
/// | synchronous          | NORMAL  | Durability without fsync on every write      |
/// | busy_timeout (ms)    | 5 000   | Retry for 5 s before returning SQLITE_BUSY   |
pub async fn init_pool() -> Result<SqlitePool> {
    let url = db_path()?;
    tracing::info!("Opening database at {}", url);

    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(5_000));

    let pool = SqlitePool::connect_with(opts).await?;

    // ── Bootstrap schema ──────────────────────────────────────────────────
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await?;


    tracing::info!("Database ready — WAL mode enabled");
    Ok(pool)
}
