//! POST /api/dosing/forward, POST /api/dosing/reverse

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::calc::composition::resolve_source;
use crate::calc::dosing::{ppm_from_dose, dose_for_target_ppm};
use crate::data::store::AppState;
use crate::server::AppError;

#[derive(Deserialize)]
pub struct DosingForwardRequest {
    pub dose_g_per_L: f64,
    #[serde(default = "default_water_volume")]
    pub water_volume_L: f64,
    #[serde(default = "default_source")]
    pub source: String,
}

#[derive(Deserialize)]
pub struct DosingReverseRequest {
    pub target_ppm: f64,
    #[serde(default = "default_nutrient")]
    pub nutrient: String,
    #[serde(default = "default_water_volume")]
    pub water_volume_L: f64,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_water_volume() -> f64 { 1.0 }
fn default_source() -> String { "valagro".to_string() }
fn default_nutrient() -> String { "N_total".to_string() }

pub async fn dosing_forward(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<DosingForwardRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fert = resolve_source(&req.source);
    let result = ppm_from_dose(req.dose_g_per_L, req.water_volume_L, &fert)
        .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "source_name": result.source_name,
        "dose_g_per_L": result.dose_g_per_l,
        "water_volume_L": result.water_volume_l,
        "total_grams": result.total_grams,
        "ppm": result.ppm,
    })))
}

pub async fn dosing_reverse(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<DosingReverseRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fert = resolve_source(&req.source);
    let result = dose_for_target_ppm(req.target_ppm, &req.nutrient, req.water_volume_L, &fert)
        .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "source_name": result.source_name,
        "dose_g_per_L": result.dose_g_per_l,
        "water_volume_L": result.water_volume_l,
        "total_grams": result.total_grams,
        "ppm": result.ppm,
    })))
}
