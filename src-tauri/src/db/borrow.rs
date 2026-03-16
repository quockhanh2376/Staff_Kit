use std::collections::HashSet;

use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

use super::auth;
use super::schema::{BORROW_LAN_HOST_SETTING_KEY, BORROW_LAN_PORT_SETTING_KEY};
use super::{
    asset, audit, get_setting_value, humanize_sqlite_error, open_runtime_connection, require_text,
    set_setting_value,
};

const REQUEST_STATUS_PENDING: &str = "pending";
const REQUEST_STATUS_APPROVED: &str = "approved";
const REQUEST_STATUS_REJECTED: &str = "rejected";
const ASSET_STATUS_IN_STOCK: &str = "in_stock";
const ASSET_STATUS_BORROWED: &str = "borrowed";
const DEFAULT_BORROW_LAN_PORT: u16 = 8787;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLanSettings {
    pub host: String,
    pub port: u16,
    pub borrow_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLanSettingsUpdateInput {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRequestSubmitInput {
    pub submitted_employee_id: String,
    pub submitted_full_name: String,
    pub asset_codes: Vec<String>,
    pub submit_source_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRequestRecord {
    pub id: i64,
    pub request_key: String,
    pub submitted_employee_id: String,
    pub submitted_full_name: String,
    pub status: String,
    pub asset_codes: Vec<String>,
    pub submitted_at: String,
    pub decision_note: Option<String>,
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

pub fn get_borrow_lan_settings(app: &AppHandle) -> Result<BorrowLanSettings, String> {
    let conn = open_runtime_connection(app)?;
    read_borrow_lan_settings(&conn)
}

pub fn update_borrow_lan_settings(
    app: &AppHandle,
    payload: BorrowLanSettingsUpdateInput,
) -> Result<BorrowLanSettings, String> {
    let conn = open_runtime_connection(app)?;
    let host = require_text(payload.host, "host")?;
    if payload.port == 0 {
        return Err("port must be between 1 and 65535".to_string());
    }

    set_setting_value(&conn, BORROW_LAN_HOST_SETTING_KEY, Some(host.as_str()))?;
    let port_text = payload.port.to_string();
    set_setting_value(
        &conn,
        BORROW_LAN_PORT_SETTING_KEY,
        Some(port_text.as_str()),
    )?;

    let payload_json = json!({
        "host": host,
        "port": payload.port,
    })
    .to_string();

    let actor_ref = auth::get_active_local_account_id(&conn)?
        .map(|id| id.to_string())
        .unwrap_or_default();
    audit::insert_audit_log_conn(
        &conn,
        "borrow_lan.update_settings",
        "local_account",
        if actor_ref.is_empty() {
            None
        } else {
            Some(actor_ref.as_str())
        },
        "borrow_lan",
        "config",
        Some(payload_json.as_str()),
    )?;

    read_borrow_lan_settings(&conn)
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

pub fn approve_borrow_request(
    app: &AppHandle,
    request_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let reviewer_account_id = require_active_admin_account_id(&conn)?;
    approve_borrow_request_conn(&mut conn, request_id, reviewer_account_id)
}

pub fn reject_borrow_request(
    app: &AppHandle,
    payload: BorrowRequestRejectInput,
) -> Result<BorrowRequestRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let reviewer_account_id = require_active_admin_account_id(&conn)?;
    reject_borrow_request_conn(&mut conn, payload.request_id, reviewer_account_id, payload.note)
}

pub(crate) fn submit_borrow_request_conn(
    conn: &mut Connection,
    input: BorrowRequestSubmitInput,
) -> Result<BorrowRequestRecord, String> {
    let submitted_employee_id =
        require_text(input.submitted_employee_id, "submittedEmployeeId")?.to_uppercase();
    let submitted_full_name = require_text(input.submitted_full_name, "submittedFullName")?;
    let asset_codes = normalize_asset_codes(input.asset_codes)?;
    let submit_source_ip = input.submit_source_ip.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start borrow submit transaction: {err}"))?;

    let employee_id_fk =
        load_employee_row_id_by_business_id_tx(&tx, submitted_employee_id.as_str())?
            .ok_or_else(|| format!("employee '{}' was not found", submitted_employee_id))?;

    let mut assets = Vec::new();
    for asset_code in &asset_codes {
        let asset_record = asset::load_asset_by_code_tx(&tx, asset_code.as_str())?
            .ok_or_else(|| format!("asset '{}' was not found", asset_code))?;
        if asset_record.status != ASSET_STATUS_IN_STOCK {
            return Err(format!(
                "asset '{}' is not in_stock",
                asset_record.asset_code
            ));
        }
        assets.push(asset_record);
    }

    let request_key = generate_request_key_tx(&tx)?;
    tx.execute(
        r#"
        INSERT INTO borrow_requests(
          request_key,
          employee_id_fk,
          submitted_employee_id,
          submitted_full_name,
          status,
          submit_source_ip,
          submitted_at
        )
        VALUES(?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            request_key.as_str(),
            employee_id_fk,
            submitted_employee_id.as_str(),
            submitted_full_name.as_str(),
            REQUEST_STATUS_PENDING,
            submit_source_ip.as_deref(),
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
    }

    let audit_payload = json!({
        "submittedEmployeeId": submitted_employee_id,
        "submittedFullName": submitted_full_name,
        "assetCodes": asset_codes,
    })
    .to_string();

    let request_id_text = request_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.submit",
        "lan_public",
        submit_source_ip.as_deref(),
        "borrow_request",
        request_id_text.as_str(),
        Some(audit_payload.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit borrow submit transaction: {err}"))?;

    load_borrow_request_record(conn, request_id)
}

pub(crate) fn approve_borrow_request_conn(
    conn: &mut Connection,
    request_id: i64,
    reviewer_account_id: i64,
) -> Result<BorrowRequestRecord, String> {
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start borrow approval transaction: {err}"))?;

    let (employee_id_fk, status) = load_request_state_tx(&tx, request_id)?;
    ensure_pending_status(status.as_str())?;

    let request_items = load_request_items_tx(&tx, request_id)?;
    if request_items.is_empty() {
        return Err("borrow request has no asset items".to_string());
    }

    for item in &request_items {
        let asset_record = asset::load_asset_by_id_tx(&tx, item.asset_id)?
            .ok_or_else(|| format!("asset with id {} was not found", item.asset_id))?;
        if asset_record.status != ASSET_STATUS_IN_STOCK {
            return Err(format!(
                "asset '{}' is not in_stock",
                asset_record.asset_code
            ));
        }
    }

    for item in &request_items {
        asset::set_asset_status_tx(&tx, item.asset_id, ASSET_STATUS_BORROWED)?;
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
        .transaction()
        .map_err(|err| format!("failed to start borrow reject transaction: {err}"))?;

    let (_, status) = load_request_state_tx(&tx, request_id)?;
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
    let host = get_setting_value(conn, BORROW_LAN_HOST_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(default_borrow_lan_host);
    let port = get_setting_value(conn, BORROW_LAN_PORT_SETTING_KEY)?
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_BORROW_LAN_PORT);

    Ok(BorrowLanSettings {
        borrow_url: build_borrow_lan_url(host.as_str(), port),
        host,
        port,
    })
}

fn default_borrow_lan_host() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn build_borrow_lan_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}/borrow")
}

fn require_active_admin_account_id(conn: &Connection) -> Result<i64, String> {
    let account_id = auth::get_active_local_account_id(conn)?
        .ok_or_else(|| "an active local account is required".to_string())?;

    let role: String = conn
        .query_row(
            "SELECT role FROM app_local_accounts WHERE id = ?",
            params![account_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to load active local account role: {err}"))?
        .ok_or_else(|| format!("local account with id {account_id} was not found"))?;

    if !role.eq_ignore_ascii_case("admin") && !role.eq_ignore_ascii_case("super_admin") {
        return Err("active local account must be admin to review borrow requests".to_string());
    }

    Ok(account_id)
}

fn load_employee_row_id_by_business_id_tx(
    tx: &Transaction<'_>,
    submitted_employee_id: &str,
) -> Result<Option<i64>, String> {
    tx.query_row(
        "SELECT id FROM employees WHERE employee_id = ? COLLATE NOCASE",
        params![submitted_employee_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("failed to load employee row id: {err}"))
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

fn load_request_state_tx(tx: &Transaction<'_>, request_id: i64) -> Result<(i64, String), String> {
    tx.query_row(
        "SELECT employee_id_fk, status FROM borrow_requests WHERE id = ?",
        params![request_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
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
        submitted_at,
        decision_note,
    ) = conn
        .query_row(
            r#"
            SELECT
              id,
              request_key,
              submitted_employee_id,
              submitted_full_name,
              status,
              submitted_at,
              decision_note
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
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("failed to load borrow request record: {err}"))?
        .ok_or_else(|| format!("borrow request with id {request_id} was not found"))?;

    Ok(BorrowRequestRecord {
        id,
        request_key,
        submitted_employee_id,
        submitted_full_name,
        status,
        asset_codes: load_request_asset_codes(conn, request_id)?,
        submitted_at,
        decision_note,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, configure_connection};

    use super::*;

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
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
                submit_source_ip: Some("192.168.1.50".to_string()),
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
    fn submit_borrow_request_rejects_unknown_staff_id() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-001", "Laptop", "in_stock");

        let error = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "UNKNOWN".to_string(),
                submitted_full_name: "Ghost User".to_string(),
                asset_codes: vec!["ASSET-001".to_string()],
                submit_source_ip: None,
            },
        )
        .expect_err("unknown employee should be rejected");

        assert!(error.contains("employee"));
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
                submit_source_ip: None,
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
                submit_source_ip: None,
            },
        )
        .expect("submit borrow request");

        let approved =
            approve_borrow_request_conn(&mut conn, request.id, 1).expect("approve request");

        assert_eq!(approved.status, "approved");
        assert_eq!(request_status(&conn, request.id), "approved");
        assert_eq!(asset_status(&conn, asset_id), "borrowed");
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
                submit_source_ip: None,
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
                submit_source_ip: None,
            },
        )
        .expect("submit borrow request");

        approve_borrow_request_conn(&mut conn, request.id, 1).expect("first approval");

        let error = approve_borrow_request_conn(&mut conn, request.id, 1)
            .expect_err("second approval must be blocked");

        assert!(error.contains("pending"));
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
                submit_source_ip: None,
            },
        )
        .expect("create first pending request");

        let approved_request = submit_borrow_request_conn(
            &mut conn,
            BorrowRequestSubmitInput {
                submitted_employee_id: "EE1001".to_string(),
                submitted_full_name: "Nguyen Van A".to_string(),
                asset_codes: vec!["ASSET-002".to_string()],
                submit_source_ip: None,
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
                submit_source_ip: None,
            },
        )
        .expect("create last pending request");

        let pending = list_pending_borrow_requests_conn(&conn).expect("list pending requests");

        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].id, last_pending.id);
        assert_eq!(pending[1].id, first_pending.id);
        assert!(pending.iter().all(|record| record.status == "pending"));
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
                submit_source_ip: Some("192.168.1.50".to_string()),
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
}
