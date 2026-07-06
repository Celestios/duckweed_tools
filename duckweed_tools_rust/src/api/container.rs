//! POST /api/container-stock

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::calc::composition::valagro;
use crate::calc::stock::calculate_stock_for_container_schedule;
use crate::data::store::{AppState, load_container_profiles};
use crate::server::AppError;

#[derive(Deserialize)]
pub struct ContainerStockRequest {
    pub container_name: String,
    #[serde(default = "default_interval")]
    pub dosing_interval_days: f64,
    #[serde(default = "default_coverage")]
    pub coverage_fraction: f64,
    #[serde(default = "default_true")]
    pub include_urea: bool,
    #[serde(default = "default_true")]
    pub include_iron: bool,
    #[serde(default = "default_depth")]
    pub water_depth_cm: f64,
}

fn default_interval() -> f64 { 7.0 }
fn default_coverage() -> f64 { 0.8 }
fn default_true() -> bool { true }
fn default_depth() -> f64 { 1.5 }

pub async fn container_stock(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ContainerStockRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.lock().unwrap();
    let profiles = load_container_profiles(&db);

    let container = profiles
        .get(&req.container_name)
        .ok_or_else(|| {
            AppError::bad_request(format!(
                "Container '{}' not found in catalog.",
                req.container_name
            ))
        })?
        .clone();
    drop(db);

    let fert = valagro();
    let plan = calculate_stock_for_container_schedule(
        &container,
        req.dosing_interval_days,
        req.coverage_fraction,
        req.include_urea,
        req.include_iron,
        &fert,
        req.water_depth_cm,
    )
    .map_err(AppError::bad_request)?;

    Ok(Json(json!({
        "container_name": plan.container_name,
        "surface_area_m2": plan.surface_area_m2,
        "water_depth_cm": plan.water_depth_cm,
        "vessel_volume_L": plan.vessel_volume_l,
        "stock_lifespan_days": plan.stock_lifespan_days,
        "dosing_cycle_days": plan.dosing_cycle_days,
        "injection_interval_days": plan.injection_interval_days,
        "stock_volume_L": plan.stock_volume_l,
        "dose_volume_mL": plan.dose_volume_ml,
        "injection_volume_mL": plan.injection_volume_ml,
        "number_of_doses": plan.number_of_doses,
        "number_of_injections_per_cycle": plan.number_of_injections_per_cycle,
        "valagro_g_per_cycle": plan.valagro_g_per_cycle,
        "urea_g_per_cycle": plan.urea_g_per_cycle,
        "iron_g_per_cycle": plan.iron_g_per_cycle,
        "valagro_g_in_stock": plan.valagro_g_in_stock,
        "urea_g_in_stock": plan.urea_g_in_stock,
        "iron_g_in_stock": plan.iron_g_in_stock,
        "cumulative_ppm": plan.cumulative_ppm,
        "injection_ppm": plan.injection_ppm,
        "warnings": plan.warnings,
    })))
}
