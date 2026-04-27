use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tauri::{AppHandle, Emitter};
use futures_util::StreamExt;
use uuid::Uuid;
use tokio::time::Instant;

use crate::models::{Chunk, ProgressPayload};
use crate::queries;

pub async fn download_file(
    app: AppHandle,
    pool: sqlx::SqlitePool,
    url: String,
    save_dir: String,
    download_id: String,
) -> Result<(), String> {
    // Ensure the save directory exists before writing anything
    tokio::fs::create_dir_all(&save_dir).await.map_err(|e| e.to_string())?;

    // ── Build client ─────────────────────────────────────────────────────────
    let client = Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/124.0.0.0 Safari/537.36",
        )
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.to_string())?;

    // ── 1. HEAD probe ─────────────────────────────────────────────────────────
    let head_res = client
        .head(&url)
        .send()
        .await
        .map_err(|e| format!("HEAD request failed: {e}"))?;

    let head_status = head_res.status();
    if !head_status.is_success() {
        return Err(format!(
            "Server rejected HEAD with HTTP {}",
            head_status.as_u16()
        ));
    }

    let final_url = head_res.url().to_string();
    tracing::debug!("Final URL after redirects: {}", final_url);

    let total_bytes: i64 = head_res
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let accepts_ranges = head_res
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v != "none" && !v.is_empty())
        .unwrap_or(false);

    // ── Derive filename ───────────────────────────────────────────────────────
    let url_parsed = reqwest::Url::parse(&final_url).map_err(|e| e.to_string())?;
    let filename = url_parsed
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "download".to_string());

    let save_path = Path::new(&save_dir)
        .join(&filename)
        .to_string_lossy()
        .to_string();

    // ── 2. Persist download record ────────────────────────────────────────────
    queries::insert_download(
        &pool,
        download_id.clone(),
        url.clone(),
        filename.clone(),
        save_path.clone(),
        total_bytes,
    )
    .await?;

    // ── 3. Chunk strategy ─────────────────────────────────────────────────────
    let use_ranges = accepts_ranges && total_bytes > 0;
    let num_chunks: i64 = if use_ranges { 4 } else { 1 };
    let chunk_size: i64 = if use_ranges { total_bytes / num_chunks } else { 0 };

    let mut chunks = Vec::new();
    for i in 0..num_chunks {
        let (start_byte, end_byte) = if use_ranges {
            let start = i * chunk_size;
            let end = if i == num_chunks - 1 {
                total_bytes - 1
            } else {
                (i + 1) * chunk_size - 1
            };
            (start, end)
        } else {
            (0_i64, -1_i64)
        };

        chunks.push(Chunk {
            id: Uuid::new_v4().to_string(),
            download_id: download_id.clone(),
            start_byte,
            end_byte,
            downloaded_bytes: 0,
            status: "pending".to_string(),
        });
    }

    queries::insert_chunks(&pool, chunks.clone()).await?;

    sqlx::query("UPDATE downloads SET status = 'downloading' WHERE id = ?")
        .bind(&download_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    // ── 4 & 5. Spawn tasks ────────────────────────────────────────────────────
    let mut handles = Vec::new();

    for chunk in chunks.clone() {
        let client_c   = client.clone();
        let app_c      = app.clone();
        let url_c      = final_url.clone();
        let save_dir_c = save_dir.clone();
        let filename_c = filename.clone();
        let pool_c     = pool.clone();

        let handle = tokio::spawn(async move {
            let part_filename = format!("{}.part_{}", filename_c, chunk.id);
            let part_path     = Path::new(&save_dir_c).join(&part_filename);

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&part_path)
                .await
                .map_err(|e| e.to_string())?;

            let mut request = client_c.get(&url_c);
            if chunk.end_byte >= 0 {
                request = request.header(
                    reqwest::header::RANGE,
                    format!("bytes={}-{}", chunk.start_byte, chunk.end_byte),
                );
            }

            let res = request.send().await.map_err(|e| e.to_string())?;
            let status = res.status();
            if !status.is_success() {
                return Err(format!("HTTP {} — {}", status.as_u16(), status.canonical_reason().unwrap_or("error")));
            }

            let mut stream = res.bytes_stream();
            let mut downloaded: i64 = 0;
            let mut last_db_update = Instant::now();
            let mut last_emit = Instant::now();
            let mut bytes_since_last_emit: i64 = 0;

            while let Some(chunk_result) = stream.next().await {
                let bytes = chunk_result.map_err(|e| e.to_string())?;
                file.write_all(&bytes).await.map_err(|e| e.to_string())?;

                let len = bytes.len() as i64;
                downloaded += len;
                bytes_since_last_emit += len;

                if last_emit.elapsed().as_millis() >= 100 {
                    let elapsed_secs = last_emit.elapsed().as_secs_f64();
                    let speed = if elapsed_secs > 0.0 { bytes_since_last_emit as f64 / elapsed_secs } else { 0.0 };
                    let _ = app_c.emit("download-progress", ProgressPayload {
                        download_id: chunk.download_id.clone(),
                        chunk_id: chunk.id.clone(),
                        downloaded_bytes: downloaded,
                        speed_bytes_per_sec: speed,
                    });
                    last_emit = Instant::now();
                    bytes_since_last_emit = 0;
                }

                if last_db_update.elapsed().as_millis() >= 1000 {
                    let _ = queries::update_chunk_progress(&pool_c, &chunk.id, downloaded).await;
                    last_db_update = Instant::now();
                }
            }

            file.flush().await.map_err(|e| e.to_string())?;
            drop(file);
            let _ = queries::update_chunk_progress(&pool_c, &chunk.id, downloaded).await;

            if downloaded == 0 {
                return Err("Server returned empty response".to_string());
            }

            Ok::<(PathBuf, i64), String>((part_path, downloaded))
        });

        handles.push(handle);
    }

    // ── 6. Collect results & Cleanup on error ──────────────────────────────────
    let mut part_paths: Vec<PathBuf> = Vec::new();
    let mut total_downloaded: i64    = 0;
    let mut any_error = None;

    for handle in handles {
        match handle.await.map_err(|_| "Worker thread panicked".to_string())? {
            Ok((path, bytes)) => {
                part_paths.push(path);
                total_downloaded += bytes;
            }
            Err(e) => {
                any_error = Some(e);
                break;
            }
        }
    }

    if let Some(e) = any_error {
        // Cleanup all .part files for this download
        for chunk in chunks {
            let part_filename = format!("{}.part_{}", filename, chunk.id);
            let part_path = Path::new(&save_dir).join(&part_filename);
            let _ = tokio::fs::remove_file(part_path).await;
        }
        let _ = sqlx::query("UPDATE downloads SET status = 'failed' WHERE id = ?").bind(&download_id).execute(&pool).await;
        return Err(e);
    }

    // ── 7. Stitch parts ───────────────────────────────────────────────────────
    let final_path = Path::new(&save_dir).join(&filename);
    let mut final_file = OpenOptions::new().create(true).write(true).truncate(true).open(&final_path).await.map_err(|e| e.to_string())?;

    for part_path in part_paths {
        let mut part_file = File::open(&part_path).await.map_err(|e| e.to_string())?;
        tokio::io::copy(&mut part_file, &mut final_file).await.map_err(|e| e.to_string())?;
        tokio::fs::remove_file(&part_path).await.map_err(|e| e.to_string())?;
    }

    final_file.flush().await.map_err(|e| e.to_string())?;

    // ── 8. Finalise DB row ────────────────────────────────────────────────────
    let real_total = if total_bytes > 0 { total_bytes } else { total_downloaded };
    sqlx::query("UPDATE downloads SET status = 'completed', total_bytes = ?, downloaded_bytes = ? WHERE id = ?")
        .bind(real_total).bind(total_downloaded).bind(&download_id).execute(&pool).await.map_err(|e| e.to_string())?;

    tracing::info!("Download complete: {} ({} bytes)", filename, total_downloaded);
    Ok(())
}
