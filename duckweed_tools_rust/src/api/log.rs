//! GET /api/log, POST /api/log, POST /api/log/export

use std::sync::Arc;
use std::collections::HashMap;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::data::store::{AppState, Database};
use crate::log::manager::export_to_markdown;
use crate::server::AppError;

#[derive(Deserialize)]
pub struct LogEntryContainer {
    #[serde(rename = "type")]
    pub container_type: String,
    #[serde(default = "default_depth")]
    pub water_depth_cm: f64,
    #[serde(default = "default_coverage")]
    pub coverage_percent: f64,
    pub tds_ppm: Option<i64>,
    #[serde(default = "default_status")]
    pub biomass_status: String,
    #[serde(default)]
    pub additives: Vec<HashMap<String, String>>,
}

#[derive(Deserialize)]
pub struct LogTransfer {
    pub from_container: String,
    pub to_container: String,
    pub amount: String,
}

#[derive(Deserialize)]
pub struct LogEntryRequest {
    pub day: Option<i64>,
    pub light_source: String,
    pub light_distance_cm: Option<f64>,
    pub photoperiod_start: Option<f64>,
    pub photoperiod_end: Option<f64>,
    #[serde(default)]
    pub containers: HashMap<String, LogEntryContainer>,
    #[serde(default)]
    pub transfers: Vec<LogTransfer>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(default)]
    pub discussions: Vec<String>,
    #[serde(default)]
    pub images: Vec<HashMap<String, String>>,
}

fn default_depth() -> f64 { 1.5 }
fn default_coverage() -> f64 { 80.0 }
fn default_status() -> String { "healthy".to_string() }

pub async fn get_log(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    Json(json!({
        "log": db.log,
        "container_types": db.container_types,
        "light_types": db.light_types,
        "fertilizer_types": db.fertilizer_types,
        "containers": db.containers,
        "light_sources": db.light_sources,
    }))
}

pub async fn add_log_entry(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogEntryRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut db = state.db.lock().unwrap();

    // Auto-calculate next day
    let day = req.day.unwrap_or_else(|| {
        db.log
            .iter()
            .filter_map(|e| e.get("day").and_then(|v| v.as_i64()))
            .max()
            .unwrap_or(0)
            + 1
    });

    // Remove existing entry for this day (overwrite)
    db.log.retain(|e| {
        e.get("day").and_then(|v| v.as_i64()).unwrap_or(-1) != day
    });

    // Build containers dict
    let mut containers_json = serde_json::Map::new();
    for (cid, cdata) in &req.containers {
        containers_json.insert(
            cid.clone(),
            json!({
                "type": cdata.container_type,
                "water_depth_cm": cdata.water_depth_cm,
                "coverage_percent": cdata.coverage_percent,
                "tds_ppm": cdata.tds_ppm,
                "biomass_status": cdata.biomass_status,
                "additives": cdata.additives,
            }),
        );
    }

    // Build transfers
    let transfers_json: Vec<Value> = req
        .transfers
        .iter()
        .map(|t| {
            json!({
                "from": t.from_container,
                "to": t.to_container,
                "amount": t.amount,
            })
        })
        .collect();

    let new_entry = json!({
        "day": day,
        "light_source": req.light_source,
        "light_distance_cm": req.light_distance_cm,
        "photoperiod_start": req.photoperiod_start,
        "photoperiod_end": req.photoperiod_end,
        "containers": containers_json,
        "transfers": transfers_json,
        "operations": req.operations,
        "observations": req.observations,
        "discussions": req.discussions,
        "images": req.images,
    });

    db.log.push(new_entry);
    db.log.sort_by(|a, b| {
        let da = a.get("day").and_then(|v| v.as_i64()).unwrap_or(0);
        let db_day = b.get("day").and_then(|v| v.as_i64()).unwrap_or(0);
        da.cmp(&db_day)
    });

    // Save
    let data_dir = state.data_dir.clone();
    drop(db);
    state.save().map_err(AppError::internal)?;

    // Auto-export to markdown (ignore errors)
    let db = state.db.lock().unwrap();
    let _ = export_to_markdown(&db, &data_dir);

    Ok(Json(json!({"status": "created", "day": day})))
}

pub async fn export_log(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.lock().unwrap();
    export_to_markdown(&db, &state.data_dir).map_err(AppError::internal)?;
    Ok(Json(json!({"status": "exported"})))
}

pub async fn export_db(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.lock().unwrap();
    let val = serde_json::to_value(&*db).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(val))
}

pub async fn import_db(
    State(state): State<Arc<AppState>>,
    Json(val): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_db: Database = serde_json::from_value(val).map_err(|e| {
        AppError::bad_request(format!("Invalid database format: {}", e))
    })?;

    let mut db = state.db.lock().unwrap();
    *db = new_db;

    let data_dir = state.data_dir.clone();
    drop(db);
    state.save().map_err(AppError::internal)?;

    let db = state.db.lock().unwrap();
    let _ = export_to_markdown(&db, &data_dir);

    Ok(Json(json!({"status": "imported"})))
}
