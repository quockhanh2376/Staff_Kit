use std::collections::HashSet;
use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr};
#[cfg(not(target_os = "windows"))]
use std::net::UdpSocket;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use png::Decoder;
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;
use unicode_normalization::UnicodeNormalization;

use super::schema::{
    BORROW_LAN_ENABLED_SETTING_KEY, BORROW_LAN_HOST_SETTING_KEY, BORROW_LAN_PORT_SETTING_KEY,
};
use super::{
    asset, audit, get_setting_value, humanize_sqlite_error, open_runtime_connection, require_text,
    set_setting_value,
};

const REQUEST_STATUS_PENDING: &str = "pending";
const REQUEST_STATUS_APPROVED: &str = "approved";
const REQUEST_STATUS_REJECTED: &str = "rejected";
const REQUEST_TYPE_BORROW: &str = "borrow";
const REQUEST_TYPE_RETURN: &str = "return";
const ASSET_STATUS_IN_STOCK: &str = "in_stock";
const ASSET_STATUS_ASSIGNED: &str = "assigned";
const ASSET_STATUS_RETIRED: &str = "retired";
const ASSET_STATUS_DISPOSED: &str = "disposed";
const ASSET_STATUS_LOST: &str = "lost";
const STAFF_GROUP_OFFBOARDING: &str = "offboarding";
const DEFAULT_BORROW_LAN_PORT: u16 = 8787;
#[cfg(not(target_os = "windows"))]
const BORROW_LAN_DETECTION_TARGETS: [&str; 3] = ["1.1.1.1:80", "8.8.8.8:80", "208.67.222.222:80"];
const HANDLE_WITH_CARE_POLICY_MAX_BYTES: usize = 20_000;

#[derive(Debug, Clone)]
pub struct HandleWithCarePolicyRecord {
    pub version: i64,
    pub text_en: String,
    pub text_vi: String,
    pub created_by_account_id: Option<i64>,
    pub created_at: String,
    pub superseded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowPolicyDesktopRecord {
    pub version: i64,
    pub text_en: String,
    pub text_vi: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowPolicyUpdateInput {
    pub text_en: String,
    pub text_vi: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BorrowLanInterfaceCandidate {
    host: Ipv4Addr,
    has_default_gateway: bool,
    is_physical: bool,
    route_metric: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationMethod {
    Signature,
    TypedName,
    Both,
}

#[derive(Debug, Clone)]
pub struct BorrowRequestConfirmationInput {
    pub policy_version: Option<i64>,
    pub policy_acknowledged: bool,
    pub confirmation_method: ConfirmationMethod,
    pub signature_png_base64: Option<String>,
    pub signature_stroke_count: Option<u32>,
    pub signature_ink_present: Option<bool>,
    pub typed_name: Option<String>,
    pub asset_codes_snapshot: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BorrowRequestWithConfirmationInput {
    pub request: BorrowRequestSubmitInput,
    pub confirmation: Option<BorrowRequestConfirmationInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowerIdentitySnapshot {
    pub employee_id_fk: Option<i64>,
    pub staff_id: String,
    pub full_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedByIdentitySnapshot {
    pub employee_id_fk: Option<i64>,
    pub staff_id: String,
    pub full_name: String,
}

#[derive(Debug, Clone)]
pub struct BorrowRequestConfirmationRecord {
    pub borrow_request_id: i64,
    pub policy_version: Option<i64>,
    pub policy_text_en_snapshot: Option<String>,
    pub policy_text_vi_snapshot: Option<String>,
    pub policy_acknowledged: bool,
    pub asset_codes_snapshot_json: String,
    pub confirmation_method: ConfirmationMethod,
    pub signature_png_blob: Option<Vec<u8>>,
    pub typed_name: Option<String>,
    pub confirmed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLanSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub borrow_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLanSettingsUpdateInput {
    #[serde(default)]
    pub enabled: Option<bool>,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRequestSubmitInput {
    pub submitted_employee_id: String,
    pub submitted_full_name: String,
    pub asset_codes: Vec<String>,
    pub request_type: Option<String>,
    #[serde(default)]
    pub manual_employee_email: Option<String>,
    #[serde(default)]
    pub manual_employee_team: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRequestRecord {
    pub id: i64,
    pub request_key: String,
    pub submitted_employee_id: String,
    pub submitted_full_name: String,
    pub status: String,
    pub request_type: String,
    pub asset_codes: Vec<String>,
    pub submitted_at: String,
    pub decision_note: Option<String>,
    pub manual_employee_email: Option<String>,
    pub manual_employee_team: Option<String>,
    pub borrower_employee_id_fk: Option<i64>,
    pub borrower_staff_id: Option<String>,
    pub borrower_name: Option<String>,
    pub submitted_by_employee_id_fk: Option<i64>,
    pub submitted_by_staff_id: Option<String>,
    pub submitted_by_name: Option<String>,
    pub has_acknowledgment: bool,
    pub confirmation_method: Option<String>,
    pub has_signature: bool,
    pub has_typed_name: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRequestEvidenceRecord {
    pub borrow_request_id: i64,
    pub policy_version: Option<i64>,
    pub policy_text_en_snapshot: Option<String>,
    pub policy_text_vi_snapshot: Option<String>,
    pub policy_acknowledged: bool,
    pub confirmation_method: String,
    pub typed_name: Option<String>,
    pub has_signature: bool,
    pub confirmed_at: String,
    pub signature_png_base64: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRequestRejectInput {
    pub request_id: i64,
    pub note: String,
}

#[derive(Debug, Clone)]
struct BorrowRequestItemRecord {
    asset_id: i64,
    asset_code_snapshot: String,
}

pub(crate) fn get_current_handle_with_care_policy_conn(
    conn: &Connection,
) -> Result<Option<HandleWithCarePolicyRecord>, String> {
    conn.query_row(
        r#"
        SELECT version, text_en, text_vi, created_by_account_id, created_at, superseded_at
        FROM borrow_handle_with_care_policies
        WHERE superseded_at IS NULL
        ORDER BY version DESC
        LIMIT 1
        "#,
        [],
        map_handle_with_care_policy_row,
    )
    .optional()
    .map_err(|err| format!("failed to load current Handle with Care policy: {err}"))
}

pub(crate) fn get_handle_with_care_policy_conn(
    conn: &Connection,
    version: i64,
) -> Result<Option<HandleWithCarePolicyRecord>, String> {
    conn.query_row(
        r#"
        SELECT version, text_en, text_vi, created_by_account_id, created_at, superseded_at
        FROM borrow_handle_with_care_policies
        WHERE version = ?
        "#,
        params![version],
        map_handle_with_care_policy_row,
    )
    .optional()
    .map_err(|err| format!("failed to load Handle with Care policy version: {err}"))
}

pub(crate) fn save_handle_with_care_policy_conn(
    conn: &mut Connection,
    created_by_account_id: i64,
    text_en: &str,
    text_vi: &str,
) -> Result<HandleWithCarePolicyRecord, String> {
    let text_en = normalize_handle_with_care_policy_text(text_en, "English")?;
    let text_vi = normalize_handle_with_care_policy_text(text_vi, "Vietnamese")?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start Handle with Care policy transaction: {err}"))?;

    if let Some(current) = tx
        .query_row(
            r#"
            SELECT version, text_en, text_vi, created_by_account_id, created_at, superseded_at
            FROM borrow_handle_with_care_policies
            WHERE superseded_at IS NULL
            ORDER BY version DESC
            LIMIT 1
            "#,
            [],
            map_handle_with_care_policy_row,
        )
        .optional()
        .map_err(|err| format!("failed to inspect current Handle with Care policy: {err}"))?
    {
        if current.text_en == text_en && current.text_vi == text_vi {
            tx.commit()
                .map_err(|err| format!("failed to commit unchanged policy lookup: {err}"))?;
            return Ok(current);
        }
    }

    tx.execute(
        "UPDATE borrow_handle_with_care_policies SET superseded_at = datetime('now') WHERE superseded_at IS NULL",
        [],
    )
    .map_err(|err| format!("failed to supersede current Handle with Care policy: {err}"))?;
    tx.execute(
        "INSERT INTO borrow_handle_with_care_policies(text_en, text_vi, created_by_account_id) VALUES(?, ?, ?)",
        params![text_en, text_vi, created_by_account_id],
    )
    .map_err(|err| format!("failed to save Handle with Care policy: {err}"))?;
    let version = tx.last_insert_rowid();
    let policy = tx
        .query_row(
            "SELECT version, text_en, text_vi, created_by_account_id, created_at, superseded_at FROM borrow_handle_with_care_policies WHERE version = ?",
            params![version],
            map_handle_with_care_policy_row,
        )
        .map_err(|err| format!("failed to load saved Handle with Care policy: {err}"))?;
    tx.commit()
        .map_err(|err| format!("failed to commit Handle with Care policy: {err}"))?;
    Ok(policy)
}

fn map_handle_with_care_policy_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<HandleWithCarePolicyRecord> {
    Ok(HandleWithCarePolicyRecord {
        version: row.get(0)?,
        text_en: row.get(1)?,
        text_vi: row.get(2)?,
        created_by_account_id: row.get(3)?,
        created_at: row.get(4)?,
        superseded_at: row.get(5)?,
    })
}

fn normalize_handle_with_care_policy_text(value: &str, label: &str) -> Result<String, String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(format!("{label} Handle with Care policy text is required"));
    }
    if normalized.len() > HANDLE_WITH_CARE_POLICY_MAX_BYTES {
        return Err(format!(
            "{label} Handle with Care policy text is too long (max {HANDLE_WITH_CARE_POLICY_MAX_BYTES} bytes)"
        ));
    }
    Ok(normalized)
}

pub fn get_borrow_lan_settings(app: &AppHandle) -> Result<BorrowLanSettings, String> {
    let conn = open_runtime_connection(app)?;
    read_borrow_lan_settings(&conn)
}

/// The audit actor comes from the verified SessionContext.
pub fn update_borrow_lan_settings_with_actor(
    app: &AppHandle,
    ctx: crate::auth_session::SessionContext,
    payload: BorrowLanSettingsUpdateInput,
) -> Result<BorrowLanSettings, String> {
    let conn = open_runtime_connection(app)?;
    let host = require_text(payload.host, "host")?;
    if payload.port == 0 {
        return Err("port must be between 1 and 65535".to_string());
    }

    set_setting_value(&conn, BORROW_LAN_HOST_SETTING_KEY, Some(host.as_str()))?;
    let port_text = payload.port.to_string();
    set_setting_value(&conn, BORROW_LAN_PORT_SETTING_KEY, Some(port_text.as_str()))?;

    if let Some(enabled) = payload.enabled {
        set_setting_value(
            &conn,
            BORROW_LAN_ENABLED_SETTING_KEY,
            Some(if enabled { "1" } else { "0" }),
        )?;
    }

    let updated_settings = read_borrow_lan_settings(&conn)?;

    let payload_json = json!({
        "enabled": updated_settings.enabled,
        "host": updated_settings.host,
        "port": updated_settings.port,
    })
    .to_string();

    let actor_ref = ctx.account_id.to_string();
    audit::insert_audit_log_conn(
        &conn,
        "borrow_lan.update_settings",
        "local_account",
        Some(actor_ref.as_str()),
        "borrow_lan",
        "config",
        Some(payload_json.as_str()),
    )?;

    Ok(updated_settings)
}

pub fn detect_borrow_lan_host() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let adapters = ipconfig::get_adapters()
            .map_err(|_| "Could not enumerate active LAN interfaces.".to_string())?;
        let candidates = adapters
            .iter()
            .filter(|adapter| adapter.oper_status() == ipconfig::OperStatus::IfOperStatusUp)
            .filter(|adapter| {
                !matches!(
                    adapter.if_type(),
                    ipconfig::IfType::SoftwareLoopback | ipconfig::IfType::Tunnel
                )
            })
            .flat_map(|adapter| {
                let has_default_gateway = adapter.gateways().iter().any(|gateway| {
                    matches!(gateway, IpAddr::V4(address) if !address.is_unspecified())
                });
                let is_physical = matches!(
                    adapter.if_type(),
                    ipconfig::IfType::EthernetCsmacd
                        | ipconfig::IfType::Ieee80211
                        | ipconfig::IfType::Ieee1394
                );
                adapter.ip_addresses().iter().filter_map(move |address| {
                    let IpAddr::V4(host) = address else {
                        return None;
                    };
                    if !is_valid_private_borrow_lan_host(*host) {
                        return None;
                    }
                    Some(BorrowLanInterfaceCandidate {
                        host: *host,
                        has_default_gateway,
                        is_physical,
                        route_metric: adapter.ipv4_metric(),
                    })
                })
            })
            .collect::<Vec<_>>();

        return Ok(select_borrow_lan_host_candidate(&candidates));
    }

    #[cfg(not(target_os = "windows"))]
    {
        for target in BORROW_LAN_DETECTION_TARGETS {
            if let Some(host) = detect_borrow_lan_host_via_target(target) {
                return Ok(Some(host));
            }
        }

        Ok(None)
    }
}

pub fn get_borrow_policy(app: &AppHandle) -> Result<Option<BorrowPolicyDesktopRecord>, String> {
    let conn = open_runtime_connection(app)?;
    get_current_handle_with_care_policy_conn(&conn).map(|policy| {
        policy.map(|record| BorrowPolicyDesktopRecord {
            version: record.version,
            text_en: record.text_en,
            text_vi: record.text_vi,
        })
    })
}

pub fn save_borrow_policy(
    app: &AppHandle,
    account_id: i64,
    text_en: &str,
    text_vi: &str,
) -> Result<BorrowPolicyDesktopRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    save_handle_with_care_policy_conn(&mut conn, account_id, text_en, text_vi).map(|record| {
        BorrowPolicyDesktopRecord {
            version: record.version,
            text_en: record.text_en,
            text_vi: record.text_vi,
        }
    })
}

pub fn get_borrow_request_evidence(
    app: &AppHandle,
    request_id: i64,
) -> Result<Option<BorrowRequestEvidenceRecord>, String> {
    let conn = open_runtime_connection(app)?;
    load_borrow_request_confirmation_conn(&conn, request_id).map(|record| {
        record.map(|evidence| BorrowRequestEvidenceRecord {
            borrow_request_id: evidence.borrow_request_id,
            policy_version: evidence.policy_version,
            policy_text_en_snapshot: evidence.policy_text_en_snapshot,
            policy_text_vi_snapshot: evidence.policy_text_vi_snapshot,
            policy_acknowledged: evidence.policy_acknowledged,
            confirmation_method: confirmation_method_text(evidence.confirmation_method).to_string(),
            typed_name: evidence.typed_name,
            has_signature: evidence.signature_png_blob.is_some(),
            confirmed_at: evidence.confirmed_at,
            signature_png_base64: evidence.signature_png_blob.map(|blob| STANDARD.encode(blob)),
        })
    })
}

pub fn list_pending_borrow_requests(app: &AppHandle) -> Result<Vec<BorrowRequestRecord>, String> {
    let conn = open_runtime_connection(app)?;
    list_pending_borrow_requests_conn(&conn)
}

pub fn get_borrow_request_detail(
    app: &AppHandle,
    request_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let conn = open_runtime_connection(app)?;
    load_borrow_request_detail_conn(&conn, request_id)
}

/// The reviewer identity comes from the verified SessionContext (passed by the
/// lib.rs command wrapper), not from the active-account DB row.
pub fn approve_borrow_request_with_actor(
    app: &AppHandle,
    ctx: crate::auth_session::SessionContext,
    request_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    approve_borrow_request_conn(&mut conn, request_id, ctx.account_id)
}

/// Phase C variant: reviewer identity from SessionContext.
pub fn reject_borrow_request_with_actor(
    app: &AppHandle,
    ctx: crate::auth_session::SessionContext,
    payload: BorrowRequestRejectInput,
) -> Result<BorrowRequestRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    reject_borrow_request_conn(&mut conn, payload.request_id, ctx.account_id, payload.note)
}

/// Cancel a pending request and release all of its asset claims atomically.
pub(crate) fn cancel_borrow_request_conn(
    conn: &mut Connection,
    request_id: i64,
    reviewer_account_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start borrow cancellation transaction: {err}"))?;
    let (_, status, _, _) = load_request_state_tx(&tx, request_id)?;
    ensure_pending_status(status.as_str())?;
    tx.execute(
        "UPDATE borrow_requests SET status = 'cancelled', decided_by_account_id = ?, decided_at = datetime('now') WHERE id = ?",
        params![reviewer_account_id, request_id],
    )
    .map_err(humanize_sqlite_error)?;
    let reviewer_account_text = reviewer_account_id.to_string();
    let request_id_text = request_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.cancel",
        "local_account",
        Some(reviewer_account_text.as_str()),
        "borrow_request",
        request_id_text.as_str(),
        None,
    )?;
    tx.execute(
        "DELETE FROM asset_pending_claims WHERE borrow_request_id = ?",
        params![request_id],
    )
    .map_err(humanize_sqlite_error)?;
    tx.commit()
        .map_err(|err| format!("failed to commit borrow cancellation: {err}"))?;
    load_borrow_request_record(conn, request_id)
}

pub fn cancel_borrow_request_with_actor(
    app: &AppHandle,
    ctx: crate::auth_session::SessionContext,
    request_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    cancel_borrow_request_conn(&mut conn, request_id, ctx.account_id)
}

struct ValidatedBorrowSubmission {
    submitted_employee_id: String,
    authoritative_full_name: String,
    asset_codes: Vec<String>,
    employee_id_fk: Option<i64>,
    manual_entry: i64,
    manual_employee_id: Option<String>,
    manual_employee_name: Option<String>,
    manual_employee_email: Option<String>,
    manual_employee_team: Option<String>,
    request_type: String,
    assets: Vec<asset::AssetLookupRecord>,
    borrower_identity: BorrowerIdentitySnapshot,
    submitted_by_identity: SubmittedByIdentitySnapshot,
}

fn validate_submission_tx(
    tx: &Transaction<'_>,
    input: BorrowRequestSubmitInput,
) -> Result<ValidatedBorrowSubmission, String> {
    let submitted_employee_id =
        require_text(input.submitted_employee_id, "submittedEmployeeId")?.to_uppercase();
    let submitted_full_name = input.submitted_full_name.trim().to_string();
    if submitted_full_name.len() > 200 {
        return Err("submittedFullName is too long (max 200)".to_string());
    }
    let asset_codes = normalize_asset_codes(input.asset_codes)?;
    let request_type = input
        .request_type
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(REQUEST_TYPE_BORROW)
        .to_string();
    if request_type != REQUEST_TYPE_BORROW && request_type != REQUEST_TYPE_RETURN {
        return Err(format!(
            "invalid request_type '{}', must be 'borrow' or 'return'",
            request_type
        ));
    }

    let employee = load_employee_identity_by_business_id_tx(&tx, submitted_employee_id.as_str())?;
    let manual_employee_email = input
        .manual_employee_email
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if manual_employee_email
        .as_ref()
        .is_some_and(|value| value.len() > 320)
    {
        return Err("manualEmployeeEmail is too long (max 320)".to_string());
    }
    let manual_employee_team = input
        .manual_employee_team
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if manual_employee_team
        .as_ref()
        .is_some_and(|value| value.len() > 200)
    {
        return Err("manualEmployeeTeam is too long (max 200)".to_string());
    }
    let (
        employee_id_fk,
        authoritative_full_name,
        manual_entry,
        manual_employee_id,
        manual_employee_name,
        manual_employee_email,
        manual_employee_team,
    ) = if let Some(employee) = employee {
        if request_type == REQUEST_TYPE_BORROW
            && employee
                .staff_group
                .eq_ignore_ascii_case(STAFF_GROUP_OFFBOARDING)
        {
            return Err("offboarding employees cannot borrow assets".to_string());
        }
        (
            Some(employee.id),
            employee.full_name,
            0_i64,
            None,
            None,
            None,
            None,
        )
    } else {
        if submitted_full_name.is_empty() {
            return Err("submittedFullName is required for manual entry".to_string());
        }
        (
            None,
            submitted_full_name.clone(),
            1_i64,
            Some(submitted_employee_id.clone()),
            Some(submitted_full_name.clone()),
            manual_employee_email,
            manual_employee_team,
        )
    };

    let mut assets = Vec::new();
    for asset_code in &asset_codes {
        let asset_record = asset::load_asset_by_code_tx(&tx, asset_code.as_str())?
            .ok_or_else(|| format!("asset '{}' was not found", asset_code))?;
        if request_type == REQUEST_TYPE_BORROW && asset_record.status != ASSET_STATUS_IN_STOCK {
            return Err(format!(
                "asset '{}' is not in_stock",
                asset_record.asset_code
            ));
        }
        if matches!(
            asset_record.status.as_str(),
            ASSET_STATUS_RETIRED | ASSET_STATUS_DISPOSED | ASSET_STATUS_LOST
        ) {
            return Err(format!(
                "asset '{}' is not eligible for LAN borrowing",
                asset_record.asset_code
            ));
        }
        let active_loan = active_loan_employee_id_tx(&tx, asset_record.id)?;
        if request_type == REQUEST_TYPE_RETURN {
            if asset_record.status != ASSET_STATUS_ASSIGNED || active_loan.is_none() {
                return Err(format!(
                    "asset '{}' is not currently assigned",
                    asset_record.asset_code
                ));
            }
        } else {
            if active_loan.is_some() {
                return Err(format!(
                    "asset '{}' already has an active loan",
                    asset_record.asset_code
                ));
            }
            let has_claim: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM asset_pending_claims WHERE asset_id = ?)",
                    params![asset_record.id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|value| value > 0)
                .map_err(|err| format!("failed to check pending asset claim: {err}"))?;
            if has_claim {
                return Err(format!(
                    "asset '{}' has a competing pending borrow claim",
                    asset_record.asset_code
                ));
            }
        }
        assets.push(asset_record);
    }

    let submitted_by_identity = SubmittedByIdentitySnapshot {
        employee_id_fk,
        staff_id: submitted_employee_id.clone(),
        full_name: authoritative_full_name.clone(),
    };
    let borrower_identity = if request_type == REQUEST_TYPE_BORROW {
        BorrowerIdentitySnapshot {
            employee_id_fk,
            staff_id: submitted_employee_id.clone(),
            full_name: authoritative_full_name.clone(),
        }
    } else {
        let mut borrower_id: Option<i64> = None;
        for asset_record in &assets {
            let current_borrower_id = active_loan_employee_id_tx(&tx, asset_record.id)?
                .ok_or_else(|| {
                    format!("asset '{}' has no active borrower", asset_record.asset_code)
                })?;
            if borrower_id.is_some_and(|id| id != current_borrower_id) {
                return Err("all returned assets must belong to the same borrower".to_string());
            }
            borrower_id = Some(current_borrower_id);
        }
        let borrower_id =
            borrower_id.ok_or_else(|| "at least one asset is required".to_string())?;
        let borrower = load_employee_identity_by_id_tx(&tx, borrower_id)?
            .ok_or_else(|| "the active asset borrower could not be resolved".to_string())?;
        BorrowerIdentitySnapshot {
            employee_id_fk: Some(borrower.id),
            staff_id: borrower.employee_id,
            full_name: borrower.full_name,
        }
    };

    Ok(ValidatedBorrowSubmission {
        submitted_employee_id,
        authoritative_full_name,
        asset_codes,
        employee_id_fk,
        manual_entry,
        manual_employee_id,
        manual_employee_name,
        manual_employee_email,
        manual_employee_team,
        request_type,
        assets,
        borrower_identity,
        submitted_by_identity,
    })
}

struct ValidatedConfirmation {
    policy_version: Option<i64>,
    policy_text_en_snapshot: Option<String>,
    policy_text_vi_snapshot: Option<String>,
    policy_acknowledged: bool,
    asset_codes_snapshot_json: String,
    confirmation_method: ConfirmationMethod,
    signature_png_blob: Option<Vec<u8>>,
    typed_name: Option<String>,
}

fn validate_confirmation_tx(
    tx: &Transaction<'_>,
    validated: &ValidatedBorrowSubmission,
    input: BorrowRequestConfirmationInput,
) -> Result<ValidatedConfirmation, String> {
    let final_asset_codes: Vec<String> = validated
        .assets
        .iter()
        .map(|record| record.asset_code.clone())
        .collect();
    let snapshot_codes = normalize_asset_codes(input.asset_codes_snapshot)?;
    if snapshot_codes != final_asset_codes {
        return Err("confirmation asset list does not match the selected assets".to_string());
    }

    let (policy_text_en_snapshot, policy_text_vi_snapshot) =
        if validated.request_type == REQUEST_TYPE_BORROW {
            let policy_version = input
                .policy_version
                .ok_or_else(|| "Handle with Care policy acknowledgment is required".to_string())?;
            if !input.policy_acknowledged {
                return Err("Handle with Care policy acknowledgment is required".to_string());
            }
            let current_policy = get_current_handle_with_care_policy_conn(tx)?
                .ok_or_else(|| "Handle with Care policy is not configured".to_string())?;
            if current_policy.version != policy_version {
                return Err(
                    "Handle with Care policy changed; reload the current policy and try again"
                        .to_string(),
                );
            }
            (Some(current_policy.text_en), Some(current_policy.text_vi))
        } else {
            if input.policy_version.is_some() || input.policy_acknowledged {
                return Err(
                    "Return requests do not require Handle with Care acknowledgment".to_string(),
                );
            }
            (None, None)
        };

    let typed_name = input
        .typed_name
        .map(|value| normalize_typed_name(value.as_str()))
        .transpose()?;
    let signature_png_blob = match input.signature_png_base64 {
        Some(value) => Some(validate_signature_png(
            value.as_str(),
            input.signature_stroke_count,
            input.signature_ink_present,
        )?),
        None => {
            if input.signature_stroke_count.is_some() || input.signature_ink_present.is_some() {
                return Err("signature metadata was supplied without a signature".to_string());
            }
            None
        }
    };

    match input.confirmation_method {
        ConfirmationMethod::Signature if signature_png_blob.is_none() => {
            return Err("a handwritten signature is required".to_string())
        }
        ConfirmationMethod::Signature if typed_name.is_some() => {
            return Err("signature confirmation cannot include a typed name".to_string())
        }
        ConfirmationMethod::TypedName if typed_name.is_none() => {
            return Err("a typed full name is required".to_string())
        }
        ConfirmationMethod::TypedName if signature_png_blob.is_some() => {
            return Err("typed-name confirmation cannot include a signature".to_string())
        }
        ConfirmationMethod::Both if typed_name.is_none() || signature_png_blob.is_none() => {
            return Err("both a handwritten signature and typed full name are required".to_string())
        }
        _ => {}
    }

    if let Some(typed_name) = typed_name.as_deref() {
        if !typed_names_match(
            typed_name,
            validated.submitted_by_identity.full_name.as_str(),
        ) {
            return Err("typed full name must match the submitting employee's name".to_string());
        }
    }

    Ok(ValidatedConfirmation {
        policy_version: input.policy_version,
        policy_text_en_snapshot,
        policy_text_vi_snapshot,
        policy_acknowledged: input.policy_acknowledged,
        asset_codes_snapshot_json: serde_json::to_string(&final_asset_codes)
            .map_err(|_| "failed to snapshot confirmation asset list".to_string())?,
        confirmation_method: input.confirmation_method,
        signature_png_blob,
        typed_name,
    })
}

pub(crate) fn normalize_typed_name(value: &str) -> Result<String, String> {
    let normalized = value
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if normalized.is_empty() {
        return Err("typed full name is required".to_string());
    }
    if normalized.len() > 200 {
        return Err("typed full name is too long (max 200)".to_string());
    }
    Ok(normalized)
}

pub(crate) fn typed_names_match(expected: &str, supplied: &str) -> bool {
    match (
        normalize_typed_name(expected),
        normalize_typed_name(supplied),
    ) {
        (Ok(expected), Ok(supplied)) => expected == supplied,
        _ => false,
    }
}

pub(crate) fn validate_signature_png(
    data_url: &str,
    stroke_count: Option<u32>,
    ink_present_hint: Option<bool>,
) -> Result<Vec<u8>, String> {
    const MAX_SIGNATURE_BYTES: usize = 256 * 1024;
    const MAX_SIGNATURE_WIDTH: u32 = 1200;
    const MAX_SIGNATURE_HEIGHT: u32 = 600;
    let encoded = data_url
        .strip_prefix("data:image/png;base64,")
        .ok_or_else(|| "signature must be a canvas-generated PNG".to_string())?;
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| "signature PNG data is invalid".to_string())?;
    if decoded.is_empty() || decoded.len() > MAX_SIGNATURE_BYTES {
        return Err("signature PNG is empty or too large".to_string());
    }
    let decoder = Decoder::new(Cursor::new(decoded.as_slice()));
    let mut reader = decoder
        .read_info()
        .map_err(|_| "signature must be a valid PNG".to_string())?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > MAX_SIGNATURE_WIDTH
        || info.height > MAX_SIGNATURE_HEIGHT
    {
        return Err("signature PNG dimensions are not allowed".to_string());
    }
    let output_size = reader.output_buffer_size();
    let mut pixels = vec![0_u8; output_size];
    let output = reader
        .next_frame(&mut pixels)
        .map_err(|_| "signature must be a valid PNG".to_string())?;
    let bytes_per_pixel = output
        .buffer_size()
        .checked_div((output.width * output.height) as usize)
        .ok_or_else(|| "signature PNG has no pixel data".to_string())?;
    let mut ink_pixels = 0_usize;
    for pixel in pixels[..output.buffer_size()].chunks(bytes_per_pixel) {
        let alpha = if bytes_per_pixel >= 4 { pixel[3] } else { 255 };
        let dark = pixel.iter().take(3).any(|channel| *channel < 245);
        if alpha > 16 && dark {
            ink_pixels += 1;
        }
    }
    if ink_pixels < 2 {
        return Err("signature must contain meaningful ink".to_string());
    }
    if stroke_count == Some(0) || ink_present_hint == Some(false) {
        return Err("signature metadata does not match the signature image".to_string());
    }
    if ink_present_hint == Some(true) && ink_pixels == 0 {
        return Err("signature must contain meaningful ink".to_string());
    }
    Ok(decoded)
}

pub(crate) fn validate_borrow_request_conn(
    conn: &mut Connection,
    input: BorrowRequestSubmitInput,
) -> Result<(), String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start borrow validation transaction: {err}"))?;
    let _ = validate_submission_tx(&tx, input)?;
    Ok(())
}

pub(crate) fn validate_borrow_request_with_confirmation_conn(
    conn: &mut Connection,
    input: BorrowRequestWithConfirmationInput,
) -> Result<(), String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start confirmed borrow validation transaction: {err}"))?;
    let validated = validate_submission_tx(&tx, input.request)?;
    validate_confirmation_tx(
        &tx,
        &validated,
        input
            .confirmation
            .ok_or_else(|| "borrow or return confirmation is required".to_string())?,
    )?;
    Ok(())
}

pub(crate) fn submit_borrow_request_conn(
    conn: &mut Connection,
    input: BorrowRequestSubmitInput,
) -> Result<BorrowRequestRecord, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start borrow submit transaction: {err}"))?;
    let validated = validate_submission_tx(&tx, input)?;
    let ValidatedBorrowSubmission {
        submitted_employee_id,
        authoritative_full_name,
        asset_codes,
        employee_id_fk,
        manual_entry,
        manual_employee_id,
        manual_employee_name,
        manual_employee_email,
        manual_employee_team,
        request_type,
        assets,
        borrower_identity,
        submitted_by_identity,
    } = validated;

    let request_key = generate_request_key_tx(&tx)?;
    tx.execute(
        r#"
        INSERT INTO borrow_requests(
          request_key,
          employee_id_fk,
          submitted_employee_id,
          submitted_full_name,
          manual_entry,
          manual_employee_id,
          manual_employee_name,
          manual_employee_email,
          manual_employee_team,
          status,
          request_type,
          returned_by_employee_id_fk,
          borrower_employee_id_fk,
          borrower_staff_id_snapshot,
          borrower_name_snapshot,
          submitted_by_employee_id_fk,
          submitted_by_staff_id_snapshot,
          submitted_by_name_snapshot,
          submitted_at
        )
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            request_key.as_str(),
            employee_id_fk,
            submitted_employee_id.as_str(),
            authoritative_full_name.as_str(),
            manual_entry,
            manual_employee_id.as_deref(),
            manual_employee_name.as_deref(),
            manual_employee_email.as_deref(),
            manual_employee_team.as_deref(),
            REQUEST_STATUS_PENDING,
            request_type,
            if request_type == REQUEST_TYPE_RETURN {
                Some(employee_id_fk)
            } else {
                None
            },
            borrower_identity.employee_id_fk,
            borrower_identity.staff_id.as_str(),
            borrower_identity.full_name.as_str(),
            submitted_by_identity.employee_id_fk,
            submitted_by_identity.staff_id.as_str(),
            submitted_by_identity.full_name.as_str(),
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let request_id = tx.last_insert_rowid();

    for asset_record in &assets {
        tx.execute(
            r#"
            INSERT INTO borrow_request_items(
              borrow_request_id,
              asset_id,
              asset_code_snapshot,
              created_at
            )
            VALUES(?, ?, ?, datetime('now'))
            "#,
            params![
                request_id,
                asset_record.id,
                asset_record.asset_code.as_str()
            ],
        )
        .map_err(humanize_sqlite_error)?;

        if request_type == REQUEST_TYPE_BORROW {
            tx.execute(
                "INSERT INTO asset_pending_claims(asset_id, borrow_request_id, created_at) VALUES(?, ?, datetime('now'))",
                params![asset_record.id, request_id],
            )
            .map_err(|err| {
                if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                    "asset has a competing pending borrow claim".to_string()
                } else {
                    format!("failed to create pending asset claim: {err}")
                }
            })?;
        }
    }

    let audit_payload = json!({
        "submittedEmployeeId": submitted_employee_id,
        "submittedFullName": authoritative_full_name,
        "requestType": request_type,
        "assetCodes": asset_codes,
    })
    .to_string();

    let request_id_text = request_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.submit",
        "lan_public",
        None,
        "borrow_request",
        request_id_text.as_str(),
        Some(audit_payload.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit borrow submit transaction: {err}"))?;

    load_borrow_request_record(conn, request_id)
}

pub(crate) fn submit_borrow_request_with_confirmation_conn(
    conn: &mut Connection,
    input: BorrowRequestWithConfirmationInput,
) -> Result<BorrowRequestRecord, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start confirmed borrow submit transaction: {err}"))?;
    let validated = validate_submission_tx(&tx, input.request)?;
    let confirmation = validate_confirmation_tx(
        &tx,
        &validated,
        input
            .confirmation
            .ok_or_else(|| "borrow or return confirmation is required".to_string())?,
    )?;
    let request_key = generate_request_key_tx(&tx)?;
    let request_type = validated.request_type.as_str();

    tx.execute(
        r#"
        INSERT INTO borrow_requests(
          request_key, employee_id_fk, submitted_employee_id, submitted_full_name,
          manual_entry, manual_employee_id, manual_employee_name, manual_employee_email,
          manual_employee_team, status, request_type, returned_by_employee_id_fk,
          borrower_employee_id_fk, borrower_staff_id_snapshot, borrower_name_snapshot,
          submitted_by_employee_id_fk, submitted_by_staff_id_snapshot, submitted_by_name_snapshot,
          submitted_at
        )
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            request_key.as_str(),
            validated.employee_id_fk,
            validated.submitted_employee_id.as_str(),
            validated.authoritative_full_name.as_str(),
            validated.manual_entry,
            validated.manual_employee_id.as_deref(),
            validated.manual_employee_name.as_deref(),
            validated.manual_employee_email.as_deref(),
            validated.manual_employee_team.as_deref(),
            REQUEST_STATUS_PENDING,
            request_type,
            if request_type == REQUEST_TYPE_RETURN {
                validated.submitted_by_identity.employee_id_fk
            } else {
                None
            },
            validated.borrower_identity.employee_id_fk,
            validated.borrower_identity.staff_id.as_str(),
            validated.borrower_identity.full_name.as_str(),
            validated.submitted_by_identity.employee_id_fk,
            validated.submitted_by_identity.staff_id.as_str(),
            validated.submitted_by_identity.full_name.as_str(),
        ],
    )
    .map_err(humanize_sqlite_error)?;
    let request_id = tx.last_insert_rowid();

    for asset_record in &validated.assets {
        tx.execute(
            "INSERT INTO borrow_request_items(borrow_request_id, asset_id, asset_code_snapshot, created_at) VALUES(?, ?, ?, datetime('now'))",
            params![request_id, asset_record.id, asset_record.asset_code.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
        if request_type == REQUEST_TYPE_BORROW {
            tx.execute(
                "INSERT INTO asset_pending_claims(asset_id, borrow_request_id, created_at) VALUES(?, ?, datetime('now'))",
                params![asset_record.id, request_id],
            )
            .map_err(|err| {
                if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                    "asset has a competing pending borrow claim".to_string()
                } else {
                    format!("failed to create pending asset claim: {err}")
                }
            })?;
        }
    }

    let method = confirmation_method_text(confirmation.confirmation_method);
    tx.execute(
        r#"
        INSERT INTO borrow_request_confirmations(
          borrow_request_id, policy_version, policy_text_en_snapshot, policy_text_vi_snapshot,
          policy_acknowledged, asset_codes_snapshot_json, confirmation_method,
          signature_png_blob, typed_name, confirmed_at
        ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            request_id,
            confirmation.policy_version,
            confirmation.policy_text_en_snapshot.as_deref(),
            confirmation.policy_text_vi_snapshot.as_deref(),
            confirmation.policy_acknowledged,
            confirmation.asset_codes_snapshot_json,
            method,
            confirmation.signature_png_blob,
            confirmation.typed_name.as_deref(),
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let audit_payload = json!({
        "submittedEmployeeId": validated.submitted_employee_id,
        "requestType": request_type,
        "assetCodes": validated.assets.iter().map(|asset| asset.asset_code.clone()).collect::<Vec<_>>(),
        "confirmationMethod": method,
        "hasSignature": confirmation.signature_png_blob.is_some(),
        "hasTypedName": confirmation.typed_name.is_some(),
        "policyVersion": confirmation.policy_version,
    })
    .to_string();
    let request_id_text = request_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.submit",
        "lan_public",
        None,
        "borrow_request",
        request_id_text.as_str(),
        Some(audit_payload.as_str()),
    )?;
    tx.commit()
        .map_err(|err| format!("failed to commit confirmed borrow submit transaction: {err}"))?;
    load_borrow_request_record(conn, request_id)
}

fn confirmation_method_text(method: ConfirmationMethod) -> &'static str {
    match method {
        ConfirmationMethod::Signature => "signature",
        ConfirmationMethod::TypedName => "typed_name",
        ConfirmationMethod::Both => "both",
    }
}

pub(crate) fn approve_borrow_request_conn(
    conn: &mut Connection,
    request_id: i64,
    reviewer_account_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start borrow approval transaction: {err}"))?;

    let (employee_id_fk, status, request_type, returned_by_employee_id_fk) =
        load_request_state_tx(&tx, request_id)?;
    ensure_pending_status(status.as_str())?;

    let request_items = load_request_items_tx(&tx, request_id)?;
    if request_items.is_empty() {
        return Err("borrow request has no asset items".to_string());
    }

    // Validate asset statuses match the request type
    for item in &request_items {
        let asset_record = asset::load_asset_by_id_tx(&tx, item.asset_id)?
            .ok_or_else(|| format!("asset with id {} was not found", item.asset_id))?;
        if request_type == REQUEST_TYPE_RETURN {
            if asset_record.status != ASSET_STATUS_ASSIGNED {
                return Err(format!(
                    "asset '{}' is not assigned (cannot approve return)",
                    asset_record.asset_code
                ));
            }
        } else if asset_record.status != ASSET_STATUS_IN_STOCK {
            return Err(format!(
                "asset '{}' is not in_stock",
                asset_record.asset_code
            ));
        }
    }

    // Apply the status transition
    for item in &request_items {
        if request_type == REQUEST_TYPE_RETURN {
            asset::set_asset_status_tx(&tx, item.asset_id, ASSET_STATUS_IN_STOCK)?;
            tx.execute(
                "UPDATE asset_loans SET returned_at = datetime('now'), returned_by_employee_id_fk = ? WHERE asset_id = ? AND returned_at IS NULL",
                params![returned_by_employee_id_fk, item.asset_id],
            )
            .map_err(humanize_sqlite_error)?;
        } else {
            asset::set_asset_status_tx(&tx, item.asset_id, ASSET_STATUS_ASSIGNED)?;
            tx.execute(
                r#"
                INSERT INTO asset_loans(
                  asset_id,
                  employee_id_fk,
                  borrow_request_id,
                  approved_by_account_id,
                  borrowed_at
                )
                VALUES(?, ?, ?, ?, datetime('now'))
                "#,
                params![
                    item.asset_id,
                    employee_id_fk,
                    request_id,
                    reviewer_account_id
                ],
            )
            .map_err(humanize_sqlite_error)?;
        }
    }

    if request_type == REQUEST_TYPE_BORROW {
        tx.execute(
            "DELETE FROM asset_pending_claims WHERE borrow_request_id = ?",
            params![request_id],
        )
        .map_err(humanize_sqlite_error)?;
    }

    tx.execute(
        r#"
        UPDATE borrow_requests
        SET
          status = ?,
          decision_note = NULL,
          decided_by_account_id = ?,
          decided_at = datetime('now')
        WHERE id = ?
        "#,
        params![REQUEST_STATUS_APPROVED, reviewer_account_id, request_id],
    )
    .map_err(humanize_sqlite_error)?;

    let audit_payload = json!({
        "reviewerAccountId": reviewer_account_id,
        "requestType": request_type,
        "assetCodes": request_items
            .iter()
            .map(|item| item.asset_code_snapshot.clone())
            .collect::<Vec<_>>(),
    })
    .to_string();

    let reviewer_account_text = reviewer_account_id.to_string();
    let request_id_text = request_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.approve",
        "local_account",
        Some(reviewer_account_text.as_str()),
        "borrow_request",
        request_id_text.as_str(),
        Some(audit_payload.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit borrow approval transaction: {err}"))?;

    load_borrow_request_record(conn, request_id)
}

pub(crate) fn reject_borrow_request_conn(
    conn: &mut Connection,
    request_id: i64,
    reviewer_account_id: i64,
    note: String,
) -> Result<BorrowRequestRecord, String> {
    let decision_note = require_text(note, "decisionNote")?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("failed to start borrow reject transaction: {err}"))?;

    let (_, status, _, _) = load_request_state_tx(&tx, request_id)?;
    ensure_pending_status(status.as_str())?;

    tx.execute(
        r#"
        UPDATE borrow_requests
        SET
          status = ?,
          decision_note = ?,
          decided_by_account_id = ?,
          decided_at = datetime('now')
        WHERE id = ?
        "#,
        params![
            REQUEST_STATUS_REJECTED,
            decision_note.as_str(),
            reviewer_account_id,
            request_id
        ],
    )
    .map_err(humanize_sqlite_error)?;

    tx.execute(
        "DELETE FROM asset_pending_claims WHERE borrow_request_id = ?",
        params![request_id],
    )
    .map_err(humanize_sqlite_error)?;

    let audit_payload = json!({
        "reviewerAccountId": reviewer_account_id,
        "decisionNote": decision_note,
    })
    .to_string();

    let reviewer_account_text = reviewer_account_id.to_string();
    let request_id_text = request_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.reject",
        "local_account",
        Some(reviewer_account_text.as_str()),
        "borrow_request",
        request_id_text.as_str(),
        Some(audit_payload.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit borrow reject transaction: {err}"))?;

    load_borrow_request_record(conn, request_id)
}

pub(crate) fn list_pending_borrow_requests_conn(
    conn: &Connection,
) -> Result<Vec<BorrowRequestRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id FROM borrow_requests WHERE status = ? ORDER BY submitted_at DESC, id DESC",
        )
        .map_err(|err| format!("failed to prepare pending borrow request query: {err}"))?;

    let rows = stmt
        .query_map(params![REQUEST_STATUS_PENDING], |row| row.get::<_, i64>(0))
        .map_err(|err| format!("failed to query pending borrow requests: {err}"))?;

    let mut records = Vec::new();
    for row in rows {
        let request_id =
            row.map_err(|err| format!("failed to read pending borrow request row: {err}"))?;
        records.push(load_borrow_request_record(conn, request_id)?);
    }

    Ok(records)
}

pub(crate) fn load_borrow_request_detail_conn(
    conn: &Connection,
    request_id: i64,
) -> Result<BorrowRequestRecord, String> {
    load_borrow_request_record(conn, request_id)
}

pub(crate) fn load_borrow_request_confirmation_conn(
    conn: &Connection,
    request_id: i64,
) -> Result<Option<BorrowRequestConfirmationRecord>, String> {
    conn.query_row(
        r#"
        SELECT borrow_request_id, policy_version, policy_text_en_snapshot,
          policy_text_vi_snapshot, policy_acknowledged, asset_codes_snapshot_json,
          confirmation_method, signature_png_blob, typed_name, confirmed_at
        FROM borrow_request_confirmations
        WHERE borrow_request_id = ?
        "#,
        params![request_id],
        |row| {
            let method: String = row.get(6)?;
            let confirmation_method = match method.as_str() {
                "signature" => ConfirmationMethod::Signature,
                "typed_name" => ConfirmationMethod::TypedName,
                "both" => ConfirmationMethod::Both,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        6,
                        "confirmation_method".to_string(),
                        rusqlite::types::Type::Text,
                    ))
                }
            };
            Ok(BorrowRequestConfirmationRecord {
                borrow_request_id: row.get(0)?,
                policy_version: row.get(1)?,
                policy_text_en_snapshot: row.get(2)?,
                policy_text_vi_snapshot: row.get(3)?,
                policy_acknowledged: row.get::<_, i64>(4)? != 0,
                asset_codes_snapshot_json: row.get(5)?,
                confirmation_method,
                signature_png_blob: row.get(7)?,
                typed_name: row.get(8)?,
                confirmed_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load borrow request confirmation: {err}"))
}

fn normalize_asset_codes(asset_codes: Vec<String>) -> Result<Vec<String>, String> {
    if asset_codes.is_empty() {
        return Err("at least one asset code is required".to_string());
    }

    let mut normalized_codes = Vec::new();
    let mut seen_codes = HashSet::new();

    for raw_code in asset_codes {
        let normalized_code = require_text(raw_code, "assetCode")?.to_uppercase();
        if !seen_codes.insert(normalized_code.clone()) {
            return Err("duplicate asset code in request".to_string());
        }
        normalized_codes.push(normalized_code);
    }

    Ok(normalized_codes)
}

fn read_borrow_lan_settings(conn: &Connection) -> Result<BorrowLanSettings, String> {
    let enabled = get_setting_value(conn, BORROW_LAN_ENABLED_SETTING_KEY)?
        .as_deref()
        .map(parse_borrow_lan_enabled_setting)
        .unwrap_or(false);
    let host = get_setting_value(conn, BORROW_LAN_HOST_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(default_borrow_lan_host);
    let port = get_setting_value(conn, BORROW_LAN_PORT_SETTING_KEY)?
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_BORROW_LAN_PORT);

    Ok(BorrowLanSettings {
        enabled,
        borrow_url: build_borrow_lan_url(host.as_str(), port),
        host,
        port,
    })
}

fn parse_borrow_lan_enabled_setting(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(any(not(target_os = "windows"), test))]
fn normalize_detected_borrow_lan_host_candidate(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    let parsed = trimmed.parse::<IpAddr>().ok()?;
    if parsed.is_loopback() || parsed.is_unspecified() {
        return None;
    }
    Some(parsed.to_string())
}

fn is_valid_private_borrow_lan_host(host: Ipv4Addr) -> bool {
    !host.is_loopback() && !host.is_unspecified() && !host.is_link_local() && host.is_private()
}

fn select_borrow_lan_host_candidate(candidates: &[BorrowLanInterfaceCandidate]) -> Option<String> {
    candidates
        .iter()
        .filter(|candidate| is_valid_private_borrow_lan_host(candidate.host))
        .min_by_key(|candidate| {
            (
                !candidate.has_default_gateway,
                !candidate.is_physical,
                candidate.route_metric,
                candidate.host.octets(),
            )
        })
        .map(|candidate| candidate.host.to_string())
}

#[cfg(not(target_os = "windows"))]
fn detect_borrow_lan_host_via_target(target: &str) -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(target).ok()?;
    let local_ip = socket.local_addr().ok()?.ip().to_string();
    normalize_detected_borrow_lan_host_candidate(local_ip.as_str())
}

fn default_borrow_lan_host() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn build_borrow_lan_url(host: &str, port: u16) -> String {
    let formatted_host = format_borrow_lan_url_host(host);
    format!("http://{formatted_host}:{port}/borrow")
}

fn format_borrow_lan_url_host(host: &str) -> String {
    let trimmed = host.trim();

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return trimmed.to_string();
    }

    match trimmed.parse::<IpAddr>() {
        Ok(IpAddr::V6(_)) => format!("[{trimmed}]"),
        _ => trimmed.to_string(),
    }
}

struct EmployeeIdentity {
    id: i64,
    employee_id: String,
    full_name: String,
    staff_group: String,
}

fn load_employee_identity_by_business_id_tx(
    tx: &Transaction<'_>,
    submitted_employee_id: &str,
) -> Result<Option<EmployeeIdentity>, String> {
    tx.query_row(
        "SELECT id, employee_id, full_name, staff_group FROM employees WHERE employee_id = ? COLLATE NOCASE",
        params![submitted_employee_id],
        |row| {
            Ok(EmployeeIdentity {
                id: row.get(0)?,
                employee_id: row.get(1)?,
                full_name: row.get(2)?,
                staff_group: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load employee identity: {err}"))
}

fn load_employee_identity_by_id_tx(
    tx: &Transaction<'_>,
    employee_id: i64,
) -> Result<Option<EmployeeIdentity>, String> {
    tx.query_row(
        "SELECT id, employee_id, full_name, staff_group FROM employees WHERE id = ?",
        params![employee_id],
        |row| {
            Ok(EmployeeIdentity {
                id: row.get(0)?,
                employee_id: row.get(1)?,
                full_name: row.get(2)?,
                staff_group: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load borrower identity: {err}"))
}

fn active_loan_employee_id_tx(tx: &Transaction<'_>, asset_id: i64) -> Result<Option<i64>, String> {
    tx.query_row(
        "SELECT employee_id_fk FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
        params![asset_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("failed to inspect active asset loan: {err}"))
}

fn request_key_exists_tx(tx: &Transaction<'_>, request_key: &str) -> Result<bool, String> {
    let exists: i64 = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM borrow_requests WHERE request_key = ?)",
            params![request_key],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to check request key existence: {err}"))?;
    Ok(exists > 0)
}

fn generate_request_key_tx(tx: &Transaction<'_>) -> Result<String, String> {
    for _ in 0..16 {
        let suffix = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect::<String>()
            .to_uppercase();
        let request_key = format!("BR-{suffix}");
        if !request_key_exists_tx(tx, request_key.as_str())? {
            return Ok(request_key);
        }
    }

    Err("failed to generate a unique borrow request key".to_string())
}

fn load_request_state_tx(
    tx: &Transaction<'_>,
    request_id: i64,
) -> Result<(Option<i64>, String, String, Option<i64>), String> {
    tx.query_row(
        "SELECT employee_id_fk, status, COALESCE(request_type, 'borrow'), returned_by_employee_id_fk FROM borrow_requests WHERE id = ?",
        params![request_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .optional()
    .map_err(|err| format!("failed to load borrow request state: {err}"))?
    .ok_or_else(|| format!("borrow request with id {request_id} was not found"))
}

fn load_request_items_tx(
    tx: &Transaction<'_>,
    request_id: i64,
) -> Result<Vec<BorrowRequestItemRecord>, String> {
    let mut stmt = tx
        .prepare(
            "SELECT asset_id, asset_code_snapshot FROM borrow_request_items WHERE borrow_request_id = ? ORDER BY id ASC",
        )
        .map_err(|err| format!("failed to prepare borrow request item query: {err}"))?;

    let rows = stmt
        .query_map(params![request_id], |row| {
            Ok(BorrowRequestItemRecord {
                asset_id: row.get(0)?,
                asset_code_snapshot: row.get(1)?,
            })
        })
        .map_err(|err| format!("failed to query borrow request items: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read borrow request item row: {err}"))?);
    }

    Ok(items)
}

fn ensure_pending_status(status: &str) -> Result<(), String> {
    if status != REQUEST_STATUS_PENDING {
        return Err("borrow request is no longer pending".to_string());
    }

    Ok(())
}

fn load_request_asset_codes(conn: &Connection, request_id: i64) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT asset_code_snapshot FROM borrow_request_items WHERE borrow_request_id = ? ORDER BY id ASC",
        )
        .map_err(|err| format!("failed to prepare request asset code query: {err}"))?;

    let rows = stmt
        .query_map(params![request_id], |row| row.get::<_, String>(0))
        .map_err(|err| format!("failed to query request asset codes: {err}"))?;

    let mut asset_codes = Vec::new();
    for row in rows {
        asset_codes
            .push(row.map_err(|err| format!("failed to read request asset code row: {err}"))?);
    }

    Ok(asset_codes)
}

fn load_borrow_request_record(
    conn: &Connection,
    request_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let (
        id,
        request_key,
        submitted_employee_id,
        submitted_full_name,
        status,
        request_type,
        submitted_at,
        decision_note,
        manual_employee_email,
        manual_employee_team,
        borrower_employee_id_fk,
        borrower_staff_id,
        borrower_name,
        submitted_by_employee_id_fk,
        submitted_by_staff_id,
        submitted_by_name,
    ) = conn
        .query_row(
            r#"
            SELECT
              id,
              request_key,
              submitted_employee_id,
              submitted_full_name,
              status,
              COALESCE(request_type, 'borrow'),
              submitted_at,
              decision_note,
              manual_employee_email,
              manual_employee_team,
              borrower_employee_id_fk,
              borrower_staff_id_snapshot,
              borrower_name_snapshot,
              submitted_by_employee_id_fk,
              submitted_by_staff_id_snapshot,
              submitted_by_name_snapshot
            FROM borrow_requests
            WHERE id = ?
            "#,
            params![request_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("failed to load borrow request record: {err}"))?
        .ok_or_else(|| format!("borrow request with id {request_id} was not found"))?;

    let evidence_metadata = conn
        .query_row(
            "SELECT policy_acknowledged, confirmation_method, signature_png_blob IS NOT NULL, typed_name IS NOT NULL FROM borrow_request_confirmations WHERE borrow_request_id = ?",
            params![request_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("failed to load borrow request evidence metadata: {err}"))?;

    let effective_submitted_by_staff_id =
        submitted_by_staff_id.or_else(|| Some(submitted_employee_id.clone()));
    let effective_submitted_by_name = submitted_by_name.or_else(|| Some(submitted_full_name.clone()));

    Ok(BorrowRequestRecord {
        id,
        request_key,
        submitted_employee_id,
        submitted_full_name,
        status,
        request_type,
        asset_codes: load_request_asset_codes(conn, request_id)?,
        submitted_at,
        decision_note,
        manual_employee_email,
        manual_employee_team,
        borrower_employee_id_fk,
        borrower_staff_id,
        borrower_name,
        submitted_by_employee_id_fk,
        submitted_by_staff_id: effective_submitted_by_staff_id,
        submitted_by_name: effective_submitted_by_name,
        has_acknowledgment: evidence_metadata.as_ref().map(|value| value.0).unwrap_or(false),
        confirmation_method: evidence_metadata.as_ref().map(|value| value.1.clone()),
        has_signature: evidence_metadata.as_ref().map(|value| value.2).unwrap_or(false),
        has_typed_name: evidence_metadata.as_ref().map(|value| value.3).unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, configure_connection};

    use super::*;

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
    }

    fn seed_policy(conn: &mut Connection) -> HandleWithCarePolicyRecord {
        let account_id: i64 = conn
            .query_row(
                "SELECT id FROM app_local_accounts ORDER BY id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("load seeded account");
        save_handle_with_care_policy_conn(
            conn,
            account_id,
            "Handle with care: protect the equipment.",
            "Vui lòng giữ gìn thiết bị.",
        )
        .expect("save policy")
    }

    fn signature_data_url(width: u32, height: u32, ink: bool) -> String {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("write png header");
        let mut pixels = vec![255_u8; (width * height * 4) as usize];
        if ink {
            pixels[0..4].copy_from_slice(&[0, 0, 0, 255]);
            pixels[4..8].copy_from_slice(&[0, 0, 0, 255]);
        } else {
            pixels.fill(0);
        }
        writer.write_image_data(&pixels).expect("write png pixels");
        drop(writer);
        format!("data:image/png;base64,{}", STANDARD.encode(bytes))
    }

    fn confirmation(
        policy_version: Option<i64>,
        method: ConfirmationMethod,
        typed_name: Option<&str>,
        signature: Option<String>,
        asset_codes: &[&str],
    ) -> BorrowRequestConfirmationInput {
        let has_signature = signature.is_some();
        BorrowRequestConfirmationInput {
            policy_version,
            policy_acknowledged: policy_version.is_some(),
            confirmation_method: method,
            signature_png_base64: signature,
            signature_stroke_count: has_signature.then_some(3),
            signature_ink_present: has_signature.then_some(true),
            typed_name: typed_name.map(str::to_string),
            asset_codes_snapshot: asset_codes.iter().map(|code| (*code).to_string()).collect(),
        }
    }

    fn confirmed_submission(
        employee_id: &str,
        full_name: &str,
        asset_codes: &[&str],
        request_type: &str,
        confirmation: BorrowRequestConfirmationInput,
    ) -> BorrowRequestWithConfirmationInput {
        BorrowRequestWithConfirmationInput {
            request: BorrowRequestSubmitInput {
                submitted_employee_id: employee_id.to_string(),
                submitted_full_name: full_name.to_string(),
                asset_codes: asset_codes.iter().map(|code| (*code).to_string()).collect(),
                request_type: Some(request_type.to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
            confirmation: Some(confirmation),
        }
    }

    #[test]
    fn confirmation_methods_require_their_declared_evidence() {
        let mut conn = open_test_connection();
        let policy = seed_policy(&mut conn);
        seed_employee(&conn, "EE-CONF", "Nguyễn Văn A");

        for (index, (method, typed_name, signature)) in [
            (
                ConfirmationMethod::Signature,
                None,
                Some(signature_data_url(4, 4, true)),
            ),
            (ConfirmationMethod::TypedName, Some("Nguyễn Văn A"), None),
            (
                ConfirmationMethod::Both,
                Some("Nguyễn Văn A"),
                Some(signature_data_url(4, 4, true)),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let asset_code = format!("ASSET-CONF-{index}");
            seed_asset(&conn, asset_code.as_str(), "Laptop", ASSET_STATUS_IN_STOCK);
            let request = submit_borrow_request_with_confirmation_conn(
                &mut conn,
                confirmed_submission(
                    "EE-CONF",
                    "client supplied name is ignored",
                    &[asset_code.as_str()],
                    "borrow",
                    confirmation(
                        Some(policy.version),
                        method,
                        typed_name,
                        signature,
                        &[asset_code.as_str()],
                    ),
                ),
            )
            .expect("valid confirmation should submit");
            assert_eq!(request.status, REQUEST_STATUS_PENDING);
        }

        let missing = confirmation(
            Some(policy.version),
            ConfirmationMethod::Signature,
            None,
            None,
            &["ASSET-CONF-0"],
        );
        assert!(submit_borrow_request_with_confirmation_conn(
            &mut conn,
            confirmed_submission("EE-CONF", "ignored", &["ASSET-CONF-0"], "borrow", missing)
        )
        .is_err());

        seed_asset(&conn, "ASSET-CONF-NONE", "Laptop", ASSET_STATUS_IN_STOCK);
        let no_confirmation = BorrowRequestWithConfirmationInput {
            request: BorrowRequestSubmitInput {
                submitted_employee_id: "EE-CONF".to_string(),
                submitted_full_name: "ignored".to_string(),
                asset_codes: vec!["ASSET-CONF-NONE".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
            confirmation: None,
        };
        assert!(submit_borrow_request_with_confirmation_conn(&mut conn, no_confirmation).is_err());
    }

    #[test]
    fn policy_is_current_and_server_text_is_snapshotted() {
        let mut conn = open_test_connection();
        let policy = seed_policy(&mut conn);
        seed_employee(&conn, "EE-POLICY", "Policy Employee");
        seed_asset(&conn, "ASSET-POLICY", "Laptop", ASSET_STATUS_IN_STOCK);
        let request = submit_borrow_request_with_confirmation_conn(
            &mut conn,
            confirmed_submission(
                "EE-POLICY",
                "forged client name",
                &["ASSET-POLICY"],
                "borrow",
                confirmation(
                    Some(policy.version),
                    ConfirmationMethod::TypedName,
                    Some("Policy Employee"),
                    None,
                    &["ASSET-POLICY"],
                ),
            ),
        )
        .expect("current policy should submit");
        let snapshot: (i64, String, String, String) = conn
            .query_row(
                "SELECT policy_version, policy_text_en_snapshot, policy_text_vi_snapshot, asset_codes_snapshot_json FROM borrow_request_confirmations WHERE borrow_request_id = ?",
                params![request.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load confirmation snapshot");
        assert_eq!(snapshot.0, policy.version);
        assert_eq!(snapshot.1, policy.text_en);
        assert_eq!(snapshot.2, policy.text_vi);
        assert_eq!(snapshot.3, "[\"ASSET-POLICY\"]");
        let evidence = load_borrow_request_confirmation_conn(&conn, request.id)
            .expect("load confirmation evidence")
            .expect("confirmation evidence exists");
        assert_eq!(evidence.confirmation_method, ConfirmationMethod::TypedName);
        assert!(conn
            .execute(
                "UPDATE borrow_request_confirmations SET typed_name = 'changed' WHERE borrow_request_id = ?",
                params![request.id],
            )
            .is_err());
        assert!(conn
            .execute(
                "DELETE FROM borrow_request_confirmations WHERE borrow_request_id = ?",
                params![request.id],
            )
            .is_err());
        approve_borrow_request_conn(&mut conn, request.id, 1).expect("approve request");
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM borrow_request_confirmations WHERE borrow_request_id = ?",
                params![request.id],
                |row| row.get::<_, i64>(0),
            )
            .expect("count preserved evidence"),
            1
        );
    }

    #[test]
    fn typed_name_matching_is_unicode_whitespace_and_case_aware_without_ascii_folding() {
        assert_eq!(
            normalize_typed_name("  NGUYỄN   văn A ").unwrap(),
            "nguyễn văn a"
        );
        assert!(typed_names_match("Nguyễn Văn A", "  NGUYỄN   VĂN A "));
        assert!(!typed_names_match("Nguyễn Văn A", "Nguyen Van A"));
    }

    #[test]
    fn signature_validation_rejects_blank_malformed_non_png_oversized_and_bad_dimensions() {
        let valid = signature_data_url(4, 4, true);
        assert!(validate_signature_png(&valid, Some(3), Some(true)).is_ok());
        assert!(
            validate_signature_png("data:image/png;base64:not-base64", Some(3), Some(true))
                .is_err()
        );
        assert!(
            validate_signature_png("data:text/plain;base64,SGVsbG8=", Some(3), Some(true)).is_err()
        );
        assert!(
            validate_signature_png(&signature_data_url(4, 4, false), Some(3), Some(true)).is_err()
        );
        assert!(
            validate_signature_png(&signature_data_url(1201, 4, true), Some(3), Some(true))
                .is_err()
        );
        assert!(validate_signature_png(&valid, Some(0), Some(true)).is_err());
        assert!(validate_signature_png(&valid, Some(3), Some(false)).is_err());
    }

    #[test]
    fn return_uses_active_loan_borrower_and_submitted_by_operator() {
        let mut conn = open_test_connection();
        let policy = seed_policy(&mut conn);
        let borrower = seed_employee(&conn, "EE-BORROWER-2", "Original Borrower");
        seed_employee(&conn, "EE-RETURNER-2", "Actual Returner");
        let asset_id = seed_asset(&conn, "ASSET-RETURN-2", "Laptop", ASSET_STATUS_ASSIGNED);
        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('LOAN-2', ?, 'EE-BORROWER-2', 'Original Borrower', 'approved', 'borrow')",
            params![borrower],
        )
        .expect("insert borrow request");
        let borrow_request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![asset_id, borrower, borrow_request_id],
        )
        .expect("insert active loan");

        let return_request = submit_borrow_request_with_confirmation_conn(
            &mut conn,
            confirmed_submission(
                "EE-RETURNER-2",
                "Actual Returner",
                &["ASSET-RETURN-2"],
                "return",
                confirmation(
                    None,
                    ConfirmationMethod::TypedName,
                    Some("Actual Returner"),
                    None,
                    &["ASSET-RETURN-2"],
                ),
            ),
        )
        .expect("return by another employee should submit");
        let identities: (Option<i64>, Option<i64>, String, String) = conn
            .query_row(
                "SELECT borrower_employee_id_fk, submitted_by_employee_id_fk, borrower_name_snapshot, submitted_by_name_snapshot FROM borrow_requests WHERE id = ?",
                params![return_request.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load return identities");
        assert_eq!(identities.0, Some(borrower));
        assert_ne!(identities.0, identities.1);
        assert_eq!(identities.2, "Original Borrower");
        assert_eq!(identities.3, "Actual Returner");

        let borrower_two = seed_employee(&conn, "EE-BORROWER-3", "Second Borrower");
        let asset_two = seed_asset(&conn, "ASSET-RETURN-3", "Laptop", ASSET_STATUS_ASSIGNED);
        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('LOAN-3', ?, 'EE-BORROWER-3', 'Second Borrower', 'approved', 'borrow')",
            params![borrower_two],
        )
        .expect("insert second borrow request");
        let second_borrow_request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![asset_two, borrower_two, second_borrow_request_id],
        )
        .expect("insert second active loan");
        assert!(submit_borrow_request_with_confirmation_conn(
            &mut conn,
            confirmed_submission(
                "EE-RETURNER-2",
                "Actual Returner",
                &["ASSET-RETURN-2", "ASSET-RETURN-3"],
                "return",
                confirmation(
                    None,
                    ConfirmationMethod::TypedName,
                    Some("Actual Returner"),
                    None,
                    &["ASSET-RETURN-2", "ASSET-RETURN-3"],
                ),
            ),
        )
        .is_err());

        let stale_policy_confirmation = confirmation(
            Some(policy.version),
            ConfirmationMethod::TypedName,
            Some("Actual Returner"),
            None,
            &["ASSET-RETURN-2"],
        );
        assert!(submit_borrow_request_with_confirmation_conn(
            &mut conn,
            confirmed_submission(
                "EE-RETURNER-2",
                "Actual Returner",
                &["ASSET-RETURN-2"],
                "return",
                stale_policy_confirmation,
            )
        )
        .is_err());
    }

    fn seed_employee(conn: &Connection, employee_id: &str, full_name: &str) -> i64 {
        conn.execute(
            r#"
            INSERT INTO employees(employee_id, full_name, updated_at)
            VALUES(?, ?, datetime('now'))
            "#,
            params![employee_id, full_name],
        )
        .expect("insert employee");
        conn.last_insert_rowid()
    }

    fn seed_asset(conn: &Connection, asset_code: &str, asset_type: &str, status: &str) -> i64 {
        conn.execute(
            r#"
            INSERT INTO assets(asset_code, asset_type, display_name, status, created_at, updated_at)
            VALUES(?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
            params![asset_code, asset_type, asset_code, status],
        )
        .expect("insert asset");
        conn.last_insert_rowid()
    }

    fn request_status(conn: &Connection, request_id: i64) -> String {
        conn.query_row(
            "SELECT status FROM borrow_requests WHERE id = ?",
            params![request_id],
            |row| row.get(0),
        )
        .expect("load request status")
    }

    fn asset_status(conn: &Connection, asset_id: i64) -> String {
        conn.query_row(
            "SELECT status FROM assets WHERE id = ?",
            params![asset_id],
            |row| row.get(0),
        )
        .expect("load asset status")
    }

    fn active_loan_count(conn: &Connection, asset_id: i64) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
            params![asset_id],
            |row| row.get(0),
        )
        .expect("count active loans")
    }

    #[test]
    fn handle_with_care_policy_versions_are_immutable_and_identical_saves_reuse_current() {
        let mut conn = open_test_connection();
        let account_id: i64 = conn
            .query_row(
                "SELECT id FROM app_local_accounts ORDER BY id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("load seeded account");

        let first = save_handle_with_care_policy_conn(
            &mut conn,
            account_id,
            "Handle with care",
            "Vui long giu gin",
        )
        .expect("save first policy");
        let same = save_handle_with_care_policy_conn(
            &mut conn,
            account_id,
            "Handle with care",
            "Vui long giu gin",
        )
        .expect("reuse identical policy");
        assert_eq!(first.version, same.version);

        let second = save_handle_with_care_policy_conn(
            &mut conn,
            account_id,
            "Handle with care - updated",
            "Vui long giu gin - cap nhat",
        )
        .expect("save updated policy");
        assert!(second.version > first.version);
        assert_eq!(
            get_handle_with_care_policy_conn(&conn, first.version)
                .unwrap()
                .unwrap()
                .text_en,
            "Handle with care"
        );
        assert_eq!(
            get_current_handle_with_care_policy_conn(&conn)
                .unwrap()
                .unwrap()
                .version,
            second.version
        );

        assert!(
            save_handle_with_care_policy_conn(&mut conn, account_id, "", "Valid Vietnamese")
                .is_err()
        );
    }

    #[test]
    fn confirmation_evidence_restricts_request_deletion_and_round_trips_through_backup() {
        let mut conn = open_test_connection();
        let account_id: i64 = conn
            .query_row(
                "SELECT id FROM app_local_accounts ORDER BY id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("load seeded account");
        let policy =
            save_handle_with_care_policy_conn(&mut conn, account_id, "English", "Vietnamese")
                .expect("save policy");

        conn.execute(
            "INSERT INTO borrow_requests(request_key, submitted_employee_id, submitted_full_name) VALUES('CONFIRMATION-1', 'EE-1', 'Employee')",
            [],
        ).expect("insert request");
        let request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO borrow_request_confirmations(borrow_request_id, policy_version, policy_text_en_snapshot, policy_text_vi_snapshot, policy_acknowledged, asset_codes_snapshot_json, confirmation_method, confirmed_at) VALUES(?, ?, ?, ?, 1, '[]', 'typed_name', datetime('now'))",
            params![request_id, policy.version, policy.text_en, policy.text_vi],
        ).expect("insert confirmation");

        let delete_result = conn.execute(
            "DELETE FROM borrow_requests WHERE id = ?",
            params![request_id],
        );
        assert!(
            delete_result.is_err(),
            "confirmation must prevent request deletion"
        );

        let backup_path = std::env::temp_dir().join(format!(
            "staff-kit-phase1-{}-{}.sqlite3",
            std::process::id(),
            request_id
        ));
        let _ = std::fs::remove_file(&backup_path);
        let escaped = backup_path.to_string_lossy().replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))
            .expect("create backup");
        let restored = Connection::open(&backup_path).expect("open restored backup");
        configure_connection(&restored).expect("configure restored backup");
        apply_migrations(&restored).expect("migrate restored backup");
        assert_eq!(
            get_current_handle_with_care_policy_conn(&restored)
                .unwrap()
                .unwrap()
                .version,
            policy.version
        );
        assert_eq!(
            restored
                .query_row(
                    "SELECT COUNT(*) FROM borrow_request_confirmations",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        let _ = std::fs::remove_file(backup_path);
    }

    #[test]
    fn phase_c_known_staff_id_uses_authoritative_name_and_records_pending_claims() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Authoritative Name");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: " EE1001 ".to_string(),
                submitted_full_name: "Untrusted Name".to_string(),
                asset_codes: vec![" ASSET-001 ".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("known staff ID should submit");

        assert_eq!(request.submitted_employee_id, "EE1001");
        assert_eq!(request.submitted_full_name, "Authoritative Name");
        assert_eq!(request.status, REQUEST_STATUS_PENDING);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM asset_pending_claims WHERE borrow_request_id = ?",
                params![request.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn phase_c_unknown_staff_id_stores_manual_identity_without_employee_fk() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: " UNKNOWN-42 ".to_string(),
                submitted_full_name: " Manual Borrower ".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: Some("manual@example.test".to_string()),
                manual_employee_team: Some("Operations".to_string()),
            },
        )
        .expect("unknown staff ID should use manual entry");

        let row = conn
            .query_row(
                "SELECT employee_id_fk, manual_entry, manual_employee_id, manual_employee_name, manual_employee_email, manual_employee_team FROM borrow_requests WHERE id = ?",
                params![request.id],
                |row| {
                    Ok((
                        row.get::<_, Option<i64>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(row.1, 1);
        assert_eq!(row.2, "UNKNOWN-42");
        assert_eq!(row.3, "Manual Borrower");
        assert_eq!(row.0, None);
        assert_eq!(row.4.as_deref(), Some("manual@example.test"));
        assert_eq!(row.5.as_deref(), Some("Operations"));
    }

    #[test]
    fn phase_c_unknown_staff_id_return_preserves_manual_identity_and_closes_current_loan() {
        let mut conn = open_test_connection();
        let borrower = seed_employee(&conn, "EE-BORROWER", "Original Borrower");
        let asset_id = seed_asset(&conn, "ASSET-MANUAL-RETURN", "Laptop", "assigned");
        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('MANUAL-LOAN', ?, 'EE-BORROWER', 'Original Borrower', 'approved', 'borrow')",
            params![borrower],
        )
        .unwrap();
        let borrow_request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![asset_id, borrower, borrow_request_id],
        )
        .unwrap();

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "UNKNOWN-RETURNER".to_string(),
                submitted_full_name: "Manual Returner".to_string(),
                asset_codes: vec!["ASSET-MANUAL-RETURN".to_string()],
                request_type: Some("return".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("unknown Staff ID may submit a return");
        let identity: (Option<i64>, i64, String, String) = conn
            .query_row(
                "SELECT employee_id_fk, manual_entry, manual_employee_id, manual_employee_name FROM borrow_requests WHERE id = ?",
                params![request.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            identity,
            (
                None,
                1,
                "UNKNOWN-RETURNER".to_string(),
                "Manual Returner".to_string()
            )
        );

        approve_borrow_request_conn(&mut conn, request.id, 1).expect("approve manual return");
        let returned_at: Option<String> = conn
            .query_row("SELECT returned_at FROM asset_loans WHERE id = (SELECT id FROM asset_loans WHERE asset_id = ?)", params![asset_id], |row| row.get(0))
            .unwrap();
        assert!(returned_at.is_some());
        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![asset_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active_count, 0);
    }

    #[test]
    fn phase_c_cancellation_releases_claims_and_is_not_repeatable() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE-CANCEL", "Cancel User");
        seed_asset(&conn, "ASSET-CANCEL", "Laptop", "in_stock");
        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-CANCEL".to_string(),
                submitted_full_name: "Cancel User".to_string(),
                asset_codes: vec!["ASSET-CANCEL".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap();
        cancel_borrow_request_conn(&mut conn, request.id, 1).expect("cancel pending request");
        assert_eq!(request_status(&conn, request.id), "cancelled");
        assert_eq!(
            conn.query_row(
                "SELECT decided_by_account_id FROM borrow_requests WHERE id = ?",
                params![request.id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .unwrap(),
            Some(1)
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM asset_pending_claims", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        assert!(cancel_borrow_request_conn(&mut conn, request.id, 1).is_err());
    }

    #[test]
    fn phase_c_item_insert_failure_rolls_back_request_items_and_claims() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE-ROLLBACK", "Rollback User");
        seed_asset(&conn, "ASSET-ROLLBACK", "Laptop", "in_stock");
        conn.execute_batch(
            "CREATE TRIGGER fail_borrow_item_insert BEFORE INSERT ON borrow_request_items BEGIN SELECT RAISE(ABORT, 'forced item insertion failure'); END;",
        )
        .unwrap();
        assert!(submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-ROLLBACK".to_string(),
                submitted_full_name: "Rollback User".to_string(),
                asset_codes: vec!["ASSET-ROLLBACK".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .is_err());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM borrow_requests", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM borrow_request_items", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM asset_pending_claims", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
    }

    #[test]
    fn phase_c_offboarding_borrow_is_rejected_but_return_is_pending() {
        let mut conn = open_test_connection();
        let employee_id = seed_employee(&conn, "EE-OFF", "Offboarding User");
        conn.execute(
            "UPDATE employees SET staff_group = 'offboarding' WHERE id = ?",
            params![employee_id],
        )
        .unwrap();
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let borrow_error = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-OFF".to_string(),
                submitted_full_name: "Offboarding User".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap_err();
        assert!(borrow_error.contains("offboarding"));

        conn.execute(
            "UPDATE assets SET status = 'assigned' WHERE asset_code = 'ASSET-001'",
            [],
        )
        .unwrap();
        let borrower = seed_employee(&conn, "EE-BORROWER", "Original Borrower");
        conn.execute(
                "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('SEED-RETURN', ?, 'EE-BORROWER', 'Original Borrower', 'approved', 'borrow')",
                params![borrower],
            )
            .unwrap();
        let request_id = conn.last_insert_rowid();
        let asset_id: i64 = conn
            .query_row(
                "SELECT id FROM assets WHERE asset_code = 'ASSET-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![asset_id, borrower, request_id],
        )
        .unwrap();

        let return_request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-OFF".to_string(),
                submitted_full_name: "Offboarding User".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: Some("return".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("offboarding employee may return assets");
        assert_eq!(return_request.status, REQUEST_STATUS_PENDING);
        assert_eq!(asset_status(&conn, asset_id), ASSET_STATUS_ASSIGNED);
    }

    #[test]
    fn phase_c_onboarding_borrow_does_not_mutate_employee_group() {
        let mut conn = open_test_connection();
        let employee_id = seed_employee(&conn, "EE-ON", "Onboarding User");
        conn.execute(
            "UPDATE employees SET staff_group = 'onboarding' WHERE id = ?",
            params![employee_id],
        )
        .unwrap();
        seed_asset(&conn, "ASSET-ON", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-ON".to_string(),
                submitted_full_name: "Wrong Name".to_string(),
                asset_codes: vec!["ASSET-ON".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("onboarding employee may borrow");
        assert_eq!(request.status, REQUEST_STATUS_PENDING);
        let group: String = conn
            .query_row(
                "SELECT staff_group FROM employees WHERE id = ?",
                params![employee_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(group, "onboarding");
    }

    #[test]
    fn phase_c_return_by_another_employee_preserves_borrower_and_returner() {
        let mut conn = open_test_connection();
        let borrower = seed_employee(&conn, "EE-BORROWER", "Original Borrower");
        seed_employee(&conn, "EE-RETURNER", "Returning Employee");
        let asset_id = seed_asset(&conn, "ASSET-RETURN", "Laptop", "assigned");
        conn.execute(
                "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('SEED-LOAN', ?, 'EE-BORROWER', 'Original Borrower', 'approved', 'borrow')",
                params![borrower],
            )
            .unwrap();
        let borrow_request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![asset_id, borrower, borrow_request_id],
        )
        .unwrap();

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-RETURNER".to_string(),
                submitted_full_name: "Returning Employee".to_string(),
                asset_codes: vec!["ASSET-RETURN".to_string()],
                request_type: Some("return".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap();
        approve_borrow_request_conn(&mut conn, request.id, 1).unwrap();

        let (loan_borrower, returned_by): (i64, i64) = conn
            .query_row(
                "SELECT employee_id_fk, returned_by_employee_id_fk FROM asset_loans WHERE asset_id = ?",
                params![asset_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let returner: i64 = conn
            .query_row(
                "SELECT id FROM employees WHERE employee_id = 'EE-RETURNER'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(loan_borrower, borrower);
        assert_eq!(returned_by, returner);
        assert_eq!(asset_status(&conn, asset_id), ASSET_STATUS_IN_STOCK);
    }

    #[test]
    fn phase_c_duplicate_asset_codes_and_competing_claims_are_rejected_atomically() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Employee One");
        seed_employee(&conn, "EE1002", "Employee Two");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");
        seed_asset(&conn, "ASSET-002", "Mouse", "in_stock");

        let first = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Employee One".to_string(),
                asset_codes: vec!["ASSET-001".to_string(), "ASSET-002".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap();

        let duplicate = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1002".to_string(),
                submitted_full_name: "Employee Two".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap_err();
        assert!(duplicate.contains("pending") || duplicate.contains("claim"));
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM borrow_requests WHERE request_key != (SELECT request_key FROM borrow_requests WHERE id = ?)",
                params![first.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn phase_c_unavailable_lifecycle_and_active_loan_assets_are_rejected() {
        let mut conn = open_test_connection();
        let borrower = seed_employee(&conn, "EE1001", "Employee One");
        for (code, status) in [
            ("ASSET-ASSIGNED", "assigned"),
            ("ASSET-RETIRED", "retired"),
            ("ASSET-DISPOSED", "disposed"),
            ("ASSET-LOST", "lost"),
        ] {
            seed_asset(&conn, code, "Laptop", status);
        }
        let active_asset = seed_asset(&conn, "ASSET-ACTIVE", "Laptop", "in_stock");
        let borrow_request_id = conn
            .execute(
                "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('ACTIVE-LOAN', ?, 'EE1001', 'Employee One', 'approved', 'borrow')",
                params![borrower],
            )
            .unwrap();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![active_asset, borrower, borrow_request_id as i64],
        )
        .unwrap();

        for code in [
            "ASSET-ASSIGNED",
            "ASSET-RETIRED",
            "ASSET-DISPOSED",
            "ASSET-LOST",
            "ASSET-ACTIVE",
        ] {
            let error = submit_borrow_request_conn(
                &mut conn,
                BorrowRequestSubmitInput {
                    submitted_employee_id: "EE1001".to_string(),
                    submitted_full_name: "Employee One".to_string(),
                    asset_codes: vec![code.to_string()],
                    request_type: Some("borrow".to_string()),
                    manual_employee_email: None,
                    manual_employee_team: None,
                },
            )
            .unwrap_err();
            assert!(error.contains("asset"));
        }
    }

    #[test]
    fn phase_c_multi_asset_failure_rolls_back_request_items_and_claims() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Employee One");
        seed_employee(&conn, "EE1002", "Employee Two");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");
        let claimed_asset = seed_asset(&conn, "ASSET-002", "Mouse", "in_stock");
        conn.execute(
                "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES('CLAIM-SEED', (SELECT id FROM employees WHERE employee_id = 'EE1002'), 'EE1002', 'Employee Two', 'pending', 'borrow')",
                [],
            )
            .unwrap();
        let claim_request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO asset_pending_claims(asset_id, borrow_request_id) VALUES(?, ?)",
            params![claimed_asset, claim_request_id],
        )
        .unwrap();

        let error = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Employee One".to_string(),
                asset_codes: vec!["ASSET-001".to_string(), "ASSET-002".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("pending") || error.contains("claim"));
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM borrow_requests WHERE submitted_employee_id = 'EE1001'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM borrow_request_items WHERE borrow_request_id != 999",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn phase_c_concurrent_overlapping_multi_asset_claims_leave_one_complete_request() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let path = std::env::temp_dir().join(format!(
            "staff-kit-phase-c-concurrency-{}.sqlite3",
            rand::random::<u64>()
        ));
        let setup = Connection::open(&path).expect("open concurrency database");
        configure_connection(&setup).expect("configure concurrency database");
        apply_migrations(&setup).expect("migrate concurrency database");
        seed_employee(&setup, "EE1001", "Employee One");
        seed_employee(&setup, "EE1002", "Employee Two");
        seed_asset(&setup, "ASSET-CONCURRENT", "Laptop", "in_stock");
        seed_asset(&setup, "ASSET-A", "Mouse", "in_stock");
        seed_asset(&setup, "ASSET-B", "Mouse", "in_stock");
        drop(setup);

        let barrier = Arc::new(Barrier::new(2));
        let run = |employee_id: &'static str,
                   full_name: &'static str,
                   request_id: &'static str,
                   barrier: Arc<Barrier>,
                   asset_codes: &'static [&'static str],
                   path: std::path::PathBuf| {
            thread::spawn(move || {
                let mut conn = Connection::open(path).expect("open worker database");
                configure_connection(&conn).expect("configure worker database");
                barrier.wait();
                submit_borrow_request_conn(
                    &mut conn,
                    BorrowRequestSubmitInput {
                        submitted_employee_id: employee_id.to_string(),
                        submitted_full_name: full_name.to_string(),
                        asset_codes: asset_codes.iter().map(|code| (*code).to_string()).collect(),
                        request_type: Some("borrow".to_string()),
                        manual_employee_email: None,
                        manual_employee_team: None,
                    },
                )
                .map(|request| request.id)
                .map_err(|error| error.to_string())
                .map(|_| request_id)
            })
        };

        let first = run(
            "EE1001",
            "Employee One",
            "request-a",
            Arc::clone(&barrier),
            &["ASSET-CONCURRENT", "ASSET-A"],
            path.clone(),
        );
        let second = run(
            "EE1002",
            "Employee Two",
            "request-b",
            barrier,
            &["ASSET-CONCURRENT", "ASSET-B"],
            path.clone(),
        );
        let first = first.join().expect("first worker must not panic");
        let second = second.join().expect("second worker must not panic");
        assert_eq!((first.is_ok() as usize) + (second.is_ok() as usize), 1);
        assert_eq!((first.is_err() as usize) + (second.is_err() as usize), 1);

        let verify = Connection::open(&path).expect("open verification database");
        configure_connection(&verify).expect("configure verification database");
        let request_count: i64 = verify
            .query_row("SELECT COUNT(*) FROM borrow_requests", [], |row| row.get(0))
            .unwrap();
        let item_count: i64 = verify
            .query_row("SELECT COUNT(*) FROM borrow_request_items", [], |row| {
                row.get(0)
            })
            .unwrap();
        let claim_count: i64 = verify
            .query_row("SELECT COUNT(*) FROM asset_pending_claims", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(request_count, 1);
        assert_eq!(item_count, 2);
        assert_eq!(claim_count, 2);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    #[test]
    fn submit_borrow_request_creates_pending_request_for_valid_employee_and_assets() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Nguyen Van A");
        let asset_a = seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");
        let asset_b = seed_asset(&conn, "ASSET-002", "Mouse", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string(), "ASSET-002".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("submit borrow request");

        assert_eq!(request.status, "pending");
        assert_eq!(request.submitted_employee_id, "EE1001");
        assert_eq!(request.submitted_full_name, "Nguyen Van A");
        assert_eq!(request.asset_codes, vec!["ASSET-001", "ASSET-002"]);
        assert_eq!(request_status(&conn, request.id), "pending");
        assert_eq!(asset_status(&conn, asset_a), "in_stock");
        assert_eq!(asset_status(&conn, asset_b), "in_stock");
        assert_eq!(active_loan_count(&conn, asset_a), 0);
        assert_eq!(active_loan_count(&conn, asset_b), 0);
    }

    #[test]
    fn submit_borrow_request_accepts_unknown_staff_id_as_manual_entry() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "UNKNOWN".to_string(),
                submitted_full_name: "Ghost User".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("unknown employee should use manual entry");

        assert_eq!(request.submitted_employee_id, "UNKNOWN");
        assert_eq!(request.submitted_full_name, "Ghost User");
    }

    #[test]
    fn submit_borrow_request_rejects_duplicate_asset_codes() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Nguyen Van A");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let error = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string(), "ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect_err("duplicate asset codes should be rejected");

        assert!(error.contains("duplicate"));
    }

    #[test]
    fn approve_borrow_request_creates_loan_and_updates_asset_status() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "EE1001", "Nguyen Van A");
        let asset_id = seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("submit borrow request");

        let approved =
            approve_borrow_request_conn(&mut conn, request.id, 1).expect("approve request");

        assert_eq!(approved.status, "approved");
        assert_eq!(request_status(&conn, request.id), "approved");
        assert_eq!(asset_status(&conn, asset_id), "assigned");
        assert_eq!(active_loan_count(&conn, asset_id), 1);

        let loan_employee_id: i64 = conn
            .query_row(
                "SELECT employee_id_fk FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![asset_id],
                |row| row.get(0),
            )
            .expect("load active loan employee");
        assert_eq!(loan_employee_id, employee_row_id);
    }

    #[test]
    fn reject_borrow_request_keeps_asset_status_unchanged() {
        let mut conn = open_test_connection();
        let asset_id = seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");
        seed_employee(&conn, "EE1001", "Nguyen Van A");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("submit borrow request");

        let rejected =
            reject_borrow_request_conn(&mut conn, request.id, 1, "Wrong asset type".to_string())
                .expect("reject request");

        assert_eq!(rejected.status, "rejected");
        assert_eq!(request_status(&conn, request.id), "rejected");
        assert_eq!(asset_status(&conn, asset_id), "in_stock");
        assert_eq!(active_loan_count(&conn, asset_id), 0);
    }

    #[test]
    fn approve_borrow_request_blocks_second_decision() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Nguyen Van A");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("submit borrow request");

        approve_borrow_request_conn(&mut conn, request.id, 1).expect("first approval");

        let error = approve_borrow_request_conn(&mut conn, request.id, 1)
            .expect_err("second approval must be blocked");

        assert!(error.contains("pending"));
    }

    #[test]
    fn reject_borrow_request_blocks_second_decision_and_claim_is_already_released() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE-REJECT", "Reject User");
        seed_asset(&conn, "ASSET-REJECT", "Laptop", "in_stock");
        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE-REJECT".to_string(),
                submitted_full_name: "Reject User".to_string(),
                asset_codes: vec!["ASSET-REJECT".to_string()],
                request_type: Some("borrow".to_string()),
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .unwrap();
        reject_borrow_request_conn(&mut conn, request.id, 1, "not needed".to_string()).unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM asset_pending_claims", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        assert!(reject_borrow_request_conn(&mut conn, request.id, 1, "retry".to_string()).is_err());
    }

    #[test]
    fn list_pending_borrow_requests_returns_only_pending_newest_first() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Nguyen Van A");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");
        seed_asset(&conn, "ASSET-002", "Mouse", "in_stock");
        seed_asset(&conn, "ASSET-003", "Headset", "in_stock");

        let first_pending = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("create first pending request");
        conn.execute(
            "INSERT INTO borrow_request_confirmations(borrow_request_id, policy_version, policy_acknowledged, asset_codes_snapshot_json, confirmation_method, typed_name, confirmed_at) VALUES(?, NULL, 1, '[\\\"ASSET-001\\\"]', 'typed_name', 'Nguyen Van A', datetime('now'))",
            params![first_pending.id],
        )
        .expect("insert queue evidence metadata");

        let approved_request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-002".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("create request to approve");
        approve_borrow_request_conn(&mut conn, approved_request.id, 1).expect("approve request");

        let last_pending = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-003".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("create last pending request");

        let pending = list_pending_borrow_requests_conn(&conn).expect("list pending requests");

        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].id, last_pending.id);
        assert_eq!(pending[1].id, first_pending.id);
        assert!(pending.iter().all(|record| record.status == "pending"));
        assert!(pending[1].has_acknowledgment);
        assert_eq!(pending[1].confirmation_method.as_deref(), Some("typed_name"));
        assert!(!pending[1].has_signature);
        assert!(pending[1].has_typed_name);
    }

    #[test]
    fn load_borrow_request_detail_returns_asset_codes_and_decision_note() {
        let mut conn = open_test_connection();
        seed_employee(&conn, "EE1001", "Nguyen Van A");
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                request_type: None,
                manual_employee_email: None,
                manual_employee_team: None,
            },
        )
        .expect("submit request");

        reject_borrow_request_conn(&mut conn, request.id, 1, "Wrong asset type".to_string())
            .expect("reject request");

        let detail =
            load_borrow_request_detail_conn(&conn, request.id).expect("load borrow detail");

        assert_eq!(detail.id, request.id);
        assert_eq!(detail.asset_codes, vec!["ASSET-001"]);
        assert_eq!(detail.status, "rejected");
        assert_eq!(detail.decision_note.as_deref(), Some("Wrong asset type"));
    }

    #[test]
    fn normalize_detected_borrow_lan_host_candidate_accepts_real_ips_and_rejects_loopback() {
        assert_eq!(
            normalize_detected_borrow_lan_host_candidate("192.168.2.15"),
            Some("192.168.2.15".to_string())
        );
        assert_eq!(
            normalize_detected_borrow_lan_host_candidate("10.24.8.9\n"),
            Some("10.24.8.9".to_string())
        );
        assert_eq!(
            normalize_detected_borrow_lan_host_candidate(" 2001:db8::10 "),
            Some("2001:db8::10".to_string())
        );
        assert_eq!(
            normalize_detected_borrow_lan_host_candidate("127.0.0.1"),
            None
        );
        assert_eq!(
            normalize_detected_borrow_lan_host_candidate("0.0.0.0"),
            None
        );
        assert_eq!(
            normalize_detected_borrow_lan_host_candidate("not-an-ip"),
            None
        );
    }

    #[test]
    fn selects_gateway_backed_physical_interface_before_gatewayless_interface() {
        let candidates = vec![
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(10, 184, 1, 51),
                has_default_gateway: false,
                is_physical: true,
                route_metric: 5,
            },
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(192, 168, 88, 69),
                has_default_gateway: true,
                is_physical: true,
                route_metric: 25,
            },
        ];

        assert_eq!(
            select_borrow_lan_host_candidate(&candidates),
            Some("192.168.88.69".to_string())
        );
    }

    #[test]
    fn selector_excludes_loopback_and_link_local_addresses() {
        let candidates = vec![
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(127, 0, 0, 1),
                has_default_gateway: true,
                is_physical: true,
                route_metric: 1,
            },
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(169, 254, 20, 10),
                has_default_gateway: true,
                is_physical: true,
                route_metric: 2,
            },
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(10, 24, 8, 9),
                has_default_gateway: false,
                is_physical: true,
                route_metric: 10,
            },
        ];

        assert_eq!(
            select_borrow_lan_host_candidate(&candidates),
            Some("10.24.8.9".to_string())
        );
    }

    #[test]
    fn selector_falls_back_to_private_interface_without_gateway() {
        let candidates = vec![
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(172, 20, 4, 12),
                has_default_gateway: false,
                is_physical: false,
                route_metric: 50,
            },
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(10, 0, 0, 4),
                has_default_gateway: false,
                is_physical: true,
                route_metric: 100,
            },
        ];

        assert_eq!(
            select_borrow_lan_host_candidate(&candidates),
            Some("10.0.0.4".to_string())
        );
    }

    #[test]
    fn selector_is_deterministic_for_multiple_equal_priority_adapters() {
        let candidates = vec![
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(192, 168, 50, 8),
                has_default_gateway: true,
                is_physical: true,
                route_metric: 10,
            },
            BorrowLanInterfaceCandidate {
                host: Ipv4Addr::new(192, 168, 50, 7),
                has_default_gateway: true,
                is_physical: true,
                route_metric: 10,
            },
        ];

        assert_eq!(
            select_borrow_lan_host_candidate(&candidates),
            Some("192.168.50.7".to_string())
        );
    }

    #[test]
    fn selector_has_no_subnet_specific_preference() {
        let candidates = vec![BorrowLanInterfaceCandidate {
            host: Ipv4Addr::new(172, 31, 12, 3),
            has_default_gateway: true,
            is_physical: true,
            route_metric: 1,
        }];

        assert_eq!(
            select_borrow_lan_host_candidate(&candidates),
            Some("172.31.12.3".to_string())
        );
    }

    #[test]
    fn build_borrow_lan_url_wraps_ipv6_hosts() {
        assert_eq!(
            build_borrow_lan_url("203.0.113.45", 8787),
            "http://203.0.113.45:8787/borrow"
        );
        assert_eq!(
            build_borrow_lan_url("2001:db8::10", 8787),
            "http://[2001:db8::10]:8787/borrow"
        );
    }

    #[test]
    fn borrow_lan_settings_default_to_disabled_until_explicitly_enabled() {
        let conn = open_test_connection();

        let settings = read_borrow_lan_settings(&conn).expect("read default borrow lan settings");
        assert!(!settings.enabled);

        set_setting_value(&conn, BORROW_LAN_ENABLED_SETTING_KEY, Some("1"))
            .expect("enable borrow lan setting");

        let updated_settings =
            read_borrow_lan_settings(&conn).expect("read updated borrow lan settings");
        assert!(updated_settings.enabled);
    }
}
