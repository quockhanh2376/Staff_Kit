use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

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
use tauri::async_runtime::JoinHandle;
use tauri::AppHandle;
use tokio::net::TcpListener;

use crate::{db, lan_assets, lan_auth};

#[cfg(test)]
use std::path::PathBuf;

type DbFactory = Arc<dyn Fn() -> Result<Connection, String> + Send + Sync>;
type SettingsFactory = Arc<dyn Fn() -> Result<db::BorrowLanSettings, String> + Send + Sync>;

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

struct ManagedLanServer {
    task: JoinHandle<()>,
    bind_host: String,
    port: u16,
}

#[cfg(test)]
struct LanLifecycleTestGate {
    events: Mutex<Vec<LanLifecycleTestEvent>>,
    listener_ready: tokio::sync::Notify,
    stop_started: tokio::sync::Notify,
    release_stop: tokio::sync::Notify,
    start_attempted: tokio::sync::Notify,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanLifecycleTestEvent {
    ListenerReady,
    StopShutdownStarted,
    StartAttempted,
    StartAcquiredLifecycle,
    ListenerTerminated,
    StopCompleted,
    StartCompleted,
}

#[cfg(test)]
impl LanLifecycleTestGate {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            listener_ready: tokio::sync::Notify::new(),
            stop_started: tokio::sync::Notify::new(),
            release_stop: tokio::sync::Notify::new(),
            start_attempted: tokio::sync::Notify::new(),
        }
    }

    fn record(&self, event: LanLifecycleTestEvent) {
        self.events
            .lock()
            .expect("lifecycle test events lock")
            .push(event);
    }

    fn events(&self) -> Vec<LanLifecycleTestEvent> {
        self.events
            .lock()
            .expect("lifecycle test events lock")
            .clone()
    }
}

#[derive(Clone)]
pub struct LanServerManager {
    settings_factory: SettingsFactory,
    db_factory: DbFactory,
    token_store: Arc<lan_auth::LanTokenStore>,
    server: Arc<Mutex<Option<ManagedLanServer>>>,
    lifecycle: Arc<tokio::sync::Mutex<()>>,
    #[cfg(test)]
    lifecycle_test_gate: Arc<Mutex<Option<Arc<LanLifecycleTestGate>>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanServerStatus {
    pub running: bool,
    pub token_ready: bool,
    pub bind_host: String,
    pub port: u16,
}

impl LanServerManager {
    pub fn new(app: AppHandle, token_store: Arc<lan_auth::LanTokenStore>) -> Self {
        let settings_app = app.clone();
        let settings_factory: SettingsFactory =
            Arc::new(move || db::get_borrow_lan_settings(&settings_app));
        let db_app = app.clone();
        let db_factory: DbFactory = Arc::new(move || db::open_runtime_connection(&db_app));
        Self::with_factories(settings_factory, db_factory, token_store)
    }

    fn with_factories(
        settings_factory: SettingsFactory,
        db_factory: DbFactory,
        token_store: Arc<lan_auth::LanTokenStore>,
    ) -> Self {
        Self {
            settings_factory,
            db_factory,
            token_store,
            server: Arc::new(Mutex::new(None)),
            lifecycle: Arc::new(tokio::sync::Mutex::new(())),
            #[cfg(test)]
            lifecycle_test_gate: Arc::new(Mutex::new(None)),
        }
    }

    #[cfg(test)]
    fn set_lifecycle_test_gate(&self, gate: Option<Arc<LanLifecycleTestGate>>) {
        *self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock") = gate;
    }

    pub async fn start_if_enabled(&self) -> Result<(), String> {
        if (self.settings_factory)()?.enabled {
            self.start().await
        } else {
            Ok(())
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        #[cfg(test)]
        let test_gate = self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock")
            .clone();
        #[cfg(test)]
        if let Some(gate) = test_gate.clone() {
            gate.start_attempted.notify_one();
            gate.record(LanLifecycleTestEvent::StartAttempted);
        }
        let _lifecycle = self.lifecycle.lock().await;
        #[cfg(test)]
        if let Some(gate) = self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock")
            .clone()
        {
            gate.record(LanLifecycleTestEvent::StartAcquiredLifecycle);
        }
        let settings = (self.settings_factory)()?;
        if !settings.enabled {
            return Err("Borrow LAN server is disabled in settings.".to_string());
        }

        let mut server = self
            .server
            .lock()
            .map_err(|_| "LAN server state is unavailable.".to_string())?;
        if let Some(existing) = server.as_ref() {
            if !existing.task.inner().is_finished() {
                return Err("Borrow LAN server is already running.".to_string());
            }
        }
        if server.take().is_some() {
            self.token_store.revoke();
        }

        let std_listener = std::net::TcpListener::bind(("0.0.0.0", settings.port))
            .map_err(|_| "Borrow LAN server could not start on the configured port.".to_string())?;
        std_listener
            .set_nonblocking(true)
            .map_err(|_| "Borrow LAN server could not configure its listener.".to_string())?;
        let listener = TcpListener::from_std(std_listener)
            .map_err(|_| "Borrow LAN server could not configure its listener.".to_string())?;

        ensure_firewall_rule(settings.port);
        let router = build_router(Arc::clone(&self.db_factory), Arc::clone(&self.token_store));
        let token_store = Arc::clone(&self.token_store);
        let task = tauri::async_runtime::spawn(async move {
            #[cfg(test)]
            if let Some(gate) = test_gate.clone() {
                gate.record(LanLifecycleTestEvent::ListenerReady);
                gate.listener_ready.notify_one();
            }
            let _ = axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
            token_store.revoke();
        });

        *server = Some(ManagedLanServer {
            task,
            bind_host: settings.host,
            port: settings.port,
        });
        #[cfg(test)]
        if let Some(gate) = self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock")
            .clone()
        {
            gate.record(LanLifecycleTestEvent::StartCompleted);
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        let task = {
            let mut server = self
                .server
                .lock()
                .map_err(|_| "LAN server state is unavailable.".to_string())?;
            server.take().map(|existing| existing.task)
        };
        #[cfg(test)]
        let test_gate = self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock")
            .clone();
        #[cfg(test)]
        if let Some(gate) = test_gate {
            gate.record(LanLifecycleTestEvent::StopShutdownStarted);
            gate.stop_started.notify_one();
            gate.release_stop.notified().await;
        }
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
        #[cfg(test)]
        if let Some(gate) = self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock")
            .clone()
        {
            gate.record(LanLifecycleTestEvent::ListenerTerminated);
        }
        self.token_store.revoke();
        #[cfg(test)]
        if let Some(gate) = self
            .lifecycle_test_gate
            .lock()
            .expect("lifecycle test gate lock")
            .clone()
        {
            gate.record(LanLifecycleTestEvent::StopCompleted);
        }
        Ok(())
    }

    pub fn require_running(&self) -> Result<(), String> {
        if self.status()?.running {
            Ok(())
        } else {
            Err("Borrow LAN server is stopped.".to_string())
        }
    }

    pub fn status(&self) -> Result<LanServerStatus, String> {
        let settings = (self.settings_factory)()?;
        let mut server = self
            .server
            .lock()
            .map_err(|_| "LAN server state is unavailable.".to_string())?;
        if server
            .as_ref()
            .is_some_and(|current| current.task.inner().is_finished())
        {
            server.take();
            self.token_store.revoke();
        }
        let current = server.as_ref();
        Ok(LanServerStatus {
            running: current.is_some(),
            token_ready: current.is_some() && self.token_store.is_ready(),
            bind_host: current
                .map(|value| value.bind_host.clone())
                .unwrap_or(settings.host),
            port: current.map(|value| value.port).unwrap_or(settings.port),
        })
    }
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
/// alongside the borrow request fields. Source IP is never accepted from the
/// client and is derived only from the peer socket for rate limiting.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanSubmitInput {
    submitted_employee_id: String,
    submitted_full_name: String,
    asset_codes: Vec<String>,
    request_type: String,
    /// Required: unique per-submission ID for replay protection.
    request_id: String,
    /// Required: per-browser-session ID for audit traceability.
    client_session_id: String,
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
    if payload.client_session_id.trim().is_empty() {
        return Err(bad_request_error("clientSessionId is required"));
    }
    if payload.client_session_id.len() > 100 {
        return Err(bad_request_error("clientSessionId is too long (max 100)"));
    }
    if payload.request_type != "borrow" && payload.request_type != "return" {
        return Err(bad_request_error("requestType must be borrow or return"));
    }

    // Build the backend DTO. The DB layer derives employee identity and does
    // not accept a client-supplied source IP.
    let backend_payload = db::BorrowRequestSubmitInput {
        submitted_employee_id: payload.submitted_employee_id,
        submitted_full_name: payload.submitted_full_name,
        asset_codes: payload.asset_codes,
        request_type: Some(payload.request_type),
        manual_employee_email: None,
        manual_employee_team: None,
    };

    // Perform authoritative employee and asset validation before consuming the
    // replay ID. The final submission revalidates under BEGIN IMMEDIATE.
    let mut conn = (state.db_factory)().map_err(|_| internal_error())?;
    db::borrow::validate_borrow_request_conn(&mut conn, backend_payload.clone()).map_err(|_| {
        bad_request_error(
            "The request could not be submitted. Please check the employee and asset details.",
        )
    })?;

    // Replay check — after syntax and business validation, before DB mutation.
    // A validation failure does not consume the requestId.
    state
        .token_store
        .check_and_record_request(request_id)
        .map_err(|e| auth_error(&e))?;

    // Execute DB mutation. Never return raw backend error text to the LAN.
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
    use crate::db::BorrowLanSettings;
    use axum::{
        body::Body,
        extract::connect_info::MockConnectInfo,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use std::sync::Mutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tower::ServiceExt;

    fn mock_addr() -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], 12345))
    }

    /// Build a router with MockConnectInfo so ConnectInfo extractor works in tests.
    fn test_router(router: Router) -> Router {
        router.layer(MockConnectInfo(mock_addr()))
    }

    fn ephemeral_port() -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("ephemeral port");
        listener.local_addr().expect("listener address").port()
    }

    fn test_manager(
        harness: &LanServerTestHarness,
        enabled: bool,
        port: u16,
    ) -> (
        LanServerManager,
        Arc<Mutex<BorrowLanSettings>>,
        Arc<lan_auth::LanTokenStore>,
    ) {
        let settings = Arc::new(Mutex::new(db::BorrowLanSettings {
            enabled,
            host: "127.0.0.1".to_string(),
            port,
            borrow_url: format!("http://127.0.0.1:{port}/borrow"),
        }));
        let settings_source = Arc::clone(&settings);
        let settings_factory: SettingsFactory = Arc::new(move || {
            settings_source
                .lock()
                .map(|value| value.clone())
                .map_err(|_| "test settings unavailable".to_string())
        });
        let token_store = Arc::new(lan_auth::LanTokenStore::new());
        let manager_token_store = Arc::clone(&token_store);
        (
            LanServerManager::with_factories(settings_factory, harness.db_factory(), token_store),
            settings,
            manager_token_store,
        )
    }

    async fn wait_for_listener(port: u16) {
        for _ in 0..50 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("listener did not start on port {port}");
    }

    async fn listener_accepts(port: u16) -> bool {
        TcpStream::connect(("127.0.0.1", port)).await.is_ok()
    }

    async fn api_status(port: u16, token: &str) -> Option<StatusCode> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.ok()?;
        let request = format!(
            "GET /api/assets HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.ok()?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.ok()?;
        let first_line = response.split(|byte| *byte == b'\n').next()?;
        let status = String::from_utf8_lossy(first_line);
        status
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse().ok())
    }

    #[tokio::test]
    async fn disabled_startup_does_not_bind_listener() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, _) = test_manager(&harness, false, port);

        manager
            .start_if_enabled()
            .await
            .expect("disabled startup is safe");
        assert!(!manager.status().expect("status").running);
        assert!(!listener_accepts(port).await);
    }

    #[tokio::test]
    async fn enabled_start_transitions_to_running_and_duplicate_start_is_rejected() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, _) = test_manager(&harness, true, port);

        manager.start().await.expect("enabled server starts");
        wait_for_listener(port).await;
        assert!(manager.status().expect("status").running);
        assert!(
            manager.start().await.is_err(),
            "duplicate start must be rejected"
        );
        manager.stop().await.expect("stop");
    }

    #[tokio::test]
    async fn stop_waits_for_socket_release_and_duplicate_stop_is_safe() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, _) = test_manager(&harness, true, port);

        manager.start().await.expect("start");
        wait_for_listener(port).await;
        manager.stop().await.expect("stop waits for task");
        assert!(!manager.status().expect("stopped status").running);
        assert!(
            !listener_accepts(port).await,
            "socket must be released before stop returns"
        );
        manager.stop().await.expect("duplicate stop is safe");
        manager.start().await.expect("immediate restart");
        wait_for_listener(port).await;
        manager.stop().await.expect("final stop");
    }

    #[tokio::test]
    async fn concurrent_stop_then_start_serializes_lifecycle_transition() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, _) = test_manager(&harness, true, port);

        let initial_gate = Arc::new(LanLifecycleTestGate::new());
        manager.set_lifecycle_test_gate(Some(Arc::clone(&initial_gate)));
        let listener_ready = initial_gate.listener_ready.notified();
        manager.start().await.expect("start");
        listener_ready.await;

        let gate = Arc::new(LanLifecycleTestGate::new());
        manager.set_lifecycle_test_gate(Some(Arc::clone(&gate)));
        let stop_started = gate.stop_started.notified();
        let stopping_manager = manager.clone();
        let stop_task = tokio::spawn(async move { stopping_manager.stop().await });
        stop_started.await;

        let start_attempted = gate.start_attempted.notified();
        let restarting_manager = manager.clone();
        let start_task = tokio::spawn(async move { restarting_manager.start().await });
        start_attempted.await;

        gate.release_stop.notify_one();
        assert!(stop_task.await.expect("stop task join").is_ok());
        let restarted_listener_ready = gate.listener_ready.notified();
        let restart = start_task.await.expect("start task join");
        assert!(
            restart.is_ok(),
            "restart after termination must succeed: {restart:?}"
        );
        restarted_listener_ready.await;
        assert!(manager.status().expect("running status").running);
        let events = gate.events();
        let event_index = |event| {
            events
                .iter()
                .position(|current| *current == event)
                .expect("expected lifecycle event")
        };
        assert!(
            event_index(LanLifecycleTestEvent::StopShutdownStarted)
                < event_index(LanLifecycleTestEvent::StartAttempted),
            "Start must be attempted after Stop begins"
        );
        assert!(
            event_index(LanLifecycleTestEvent::StartAttempted)
                < event_index(LanLifecycleTestEvent::ListenerTerminated),
            "Start must not acquire lifecycle control before listener termination"
        );
        assert!(
            event_index(LanLifecycleTestEvent::ListenerTerminated)
                < event_index(LanLifecycleTestEvent::StopCompleted),
            "Stop must complete listener termination before completing"
        );
        assert!(
            event_index(LanLifecycleTestEvent::StopCompleted)
                < event_index(LanLifecycleTestEvent::StartAcquiredLifecycle),
            "Start must acquire lifecycle control only after Stop completes"
        );
        assert!(
            event_index(LanLifecycleTestEvent::StartAcquiredLifecycle)
                < event_index(LanLifecycleTestEvent::StartCompleted),
            "Start must complete after acquiring lifecycle control"
        );
        manager.set_lifecycle_test_gate(None);
        manager.stop().await.expect("cleanup");
    }

    #[tokio::test]
    async fn stop_revokes_token_and_stale_bearer_is_rejected_after_stop() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, token_store) = test_manager(&harness, true, port);
        manager.start().await.expect("start");
        wait_for_listener(port).await;

        let token = token_store.issue();
        assert_eq!(api_status(port, &token).await, Some(StatusCode::OK));
        manager.stop().await.expect("stop");
        assert!(matches!(
            token_store.verify(&token),
            Err(lan_auth::LanAuthError::Invalid)
        ));
        let router = build_router_for_tests(token_store.clone()).await.0;
        let response = send(
            router,
            Request::builder()
                .uri("/api/assets")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn issue_while_stopped_is_rejected_and_restart_creates_fresh_lifecycle() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, settings, token_store) = test_manager(&harness, true, port);

        assert!(manager.require_running().is_err());
        manager.start().await.expect("start");
        wait_for_listener(port).await;
        let old_token = token_store.issue();
        manager.stop().await.expect("stop");
        assert!(manager.require_running().is_err());
        assert!(matches!(
            token_store.verify(&old_token),
            Err(lan_auth::LanAuthError::Invalid)
        ));

        settings.lock().expect("settings").port = port;
        manager.start().await.expect("restart");
        wait_for_listener(port).await;
        let new_token = token_store.issue();
        assert_ne!(old_token, new_token);
        manager.stop().await.expect("final stop");
    }

    #[tokio::test]
    async fn unexpected_listener_exit_is_removed_from_status_and_revokes_token() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, token_store) = test_manager(&harness, true, port);
        manager.start().await.expect("start");
        wait_for_listener(port).await;
        let token = token_store.issue();
        manager
            .server
            .lock()
            .expect("server state")
            .as_ref()
            .expect("managed task")
            .task
            .abort();
        for _ in 0..50 {
            if !manager.status().expect("status").running {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(!manager.status().expect("final status").running);
        assert!(matches!(
            token_store.verify(&token),
            Err(lan_auth::LanAuthError::Invalid)
        ));
        manager.stop().await.expect("cleanup");
    }

    #[tokio::test]
    async fn status_reports_running_and_token_ready_independently() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, token_store) = test_manager(&harness, true, port);

        let stopped = manager.status().expect("stopped status");
        assert!(!stopped.running);
        assert!(!stopped.token_ready);
        manager.start().await.expect("start");
        wait_for_listener(port).await;
        let running_without_token = manager.status().expect("running status");
        assert!(running_without_token.running);
        assert!(!running_without_token.token_ready);
        token_store.issue();
        assert!(manager.status().expect("ready status").token_ready);
        manager.stop().await.expect("cleanup");
    }

    #[tokio::test]
    async fn application_shutdown_closes_listener_and_revokes_token() {
        let harness = LanServerTestHarness::new();
        let port = ephemeral_port();
        let (manager, _, token_store) = test_manager(&harness, true, port);
        manager.start().await.expect("start");
        wait_for_listener(port).await;
        let token = token_store.issue();

        // The Tauri close handler calls the same awaited manager stop path.
        manager.stop().await.expect("shutdown stop");
        assert!(!listener_accepts(port).await);
        assert!(matches!(
            token_store.verify(&token),
            Err(lan_auth::LanAuthError::Invalid)
        ));
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
                        "requestType": "borrow",
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
    async fn submit_unknown_employee_uses_manual_entry() {
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
                        "requestType": "borrow",
                        "clientSessionId": "client-session-invalid-employee",
                        "requestId": "request-invalid-employee"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), LAN_BODY_LIMIT)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "pending");
    }

    #[tokio::test]
    async fn replay_id_is_not_consumed_by_business_validation_failure() {
        let (router, _h, token) = build_router_for_tests_with_token().await;
        let invalid = send(
            router.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", bearer(&token))
                .body(Body::from(
                    json!({
                        "submittedEmployeeId": "EE1001",
                        "submittedFullName": "Ignored",
                        "assetCodes": ["DOES-NOT-EXIST"],
                        "requestType": "borrow",
                        "clientSessionId": "client-session-retry",
                        "requestId": "request-retry-after-validation"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

        let valid = send(
            router,
            Request::builder()
                .method("POST")
                .uri("/api/borrow-requests")
                .header("content-type", "application/json")
                .header("authorization", bearer(&token))
                .body(Body::from(
                    json!({
                        "submittedEmployeeId": "EE1001",
                        "submittedFullName": "Ignored",
                        "assetCodes": ["ASSET-001"],
                        "requestType": "borrow",
                        "clientSessionId": "client-session-retry",
                        "requestId": "request-retry-after-validation"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(valid.status(), StatusCode::OK);
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
                        "requestType": "borrow",
                        "clientSessionId": "client-session-empty-employee",
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
                                "requestType": "borrow",
                                "clientSessionId": "client-session-rate-search",
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
                            "requestType": "borrow",
                            "clientSessionId": "client-session-rate-submit",
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

    #[test]
    fn server_status_does_not_serialize_token_material() {
        let status = LanServerStatus {
            running: true,
            token_ready: true,
            bind_host: "192.168.1.10".to_string(),
            port: 8787,
        };
        let json = serde_json::to_string(&status).expect("status serializes");
        assert!(json.contains("running"));
        assert!(json.contains("tokenReady"));
        assert!(!json.contains("Bearer"));
        assert!(!json.contains("abc123"));
    }
}
