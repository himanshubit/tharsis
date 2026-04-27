use sqlx::SqlitePool;
use crate::models::{Download, Chunk};

pub async fn insert_download(
    pool: &SqlitePool,
    id: String,
    url: String,
    filename: String,
    save_path: String,
    total_bytes: i64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO downloads (id, url, filename, save_path, total_bytes, status) \
         VALUES (?, ?, ?, ?, ?, 'pending')"
    )
    .bind(id)
    .bind(url)
    .bind(filename)
    .bind(save_path)
    .bind(total_bytes)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn insert_chunks(pool: &SqlitePool, chunks: Vec<Chunk>) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for chunk in chunks {
        sqlx::query(
            "INSERT INTO chunks (id, download_id, start_byte, end_byte, downloaded_bytes, status) \
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(chunk.id)
        .bind(chunk.download_id)
        .bind(chunk.start_byte)
        .bind(chunk.end_byte)
        .bind(chunk.downloaded_bytes)
        .bind(chunk.status)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_active_downloads(pool: &SqlitePool) -> Result<Vec<Download>, String> {
    let downloads = sqlx::query_as::<_, Download>(
        "SELECT id, url, filename, save_path, total_bytes, downloaded_bytes, status, \
         CAST(created_at AS TEXT) as created_at \
         FROM downloads WHERE status != 'completed' ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(downloads)
}

/// Returns ALL downloads (active + completed) ordered newest first.
pub async fn get_all_downloads(pool: &SqlitePool) -> Result<Vec<Download>, String> {
    let downloads = sqlx::query_as::<_, Download>(
        "SELECT id, url, filename, save_path, total_bytes, downloaded_bytes, status, \
         CAST(created_at AS TEXT) as created_at \
         FROM downloads ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(downloads)
}

pub async fn update_chunk_progress(
    pool: &SqlitePool,
    chunk_id: &str,
    downloaded_bytes: i64,
) -> Result<(), String> {
    sqlx::query("UPDATE chunks SET downloaded_bytes = ? WHERE id = ?")
        .bind(downloaded_bytes)
        .bind(chunk_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
