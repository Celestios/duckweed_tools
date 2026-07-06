//! POST /api/images/import — upload images and correlate with log entries

use std::sync::Arc;

use axum::extract::{Multipart, State};
use axum::Json;
use serde_json::json;

use crate::data::store::AppState;
use crate::server::AppError;

pub async fn import_images(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let images_dir = state.data_dir.join("images");
    std::fs::create_dir_all(&images_dir)
        .map_err(|e| AppError::internal(format!("Failed to create images dir: {}", e)))?;

    let mut saved = Vec::new();
    let mut correlated = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("Multipart error: {}", e)))?
    {
        let filename = field
            .file_name()
            .unwrap_or("unknown")
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::bad_request(format!("Read error: {}", e)))?;

        // Save file
        let path = images_dir.join(&filename);
        std::fs::write(&path, &data)
            .map_err(|e| AppError::internal(format!("Write error: {}", e)))?;
        saved.push(filename.clone());

        // Try to correlate with log entries by filename pattern
        // Patterns: "day_N_...", "N_...", or just a number before extension
        let stem = std::path::Path::new(&filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let day_num = extract_day_number(stem);

        if let Some(day) = day_num {
            let mut db = state.db.lock().unwrap();
            for entry in &mut db.log {
                if entry.get("day").and_then(|v| v.as_i64()) == Some(day) {
                    // Ensure images array exists
                    if entry.get("images").and_then(|v| v.as_array()).is_none() {
                        entry["images"] = json!([]);
                    }
                    let images = entry["images"].as_array_mut().unwrap();
                    let already = images.iter().any(|img| {
                        img.get("filename").and_then(|v| v.as_str()) == Some(&filename)
                    });
                    if !already {
                        images.push(json!({
                            "filename": filename,
                            "description": stem,
                        }));
                        correlated.push(json!({
                            "filename": filename,
                            "day": day,
                        }));
                    }
                    break;
                }
            }
            drop(db);
            let _ = state.save();
        }
    }

    Ok(Json(json!({
        "status": "imported",
        "saved_count": saved.len(),
        "correlated_count": correlated.len(),
        "saved": saved,
        "correlated": correlated,
    })))
}

/// Try to extract a day number from a filename stem.
/// Patterns: "day5_photo", "5_img", "day_5", or just "5"
fn extract_day_number(stem: &str) -> Option<i64> {
    let lower = stem.to_lowercase();
    // "day5..." or "day_5..."
    if let Some(pos) = lower.find("day") {
        let rest = &lower[pos + 3..];
        let rest = rest.trim_start_matches('_');
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse::<i64>() {
            return Some(n);
        }
    }
    // Just a number at the start
    let num_str: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
    if let Ok(n) = num_str.parse::<i64>() {
        return Some(n);
    }
    None
}
