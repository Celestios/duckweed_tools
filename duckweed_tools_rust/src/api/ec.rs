//! POST /api/ec/forward, POST /api/ec/reverse

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::calc::composition::valagro;
use crate::calc::ec::{estimate_ec, dose_for_target_ec};
use crate::data::store::AppState;
use crate::server::AppError;

#[derive(Deserialize)]
pub struct ECForwardRequest {
    pub dose_g_per_L: f64,
    #[serde(default = "default_scale")]
    pub scale: String,
}

#[derive(Deserialize)]
pub struct ECReverseRequest {
    pub target_ec: f64,
    #[serde(default = "default_scale")]
    pub scale: String,
}

fn default_scale() -> String { "700".to_string() }

pub async fn ec_forward(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ECForwardRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fert = valagro();
    let result = estimate_ec(req.dose_g_per_L, &req.scale, &fert)
        .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "dose_g_per_L": result.dose_g_per_l,
        "total_dissolved_solids_ppm": result.total_dissolved_solids_ppm,
        "scale": result.scale,
        "estimated_EC_mS_cm": result.estimated_ec_ms_cm,
    })))
}

pub async fn ec_reverse(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ECReverseRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fert = valagro();
    let result = dose_for_target_ec(req.target_ec, &req.scale, &fert)
        .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "dose_g_per_L": result.dose_g_per_l,
        "total_dissolved_solids_ppm": result.total_dissolved_solids_ppm,
        "scale": result.scale,
        "estimated_EC_mS_cm": result.estimated_ec_ms_cm,
    })))
}
