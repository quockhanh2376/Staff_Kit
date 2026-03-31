use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tokio::net::TcpListener;

use crate::{db, lan_assets};

#[cfg(test)]
use std::path::PathBuf;

type DbFactory = Arc<dyn Fn() -> Result<Connection, String> + Send + Sync>;

#[derive(Clone)]
struct LanServerState {
    db_factory: DbFactory,
}

#[derive(Debug, Deserialize)]
struct AssetSearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiErrorPayload {
    error: String,
}

pub fn start(app: AppHandle) -> Result<(), String> {
    let lan_settings = db::get_borrow_lan_settings(&app)?;
    let bind_addr = format!("0.0.0.0:{}", lan_settings.port);

    let app_handle = app.clone();
    let db_factory: DbFactory = Arc::new(move || db::open_runtime_connection(&app_handle));
    let router = build_router(db_factory);

    tauri::async_runtime::spawn(async move {
        match TcpListener::bind(bind_addr.as_str()).await {
            Ok(listener) => {
                if let Err(error) = axum::serve(listener, router).await {
                    eprintln!("failed to serve Staff Kit LAN borrow server: {error}");
                }
            }
            Err(error) => {
                eprintln!(
                    "failed to bind Staff Kit LAN borrow server on {}: {}",
                    bind_addr, error
                );
            }
        }
    });

    Ok(())
}

fn build_router(db_factory: DbFactory) -> Router {
    Router::new()
        .route("/borrow", get(serve_borrow_page))
        .route("/api/assets", get(search_assets))
        .route("/api/borrow-requests", post(submit_borrow_request))
        .with_state(LanServerState { db_factory })
}

async fn serve_borrow_page() -> Html<&'static str> {
    Html(lan_assets::borrow_page_html())
}

async fn search_assets(
    State(state): State<LanServerState>,
    Query(query): Query<AssetSearchQuery>,
) -> Result<Json<Vec<db::AssetRecord>>, (StatusCode, Json<ApiErrorPayload>)> {
    let conn = (state.db_factory)().map_err(internal_api_error)?;
    let items = db::asset::search_in_stock_assets_conn(
        &conn,
        query.q.as_deref(),
        query.limit.unwrap_or(12),
    )
    .map_err(internal_api_error)?;
    Ok(Json(items))
}

async fn submit_borrow_request(
    State(state): State<LanServerState>,
    Json(payload): Json<db::BorrowRequestSubmitInput>,
) -> Result<Json<db::BorrowRequestRecord>, (StatusCode, Json<ApiErrorPayload>)> {
    let mut conn = (state.db_factory)().map_err(internal_api_error)?;
    let record = db::borrow::submit_borrow_request_conn(&mut conn, payload)
        .map_err(bad_request_api_error)?;
    Ok(Json(record))
}

fn internal_api_error(message: String) -> (StatusCode, Json<ApiErrorPayload>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiErrorPayload { error: message }),
    )
}

fn bad_request_api_error(message: String) -> (StatusCode, Json<ApiErrorPayload>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorPayload { error: message }),
    )
}

#[cfg(test)]
async fn build_router_for_tests() -> (Router, LanServerTestHarness) {
    let harness = LanServerTestHarness::new();
    let router = build_router(harness.db_factory());
    (router, harness)
}

#[cfg(test)]
struct LanServerTestHarness {
    db_path: PathBuf,
}

#[cfg(test)]
impl LanServerTestHarness {
    fn new() -> Self {
        let file_name = format!("staff-kit-lan-server-{}.sqlite3", rand::random::<u64>());
        let db_path = std::env::temp_dir().join(file_name);

        let conn = Connection::open(&db_path).expect("open test sqlite file");
        db::configure_connection(&conn).expect("configure sqlite pragmas");
        db::apply_migrations(&conn).expect("apply migrations");

        conn.execute(
            r#"
            INSERT INTO employees(employee_id, full_name, updated_at)
            VALUES('EE1001', 'Nguyen Van A', datetime('now'))
            "#,
            [],
        )
        .expect("seed employee");

        conn.execute(
            r#"
            INSERT INTO assets(asset_code, asset_type, display_name, status, created_at, updated_at)
            VALUES('ASSET-001', 'Laptop', 'Dell Latitude', 'in_stock', datetime('now'), datetime('now'))
            "#,
            [],
        )
        .expect("seed in-stock asset");

        conn.execute(
            r#"
            INSERT INTO assets(asset_code, asset_type, display_name, status, created_at, updated_at)
            VALUES('ASSET-002', 'Laptop', 'Dell Latitude Old', 'assigned', datetime('now'), datetime('now'))
            "#,
            [],
        )
        .expect("seed assigned asset");

        drop(conn);

        Self { db_path }
    }

    fn db_factory(&self) -> DbFactory {
        let db_path = self.db_path.clone();
        Arc::new(move || {
            let conn = Connection::open(&db_path)
                .map_err(|error| format!("failed to open test sqlite file: {error}"))?;
            db::configure_connection(&conn)?;
            Ok(conn)
        })
    }
}

#[cfg(test)]
impl Drop for LanServerTestHarness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_file(self.db_path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(self.db_path.with_extension("sqlite3-shm"));
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn serves_borrow_page_on_borrow_route() {
        let (router, _harness) = build_router_for_tests().await;

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/borrow")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("serve borrow page");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn searches_only_in_stock_assets() {
        let (router, _harness) = build_router_for_tests().await;

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/assets?q=asset")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("search assets");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(payload.as_array().expect("array").len(), 1);
        assert_eq!(payload[0]["assetCode"], "ASSET-001");
    }

    #[tokio::test]
    async fn rejects_invalid_submit_payloads() {
        let (router, _harness) = build_router_for_tests().await;

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/borrow-requests")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "submittedEmployeeId": "UNKNOWN",
                            "submittedFullName": "Ghost User",
                            "assetCodes": ["ASSET-001"]
                        })
                        .to_string(),
                    ))
                    .expect("build request"),
            )
            .await
            .expect("submit invalid borrow request");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn accepts_valid_borrow_submit_and_creates_pending_request() {
        let (router, _harness) = build_router_for_tests().await;

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/borrow-requests")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "submittedEmployeeId": "EE1001",
                            "submittedFullName": "Nguyen Van A",
                            "assetCodes": ["ASSET-001"],
                            "submitSourceIp": "192.168.1.50"
                        })
                        .to_string(),
                    ))
                    .expect("build request"),
            )
            .await
            .expect("submit valid borrow request");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(payload["status"], "pending");
        assert_eq!(payload["assetCodes"][0], "ASSET-001");
    }
}
