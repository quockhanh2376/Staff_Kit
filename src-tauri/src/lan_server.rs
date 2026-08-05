use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, Query, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tokio::net::TcpListener;

use crate::{db, lan_assets, lan_auth};

#[cfg(test)]
use std::path::PathBuf;

type DbFactory = Arc<dyn Fn() -> Result<Connection, String> + Send + Sync>;

// ── Rate-limit constants ────────────────────────────────────────────────────

const SEARCH_RATE_LIMIT: u32 = 30;
const SUBMIT_RATE_LIMIT: u32 = 10;
const LAN_BODY_LIMIT: usize = 4096;

// ── Application state ───────────────────────────────────────────────────────

#[derive(Clone)]
struct LanServerState {
    db_factory: DbFactory,
    token_store: Arc<lan_auth::LanTokenStore>,
}

// ── Minimal DTO for LAN asset responses ─────────────────────────────────────

/// Minimal asset summary returned by LAN API endpoints.
/// Does NOT include internal `id`, `notes`, or `status` fields.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LanAssetSummary {
    asset_code: String,
    asset_type: String,
    display_name: String,
    model: Option<String>,
    serial_number: Option<String>,
}

impl From<&db::AssetRecord> for LanAssetSummary {
    fn from(record: &db::AssetRecord) -> Self {
        Self {
            asset_code: record.asset_code.clone(),
            asset_type: record.asset_type.clone(),
            display_name: record.display_name.clone(),
            model: record.model.clone(),
            serial_number: record.serial_number.clone(),
        }
    }
}

// ── Query/error DTOs ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AssetSearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

/// LAN submit input — carries requestId/clientSessionId for replay protection
/// alongside the borrow request fields. `submit_source_ip` is NOT accepted from
/// the client; the server derives it from the peer socket.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanSubmitInput {
    submitted_employee_id: String,
    submitted_full_name: String,
    asset_codes: Vec<String>,
    request_type: Option<String>,
    /// Required: unique per-submission ID for replay protection.
    request_id: String,
    /// Optional: per-browser-session ID for audit traceability.
    client_session_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiErrorPayload {
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LanSubmitResponse {
    request_reference: String,
    status: String,
    message: String,
}

// ── Sanitized error helpers ─────────────────────────────────────────────────

/// 500 — generic message, never exposes internal details.
fn internal_error() -> (StatusCode, Json<ApiErrorPayload>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiErrorPayload {
            error: "The LAN service could not complete the request.".to_string(),
        }),
    )
}

/// 400 — surfaces safe business-rule messages from the backend.
fn bad_request_error(message: &str) -> (StatusCode, Json<ApiErrorPayload>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorPayload {
            error: message.to_string(),
        }),
    )
}

/// Auth-layer error → HTTP response. Never includes the token.
fn auth_error(err: &lan_auth::LanAuthError) -> (StatusCode, Json<ApiErrorPayload>) {
    let (status, message) = match err {
        lan_auth::LanAuthError::Missing | lan_auth::LanAuthError::Malformed => {
            (StatusCode::UNAUTHORIZED, "LAN access token is required.")
        }
        lan_auth::LanAuthError::Invalid => (
            StatusCode::UNAUTHORIZED,
            "LAN access token is invalid or revoked.",
        ),
        lan_auth::LanAuthError::DuplicateRequest => (
            StatusCode::CONFLICT,
            "This request was already submitted recently.",
        ),
        lan_auth::LanAuthError::RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many requests. Please try again later.",
        ),
    };
    (
        status,
        Json(ApiErrorPayload {
            error: message.to_string(),
        }),
    )
}

// ── Token extraction ────────────────────────────────────────────────────────

/// Extract the token from `Authorization: Bearer <token>` header.
fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Result<String, lan_auth::LanAuthError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(lan_auth::LanAuthError::Missing)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or(lan_auth::LanAuthError::Malformed)?;

    if token.is_empty() {
        return Err(lan_auth::LanAuthError::Missing);
    }
    Ok(token.to_string())
}

/// Authenticate API requests before any body extractor runs.
async fn require_bearer(
    State(token_store): State<Arc<lan_auth::LanTokenStore>>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Response {
    let result = extract_bearer_token(request.headers())
        .and_then(|token| token_store.verify(&token).map(|_| ()));
    match result {
        Ok(()) => next.run(request).await,
        Err(error) => auth_error(&error).into_response(),
    }
}

// ── Firewall rule ───────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn ensure_firewall_rule(port: u16) {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const RULE_NAME: &str = "Staff Kit LAN Borrow Server";

    let _ = Command::new("netsh")
        .args([
            "advfirewall",
            "firewall",
            "delete",
            "rule",
            &format!("name={RULE_NAME}"),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            &format!("name={RULE_NAME}"),
            "protocol=TCP", "dir=in", "action=allow",
            &format!("localport={port}"),
            "profile=private,domain",
            "description=Allows Staff Kit employees on the local network to submit borrow requests via QR code.",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(not(target_os = "windows"))]
fn ensure_firewall_rule(_port: u16) {}

// ── Server startup ──────────────────────────────────────────────────────────

pub fn start(app: AppHandle, token_store: Arc<lan_auth::LanTokenStore>) -> Result<(), String> {
    let lan_settings = db::get_borrow_lan_settings(&app)?;
    let bind_addr = format!("0.0.0.0:{}", lan_settings.port);

    ensure_firewall_rule(lan_settings.port);

    let app_handle = app.clone();
    let db_factory: DbFactory = Arc::new(move || db::open_runtime_connection(&app_handle));
    let router = build_router(db_factory, token_store);

    tauri::async_runtime::spawn(async move {
        match TcpListener::bind(bind_addr.as_str()).await {
            Ok(listener) => {
                if let Err(error) = axum::serve(
                    listener,
                    router.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                {
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

// ── Router ──────────────────────────────────────────────────────────────────

fn build_router(db_factory: DbFactory, token_store: Arc<lan_auth::LanTokenStore>) -> Router {
    let api_routes = Router::new()
        .route("/api/assets", get(search_assets))
        .route("/api/assigned-assets", get(search_assigned_assets))
        .route("/api/borrow-requests", post(submit_borrow_request))
        .layer(axum::extract::DefaultBodyLimit::max(LAN_BODY_LIMIT))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&token_store),
            require_bearer,
        ));

    Router::new()
        .route("/borrow", get(serve_borrow_page))
        .merge(api_routes)
        .with_state(LanServerState {
            db_factory,
            token_store,
        })
}

// ── Handlers ────────────────────────────────────────────────────────────────

async fn serve_borrow_page() -> Html<&'static str> {
    // The token is deliberately carried in the URL fragment and therefore is
    // never sent to this handler or included in HTTP request logs.
    Html(lan_assets::borrow_page_html())
}

async fn search_assets(
    State(state): State<LanServerState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    Query(query): Query<AssetSearchQuery>,
) -> Result<Json<Vec<LanAssetSummary>>, (StatusCode, Json<ApiErrorPayload>)> {
    // Rate limit by peer IP + endpoint group
    let rl_key = format!("{}:search", peer_addr.ip());
    state
        .token_store
        .check_rate_limit(&rl_key, SEARCH_RATE_LIMIT)
        .map_err(|e| auth_error(&e))?;

    let conn = (state.db_factory)().map_err(|_| internal_error())?;
    let items = db::asset::search_in_stock_assets_conn(
        &conn,
        query.q.as_deref(),
        query.limit.unwrap_or(12).min(100),
    )
    .map_err(|_| internal_error())?;

    let summaries: Vec<LanAssetSummary> = items.iter().map(LanAssetSummary::from).collect();
    Ok(Json(summaries))
}

async fn search_assigned_assets(
    State(state): State<LanServerState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    Query(query): Query<AssetSearchQuery>,
) -> Result<Json<Vec<LanAssetSummary>>, (StatusCode, Json<ApiErrorPayload>)> {
    let rl_key = format!("{}:search", peer_addr.ip());
    state
        .token_store
        .check_rate_limit(&rl_key, SEARCH_RATE_LIMIT)
        .map_err(|e| auth_error(&e))?;

    let conn = (state.db_factory)().map_err(|_| internal_error())?;
    let items = db::asset::search_assigned_assets_conn(
        &conn,
        query.q.as_deref(),
        query.limit.unwrap_or(12).min(100),
    )
    .map_err(|_| internal_error())?;

    let summaries: Vec<LanAssetSummary> = items.iter().map(LanAssetSummary::from).collect();
    Ok(Json(summaries))
}

async fn submit_borrow_request(
    State(state): State<LanServerState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<LanSubmitInput>,
) -> Result<Json<LanSubmitResponse>, (StatusCode, Json<ApiErrorPayload>)> {
    // Authentication is enforced by `require_bearer` before this JSON
    // extractor runs. Only authenticated requests reach this handler.
    // Rate limit by peer IP + endpoint group.
    let rl_key = format!("{}:submit", peer_addr.ip());
    state
        .token_store
        .check_rate_limit(&rl_key, SUBMIT_RATE_LIMIT)
        .map_err(|e| auth_error(&e))?;

    // Validate input fields before replay check and DB mutation.
    if payload.submitted_employee_id.trim().is_empty() {
        return Err(bad_request_error("submittedEmployeeId is required"));
    }
    if payload.submitted_employee_id.len() > 50 {
        return Err(bad_request_error(
            "submittedEmployeeId is too long (max 50)",
        ));
    }
    if payload.submitted_full_name.trim().is_empty() {
        return Err(bad_request_error("submittedFullName is required"));
    }
    if payload.submitted_full_name.len() > 200 {
        return Err(bad_request_error("submittedFullName is too long (max 200)"));
    }
    if payload.asset_codes.is_empty() {
        return Err(bad_request_error("at least one assetCode is required"));
    }
    if payload.asset_codes.len() > 20 {
        return Err(bad_request_error("too many asset codes (max 20)"));
    }
    // requestId is required and must be a non-empty bounded string.
    let request_id = payload.request_id.as_str();
    if request_id.trim().is_empty() {
        return Err(bad_request_error("requestId is required"));
    }
    if request_id.len() > 100 {
        return Err(bad_request_error("requestId is too long (max 100)"));
    }
    // clientSessionId is optional but if present must be bounded.
    if let Some(ref sid) = payload.client_session_id {
        if sid.len() > 100 {
            return Err(bad_request_error("clientSessionId is too long (max 100)"));
        }
    }

    // Replay check — AFTER input validation, BEFORE DB mutation.
    //    A validation failure above does NOT consume the requestId.
    state
        .token_store
        .check_and_record_request(request_id)
        .map_err(|e| auth_error(&e))?;

    // Derive authoritative peer IP (ignore client-supplied submitSourceIp).
    let peer_ip = peer_addr.ip().to_string();

    // Build the backend DTO with authoritative peer IP.
    let backend_payload = db::BorrowRequestSubmitInput {
        submitted_employee_id: payload.submitted_employee_id,
        submitted_full_name: payload.submitted_full_name,
        asset_codes: payload.asset_codes,
        submit_source_ip: Some(peer_ip),
        request_type: payload.request_type,
    };

    // Execute DB mutation. Never return raw backend error text to the LAN.
    let mut conn = (state.db_factory)().map_err(|_| internal_error())?;
    match db::borrow::submit_borrow_request_conn(&mut conn, backend_payload) {
        Ok(record) => Ok(Json(LanSubmitResponse {
            request_reference: record.request_key,
            status: record.status,
            message: "Request submitted for IT review.".to_string(),
        })),
        Err(_) => Err(bad_request_error(
            "The request could not be submitted. Please check the employee and asset details.",
        )),
    }
}

// ── Test infrastructure ─────────────────────────────────────────────────────

#[cfg(test)]
async fn build_router_for_tests(
    token_store: Arc<lan_auth::LanTokenStore>,
) -> (Router, LanServerTestHarness) {
    let harness = LanServerTestHarness::new();
    let router = build_router(harness.db_factory(), token_store);
    (router, harness)
}

#[cfg(test)]
async fn build_router_for_tests_no_token() -> (Router, LanServerTestHarness) {
    let harness = LanServerTestHarness::new();
    let token_store = Arc::new(lan_auth::LanTokenStore::new());
    let router = build_router(harness.db_factory(), token_store);
    (router, harness)
}

#[cfg(test)]
async fn build_router_for_tests_with_token() -> (Router, LanServerTestHarness, String) {
    let token_store = Arc::new(lan_auth::LanTokenStore::new());
    let token = token_store.issue();
    let harness = LanServerTestHarness::new();
    let router = build_router(harness.db_factory(), token_store);
    (router, harness, token)
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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::connect_info::MockConnectInfo,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    fn mock_addr() -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], 12345))
    }

    /// Build a router with MockConnectInfo so ConnectInfo extractor works in tests.
    fn test_router(router: Router) -> Router {
        router.layer(MockConnectInfo(mock_addr()))
    }

    async fn send(router: Router, req: Request<Body>) -> axum::response::Response {
        test_router(router)
            .oneshot(req)
            .await
            .expect("router responds")
    }

    fn bearer(token: &str) -> String {
        format!("Bearer {token}")
    }

    // ── /borrow page tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn borrow_page_served_without_token_param() {
        let (router, _h) = build_router_for_tests_no_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/borrow")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn borrow_page_is_served_without_query_token_authentication() {
        let (router, _h, _token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/borrow")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn borrow_page_does_not_read_query_token_param() {
        let (router, _h, _token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/borrow?t=not-a-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── /api/assets auth tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn assets_without_bearer_returns_401() {
        let (router, _h) = build_router_for_tests_no_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn assets_with_wrong_bearer_returns_401() {
        let (router, _h, _token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=test")
                .header("authorization", "Bearer AAAA".repeat(11))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn assets_with_malformed_bearer_returns_401() {
        let (router, _h, _token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=test")
                .header("authorization", "NotBearer stuff")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn assets_with_empty_bearer_returns_401() {
        let (router, _h, _token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=test")
                .header("authorization", "Bearer ")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn assets_with_valid_bearer_returns_results() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=asset")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body_text = String::from_utf8_lossy(&body);
        assert!(!body_text.contains(&token));
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(payload.as_array().unwrap().len(), 1);
        assert_eq!(payload[0]["assetCode"], "ASSET-001");
    }

    #[tokio::test]
    async fn assets_response_has_no_internal_fields() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=asset")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        let item = &payload[0];
        assert!(item.get("id").is_none(), "id must not be in LAN DTO");
        assert!(item.get("notes").is_none(), "notes must not be in LAN DTO");
        assert!(
            item.get("status").is_none(),
            "status must not be in LAN DTO"
        );
        assert!(item.get("assetCode").is_some());
        assert!(item.get("assetType").is_some());
        assert!(item.get("displayName").is_some());
    }

    // ── /api/assigned-assets auth tests ──────────────────────────────────────

    #[tokio::test]
    async fn assigned_assets_without_bearer_returns_401() {
        let (router, _h) = build_router_for_tests_no_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assigned-assets?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn assigned_assets_with_valid_bearer_returns_results() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assigned-assets?q=asset")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(payload.as_array().unwrap().len(), 1);
        assert_eq!(payload[0]["assetCode"], "ASSET-002");
    }

    // ── /api/borrow-requests auth tests ──────────────────────────────────────

    #[tokio::test]
    async fn submit_without_bearer_returns_401() {
        let (router, _h) = build_router_for_tests_no_token().await;
        let response = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "submittedEmployeeId": "EE1001",
                        "submittedFullName": "Test",
                        "assetCodes": ["ASSET-001"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_token_and_invalid_json_returns_401_before_body_deserialization() {
        let (router, _h) = build_router_for_tests_no_token().await;
        let response = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", "Bearer invalid")
                .body(Body::from("{not-json"))
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn submit_with_valid_bearer_creates_pending_request() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", bearer(&token))
                .body(Body::from(
                    json!({
                        "submittedEmployeeId": "EE1001",
                        "submittedFullName": "Nguyen Van A",
                        "assetCodes": ["ASSET-001"],
                        "requestId": "request-valid-1",
                        "clientSessionId": "client-session-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert!(!String::from_utf8_lossy(&body).contains(&token));
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(payload["status"], "pending");
        assert!(payload["requestReference"].as_str().is_some());
        assert_eq!(payload["message"], "Request submitted for IT review.");
        assert!(payload.get("id").is_none());
        assert!(payload.get("decisionNote").is_none());
        assert!(payload.get("assetCodes").is_none());
    }

    #[tokio::test]
    async fn submit_invalid_employee_returns_400() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", bearer(&token))
                .body(Body::from(
                    json!({
                        "submittedEmployeeId": "UNKNOWN",
                        "submittedFullName": "Ghost",
                        "assetCodes": ["ASSET-001"],
                        "requestId": "request-invalid-employee"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn submit_empty_employee_returns_400() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let response = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", bearer(&token))
                .body(Body::from(
                    json!({
                        "submittedEmployeeId": "",
                        "submittedFullName": "Ghost",
                        "assetCodes": ["ASSET-001"],
                        "requestId": "request-empty-employee"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ── Body size limit ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn submit_oversized_body_rejected() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let big_payload = "x".repeat(5000);
        let body_json = json!({
            "submittedEmployeeId": "EE1001",
            "submittedFullName": &big_payload,
            "assetCodes": ["ASSET-001"]
        })
        .to_string();

        let response = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", bearer(&token))
                .body(Body::from(body_json))
                .unwrap(),
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "body over 4 KB must be rejected"
        );
    }

    // ── Rate limiting ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_rate_limit_exceeded_returns_429() {
        let token_store = Arc::new(lan_auth::LanTokenStore::new());
        let token = token_store.issue();
        let (router, _h) = build_router_for_tests(Arc::clone(&token_store)).await;
        let svc = test_router(router);

        for _ in 0..30 {
            let response = svc
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/assets?q=test")
                        .header("authorization", bearer(&token))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert!(response.status() != StatusCode::TOO_MANY_REQUESTS);
        }

        let response = svc
            .oneshot(
                Request::builder()
                    .uri("/api/assets?q=test")
                    .header("authorization", bearer(&token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn submit_rate_limit_exceeded_returns_429() {
        let token_store = Arc::new(lan_auth::LanTokenStore::new());
        let token = token_store.issue();
        let (router, _h) = build_router_for_tests(Arc::clone(&token_store)).await;
        let svc = test_router(router);

        for attempt in 0..10 {
            let response = svc
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/borrow-requests")
                        .header("content-type", "application/json")
                        .header("authorization", bearer(&token))
                        .body(Body::from(
                            json!({
                                "submittedEmployeeId": "BAD",
                                "submittedFullName": "Test",
                                "assetCodes": ["NONEXIST"],
                                "requestId": format!("rate-limit-{attempt}")
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let response = svc
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/borrow-requests")
                    .header("content-type", "application/json")
                    .header("authorization", bearer(&token))
                    .body(Body::from(
                        json!({
                            "submittedEmployeeId": "EE1001",
                            "submittedFullName": "Test",
                            "assetCodes": ["ASSET-001"],
                            "requestId": "rate-limit-final"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // ── Token not leaked in error responses ─────────────────────────────────

    #[tokio::test]
    async fn token_not_leaked_in_error_response() {
        let token_store = Arc::new(lan_auth::LanTokenStore::new());
        let real_token = token_store.issue();
        let (router, _h) = build_router_for_tests(token_store).await;

        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=test")
                .header("authorization", format!("Bearer {real_token}___tampered"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            !body_str.contains(&real_token),
            "real token must not appear in error response body"
        );
    }

    // ── Internal errors are sanitized ───────────────────────────────────────

    #[tokio::test]
    async fn internal_errors_return_generic_message() {
        let token_store = Arc::new(lan_auth::LanTokenStore::new());
        let token = token_store.issue();
        let db_factory: DbFactory =
            Arc::new(|| Err("C:\\secret\\staff.sqlite3: database is locked".to_string()));
        let router = build_router(db_factory, token_store);

        let response = send(
            router,
            Request::builder()
                .uri("/api/assets?q=test")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            !body_str.contains("C:\\") && !body_str.contains("database is locked"),
            "internal error details must not leak"
        );
        assert!(
            body_str.contains("LAN service"),
            "should contain generic message"
        );
    }
}
