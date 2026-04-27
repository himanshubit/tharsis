use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Download {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub save_path: String,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub status: String,
    pub created_at: String, // Stringified timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Chunk {
    pub id: String,
    pub download_id: String,
    pub start_byte: i64,
    pub end_byte: i64,
    pub downloaded_bytes: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub download_id: String,
    pub chunk_id: String,
    pub downloaded_bytes: i64,
    pub speed_bytes_per_sec: f64,
}
