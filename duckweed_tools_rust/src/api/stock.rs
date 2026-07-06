//! POST /api/stock/forward, POST /api/stock/reverse

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::calc::composition::valagro;
use crate::calc::stock::{build_stock_for_target_final_dose, final_dose_from_stock};
use crate::data::store::AppState;
use crate::server::AppError;

#[derive(Deserialize)]
pub struct StockForwardRequest {
    pub final_dose_g_per_L: f64,
    pub dilution_ratio: f64,
    #[serde(default = "default_one")]
    pub stock_volume_L: f64,
}

#[derive(Deserialize)]
pub struct StockReverseRequest {
    pub stock_grams: f64,
    #[serde(default = "default_one")]
    pub stock_volume_L: f64,
    pub dilution_ratio: f64,
}

fn default_one() -> f64 { 1.0 }

pub async fn stock_forward(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<StockForwardRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fert = valagro();
    let plan = build_stock_for_target_final_dose(
        req.final_dose_g_per_L,
        req.dilution_ratio,
        req.stock_volume_L,
        &fert,
    )
    .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "stock_volume_L": plan.stock_volume_l,
        "stock_grams": plan.stock_grams,
        "stock_dose_g_per_L": plan.stock_dose_g_per_l,
        "dilution_ratio": plan.dilution_ratio,
        "final_dose_g_per_L": plan.final_dose_g_per_l,
        "final_ppm": plan.final_ppm.ppm,
    })))
}

pub async fn stock_reverse(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<StockReverseRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fert = valagro();
    let plan = final_dose_from_stock(
        req.stock_grams,
        req.stock_volume_L,
        req.dilution_ratio,
        &fert,
    )
    .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "stock_volume_L": plan.stock_volume_l,
        "stock_grams": plan.stock_grams,
        "stock_dose_g_per_L": plan.stock_dose_g_per_l,
        "dilution_ratio": plan.dilution_ratio,
        "final_dose_g_per_L": plan.final_dose_g_per_l,
        "final_ppm": plan.final_ppm.ppm,
    })))
}
