//! Axum router, middleware, static file serving, and error types.

use std::sync::Arc;


use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;
use tower_http::cors::{CorsLayer, Any};
use serde_json::json;

use crate::data::store::AppState;
use crate::api;

#[derive(Embed)]
#[folder = "web/"]
struct WebAssets;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: msg.into(),
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = json!({ "detail": self.message });
        (self.status, axum::Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// Static file serving
// ---------------------------------------------------------------------------
async fn serve_index() -> impl IntoResponse {
    match WebAssets::get("index.html") {
        Some(content) => Html(
            std::str::from_utf8(content.data.as_ref())
                .unwrap_or("")
                .to_string(),
        )
        .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve_static(Path(path): Path<String>) -> impl IntoResponse {
    match WebAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            (
                [(header::CONTENT_TYPE, mime)],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve_image(
    Path(filename): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let path = state.data_dir.join("images").join(&filename);
    match std::fs::read(&path) {
        Ok(data) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            ([(header::CONTENT_TYPE, mime)], data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let shared_state = Arc::new(state);

    Router::new()
        // Web UI
        .route("/", axum::routing::get(serve_index))
        .route("/web/*path", axum::routing::get(serve_static))
        // API: Dosing
        .route("/api/dosing/forward", axum::routing::post(api::dosing::dosing_forward))
        .route("/api/dosing/reverse", axum::routing::post(api::dosing::dosing_reverse))
        // API: EC
        .route("/api/ec/forward", axum::routing::post(api::ec::ec_forward))
        .route("/api/ec/reverse", axum::routing::post(api::ec::ec_reverse))
        // API: Stock
        .route("/api/stock/forward", axum::routing::post(api::stock::stock_forward))
        .route("/api/stock/reverse", axum::routing::post(api::stock::stock_reverse))
        // API: Container Stock
        .route("/api/container-stock", axum::routing::post(api::container::container_stock))
        // API: Simulator
        .route("/api/simulator", axum::routing::post(api::simulator::simulator))
        // API: Catalog - Containers
        .route("/api/catalog/containers", axum::routing::get(api::catalog::get_containers))
        .route("/api/catalog/containers", axum::routing::post(api::catalog::create_container))
        .route("/api/catalog/containers/{name}", axum::routing::put(api::catalog::update_container))
        .route("/api/catalog/containers/{name}", axum::routing::delete(api::catalog::remove_container))
        // API: Catalog - Lights
        .route("/api/catalog/lights", axum::routing::get(api::catalog::get_lights))
        .route("/api/catalog/lights", axum::routing::post(api::catalog::create_light))
        .route("/api/catalog/lights/{name}", axum::routing::put(api::catalog::update_light))
        .route("/api/catalog/lights/{name}", axum::routing::delete(api::catalog::remove_light))
        // API: Catalog - Fertilizers
        .route("/api/catalog/fertilizers", axum::routing::get(api::catalog::get_fertilizers))
        .route("/api/catalog/fertilizers", axum::routing::post(api::catalog::create_fertilizer))
        .route("/api/catalog/fertilizers/{name}", axum::routing::put(api::catalog::update_fertilizer))
        .route("/api/catalog/fertilizers/{name}", axum::routing::delete(api::catalog::remove_fertilizer))
        // API: Log
        .route("/api/log", axum::routing::get(api::log::get_log))
        .route("/api/log", axum::routing::post(api::log::add_log_entry))
        .route("/api/log/export", axum::routing::post(api::log::export_log))
        .route("/api/db/export", axum::routing::get(api::log::export_db))
        .route("/api/db/import", axum::routing::post(api::log::import_db))
        // API: Images
        .route("/api/images", axum::routing::get(api::images::list_images))
        .route("/api/images/import", axum::routing::post(api::images::import_images))
        .route("/api/images/correlate", axum::routing::post(api::images::correlate_image))
        .route("/api/images/file/{filename}", axum::routing::get(serve_image))
        // State & middleware
        .with_state(shared_state)
        .layer(cors)
}
