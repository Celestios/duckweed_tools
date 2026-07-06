//! POST /api/simulator

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::calc::simulator::{DuckweedVessel, simulate_weekly_program};
use crate::data::store::AppState;
use crate::server::AppError;

#[derive(Deserialize)]
pub struct SimulatorRequest {
    #[serde(default = "default_one")]
    pub volume_L: f64,
    #[serde(default = "default_width")]
    pub width_cm: f64,
    #[serde(default = "default_length")]
    pub length_cm: f64,
    #[serde(default = "default_valagro")]
    pub valagro_g_per_week: f64,
    #[serde(default = "default_urea")]
    pub urea_g_per_week: f64,
    #[serde(default = "default_iron")]
    pub iron_g_per_week: f64,
    #[serde(default = "default_weeks")]
    pub weeks: i32,
    #[serde(default = "default_exchange")]
    pub exchange_fraction: f64,
}

fn default_one() -> f64 { 1.0 }
fn default_width() -> f64 { 15.5 }
fn default_length() -> f64 { 23.0 }
fn default_valagro() -> f64 { 0.5 }
fn default_urea() -> f64 { 0.1 }
fn default_iron() -> f64 { 0.07 }
fn default_weeks() -> i32 { 6 }
fn default_exchange() -> f64 { 0.3 }

pub async fn simulator(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SimulatorRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut vessel = DuckweedVessel::new(req.volume_L, req.width_cm, req.length_cm)
        .map_err(AppError::bad_request)?;

    let snapshots = simulate_weekly_program(
        &mut vessel,
        req.valagro_g_per_week,
        req.urea_g_per_week,
        req.weeks,
        req.exchange_fraction,
        req.iron_g_per_week,
    )
    .map_err(AppError::bad_request)?;

    let weeks_json: Vec<_> = snapshots
        .iter()
        .map(|s| {
            json!({
                "week": s.week,
                "concentrations": s.concentrations_mg_l,
                "statuses": s.statuses,
            })
        })
        .collect();

    Ok(Json(json!({
        "volume_L": req.volume_L,
        "surface_area_m2": vessel.surface_area_m2(),
        "weeks": weeks_json,
    })))
}
