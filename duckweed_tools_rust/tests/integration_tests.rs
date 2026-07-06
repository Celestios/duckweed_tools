use std::collections::HashMap;
use std::path::PathBuf;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use duckweed_server::data::store::AppState;
use duckweed_server::server::create_router;

// Helper to create the test router with a temporary directory
fn setup_test_app() -> (axum::Router, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = AppState::new(temp_dir.path().to_path_buf());
    
    // Seed some mock data to container_types and log if needed
    {
        let mut db = state.db.lock().unwrap();
        db.container_types.insert(
            "Small Tub".to_string(),
            duckweed_server::data::profiles::ContainerData {
                width_cm: 15.5,
                length_cm: 23.0,
                height_cm: 5.0,
            },
        );
    }
    
    let app = create_router(state);
    (app, temp_dir)
}

#[tokio::test]
async fn test_dosing_forward_endpoint() {
    let (app, _dir) = setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dosing/forward")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "dose_g_per_L": 1.0,
                        "water_volume_L": 1.0,
                        "source": "valagro"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(res["source_name"].as_str().unwrap(), "VALAGRO MASTER 15-5-30+TE");
    assert_eq!(res["ppm"]["N_total"].as_f64().unwrap(), 150.0);
}

#[tokio::test]
async fn test_dosing_reverse_endpoint() {
    let (app, _dir) = setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dosing/reverse")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "target_ppm": 150.0,
                        "nutrient": "N_total",
                        "water_volume_L": 2.0,
                        "source": "valagro"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(res["dose_g_per_L"].as_f64().unwrap(), 1.0);
    assert_eq!(res["total_grams"].as_f64().unwrap(), 2.0);
}

#[tokio::test]
async fn test_ec_endpoints() {
    let (app, _dir) = setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ec/forward")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "dose_g_per_L": 1.0,
                        "scale": "700"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert!(res["estimated_EC_mS_cm"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_container_stock_endpoint() {
    let (app, _dir) = setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/container-stock")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "container_name": "Small Tub",
                        "dosing_interval_days": 7.0,
                        "coverage_fraction": 0.8,
                        "include_urea": true,
                        "include_iron": true,
                        "water_depth_cm": 1.5
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(res["container_name"].as_str().unwrap(), "Small Tub");
    assert!(res["stock_volume_L"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_log_endpoints() {
    let (app, _dir) = setup_test_app();

    // 1. Get empty log
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert!(res["log"].as_array().unwrap().is_empty());

    // 2. Add log entry
    let response2 = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/log")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "day": 1,
                        "light_source": "LED 15W",
                        "light_distance_cm": 15.0,
                        "photoperiod_hours": 16.0,
                        "containers": {
                            "Tub-1": {
                                "type": "Small Tub",
                                "water_depth_cm": 1.5,
                                "coverage_percent": 80.0,
                                "tds_ppm": 250,
                                "biomass_status": "healthy",
                                "additives": []
                            }
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_db_export_import() {
    let (app, _dir) = setup_test_app();

    // 1. Export database
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/db/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let db_val: Value = serde_json::from_slice(&body).unwrap();
    
    // Verify it contains the default struct elements
    assert!(db_val.get("container_types").is_some());
    assert!(db_val.get("log").is_some());

    // 2. Import database (with an added container type)
    let mut db_to_import = db_val.clone();
    db_to_import["container_types"]["New Imported Tub"] = json!({
        "width_cm": 20.0,
        "length_cm": 30.0,
        "height_cm": 10.0
    });

    let response2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/db/import")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&db_to_import).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response2.status(), StatusCode::OK);

    // 3. Verify it was loaded successfully
    let response3 = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/db/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response3.status(), StatusCode::OK);
    let body3 = response3.into_body().collect().await.unwrap().to_bytes();
    let db_val3: Value = serde_json::from_slice(&body3).unwrap();
    assert!(db_val3["container_types"].get("New Imported Tub").is_some());
}

