//! GET/POST/PUT/DELETE for /api/catalog/containers, /lights, /fertilizers

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::data::profiles::{ContainerProfile, LightProfile, FertilizerProfile};
use crate::data::store::{
    AppState, load_container_profiles, save_container_profile, delete_container_profile,
    load_light_profiles, save_light_profile, delete_light_profile,
    load_fertilizer_profiles, save_fertilizer_profile, delete_fertilizer_profile,
};
use crate::server::AppError;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------
#[derive(Deserialize)]
pub struct ContainerData {
    pub name: String,
    pub width_cm: f64,
    pub length_cm: f64,
    pub height_cm: f64,
}

#[derive(Deserialize)]
pub struct LightData {
    pub name: String,
    #[serde(rename = "wattage_W")]
    pub wattage_w: f64,
    pub lumens: f64,
    pub kelvin: f64,
}

#[derive(Deserialize)]
pub struct FertilizerData {
    pub name: String,
    #[serde(rename = "N_total")]
    pub n_total: f64,
    #[serde(rename = "P2O5")]
    pub p2o5: f64,
    #[serde(rename = "K2O")]
    pub k2o: f64,
    #[serde(rename = "MgO")]
    pub mgo: f64,
    pub trace_Fe: f64,
    #[serde(default)]
    pub trace_Mn: f64,
    #[serde(default)]
    pub trace_Zn: f64,
    #[serde(default)]
    pub trace_Cu: f64,
    #[serde(default)]
    pub trace_B: f64,
}

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------
pub async fn get_containers(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().unwrap();
    let profiles = load_container_profiles(&db);
    let result: serde_json::Map<String, serde_json::Value> = profiles
        .iter()
        .map(|(name, p)| {
            (
                name.clone(),
                json!({
                    "name": p.name,
                    "width_cm": p.width_cm,
                    "length_cm": p.length_cm,
                    "height_cm": p.height_cm,
                    "surface_area_m2": (p.surface_area_m2() * 10000.0).round() / 10000.0,
                }),
            )
        })
        .collect();
    Ok(Json(serde_json::Value::Object(result)))
}

pub async fn create_container(
    State(state): State<Arc<AppState>>,
    Json(data): Json<ContainerData>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    let profiles = load_container_profiles(&db);
    if profiles.contains_key(&data.name) {
        return Err(AppError::conflict(format!(
            "Container '{}' already exists.",
            data.name
        )));
    }
    let p = ContainerProfile {
        name: data.name.clone(),
        width_cm: data.width_cm,
        length_cm: data.length_cm,
        height_cm: data.height_cm,
    };
    save_container_profile(&mut db, &p);
    drop(db);
    state.save().map_err(AppError::internal)?;
    Ok(Json(json!({"status": "created", "name": data.name})))
}

pub async fn update_container(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(data): Json<ContainerData>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    let profiles = load_container_profiles(&db);
    if !profiles.contains_key(&name) {
        return Err(AppError::not_found(format!(
            "Container '{}' not found.",
            name
        )));
    }
    if name != data.name {
        delete_container_profile(&mut db, &name);
    }
    let p = ContainerProfile {
        name: data.name.clone(),
        width_cm: data.width_cm,
        length_cm: data.length_cm,
        height_cm: data.height_cm,
    };
    save_container_profile(&mut db, &p);
    drop(db);
    state.save().map_err(AppError::internal)?;
    Ok(Json(json!({"status": "updated", "name": data.name})))
}

pub async fn remove_container(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    if delete_container_profile(&mut db, &name) {
        drop(db);
        state.save().map_err(AppError::internal)?;
        Ok(Json(json!({"status": "deleted", "name": name})))
    } else {
        Err(AppError::not_found(format!(
            "Container '{}' not found.",
            name
        )))
    }
}

// ---------------------------------------------------------------------------
// Lights
// ---------------------------------------------------------------------------
pub async fn get_lights(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().unwrap();
    let profiles = load_light_profiles(&db);
    let result: serde_json::Map<String, serde_json::Value> = profiles
        .iter()
        .map(|(name, p)| {
            (
                name.clone(),
                json!({
                    "name": p.name,
                    "wattage_W": p.wattage_w,
                    "lumens": p.lumens,
                    "kelvin": p.kelvin,
                }),
            )
        })
        .collect();
    Ok(Json(serde_json::Value::Object(result)))
}

pub async fn create_light(
    State(state): State<Arc<AppState>>,
    Json(data): Json<LightData>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    let profiles = load_light_profiles(&db);
    if profiles.contains_key(&data.name) {
        return Err(AppError::conflict(format!(
            "Light '{}' already exists.",
            data.name
        )));
    }
    let p = LightProfile {
        name: data.name.clone(),
        wattage_w: data.wattage_w,
        lumens: data.lumens,
        kelvin: data.kelvin,
    };
    save_light_profile(&mut db, &p);
    drop(db);
    state.save().map_err(AppError::internal)?;
    Ok(Json(json!({"status": "created", "name": data.name})))
}

pub async fn update_light(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(data): Json<LightData>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    let profiles = load_light_profiles(&db);
    if !profiles.contains_key(&name) {
        return Err(AppError::not_found(format!("Light '{}' not found.", name)));
    }
    if name != data.name {
        delete_light_profile(&mut db, &name);
    }
    let p = LightProfile {
        name: data.name.clone(),
        wattage_w: data.wattage_w,
        lumens: data.lumens,
        kelvin: data.kelvin,
    };
    save_light_profile(&mut db, &p);
    drop(db);
    state.save().map_err(AppError::internal)?;
    Ok(Json(json!({"status": "updated", "name": data.name})))
}

pub async fn remove_light(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    if delete_light_profile(&mut db, &name) {
        drop(db);
        state.save().map_err(AppError::internal)?;
        Ok(Json(json!({"status": "deleted", "name": name})))
    } else {
        Err(AppError::not_found(format!("Light '{}' not found.", name)))
    }
}

// ---------------------------------------------------------------------------
// Fertilizers
// ---------------------------------------------------------------------------
pub async fn get_fertilizers(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let db = state.db.lock().unwrap();
    let profiles = load_fertilizer_profiles(&db);
    let result: serde_json::Map<String, serde_json::Value> = profiles
        .iter()
        .map(|(name, p)| {
            (
                name.clone(),
                json!({
                    "name": p.name,
                    "N_total": p.n_total,
                    "P2O5": p.p2o5,
                    "K2O": p.k2o,
                    "MgO": p.mgo,
                    "trace_Fe": p.trace_Fe,
                    "trace_Mn": p.trace_Mn,
                    "trace_Zn": p.trace_Zn,
                    "trace_Cu": p.trace_Cu,
                    "trace_B": p.trace_B,
                }),
            )
        })
        .collect();
    Ok(Json(serde_json::Value::Object(result)))
}

pub async fn create_fertilizer(
    State(state): State<Arc<AppState>>,
    Json(data): Json<FertilizerData>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    let profiles = load_fertilizer_profiles(&db);
    if profiles.contains_key(&data.name) {
        return Err(AppError::conflict(format!(
            "Fertilizer '{}' already exists.",
            data.name
        )));
    }
    let p = FertilizerProfile {
        name: data.name.clone(),
        n_total: data.n_total,
        p2o5: data.p2o5,
        k2o: data.k2o,
        mgo: data.mgo,
        trace_Fe: data.trace_Fe,
        trace_Mn: data.trace_Mn,
        trace_Zn: data.trace_Zn,
        trace_Cu: data.trace_Cu,
        trace_B: data.trace_B,
        shelf_life_days: 365,
    };
    save_fertilizer_profile(&mut db, &p);
    drop(db);
    state.save().map_err(AppError::internal)?;
    Ok(Json(json!({"status": "created", "name": data.name})))
}

pub async fn update_fertilizer(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(data): Json<FertilizerData>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    let profiles = load_fertilizer_profiles(&db);
    if !profiles.contains_key(&name) {
        return Err(AppError::not_found(format!(
            "Fertilizer '{}' not found.",
            name
        )));
    }
    if name != data.name {
        delete_fertilizer_profile(&mut db, &name);
    }
    let p = FertilizerProfile {
        name: data.name.clone(),
        n_total: data.n_total,
        p2o5: data.p2o5,
        k2o: data.k2o,
        mgo: data.mgo,
        trace_Fe: data.trace_Fe,
        trace_Mn: data.trace_Mn,
        trace_Zn: data.trace_Zn,
        trace_Cu: data.trace_Cu,
        trace_B: data.trace_B,
        shelf_life_days: 365,
    };
    save_fertilizer_profile(&mut db, &p);
    drop(db);
    state.save().map_err(AppError::internal)?;
    Ok(Json(json!({"status": "updated", "name": data.name})))
}

pub async fn remove_fertilizer(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut db = state.db.lock().unwrap();
    if delete_fertilizer_profile(&mut db, &name) {
        drop(db);
        state.save().map_err(AppError::internal)?;
        Ok(Json(json!({"status": "deleted", "name": name})))
    } else {
        Err(AppError::not_found(format!(
            "Fertilizer '{}' not found.",
            name
        )))
    }
}
