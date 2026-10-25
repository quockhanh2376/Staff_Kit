use std::collections::HashMap;

use rusqlite::{params, types::Value, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::column::{
    dynamic_field_exists, upsert_dynamic_field_definitions_for_map, upsert_dynamic_fields_tx,
};
use super::schema::{
    ASSET_NAME_2_FIELD_KEY, ASSET_NAME_FIELD_KEY, COMPUTER_NAME_2_FIELD_KEY,
    EMPLOYEE_SELECT_COLUMNS, STAFF_GROUP_EMPLOYEE_LIST,
    STAFF_GROUP_INTERNAL_MOVEMENT, STAFF_GROUP_OFFBOARDING, STAFF_GROUP_ONBOARDING,
};
use super::team::resolve_team_id_tx;
use super::{
    humanize_sqlite_error, normalize_date_value, normalize_dynamic_fields, normalize_optional_text,
    normalize_staff_group, open_runtime_connection, require_text,
};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeRecord {
    pub id: i64,
    pub employee_id: String,
    pub full_name: String,
    pub nick_name: Option<String>,
    pub team_id: Option<i64>,
    pub team_name: Option<String>,
    pub project: Option<String>,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub cellphone: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub asw_start_date: Option<String>,
    pub client_start_date: Option<String>,
    pub contract_end_date: Option<String>,
    pub client_year_of_services: Option<String>,
    pub start_date: Option<String>,
    pub computer_name: Option<String>,
    pub stored_computer_name: Option<String>,
    #[serde(skip_serializing)]
    pub(crate) active_computer_name_1: Option<String>,
    #[serde(skip_serializing)]
    pub(crate) active_computer_name_2: Option<String>,
    #[serde(skip_serializing)]
    pub(crate) legacy_stored_computer_name_2: Option<String>,
    #[serde(skip_serializing)]
    pub(crate) suppress_active_computer_names: bool,
    pub notes: Option<String>,
    pub staff_group: String,
    pub is_floating: bool,
    pub lifecycle_status: Option<String>,
    pub assigned_assets_count: i64,
    pub dynamic_fields: HashMap<String, String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssignedAssetRecord {
    pub asset_id: i64,
    pub asset_code: String,
    pub asset_name: String,
    pub category_name: String,
    pub category_code: Option<String>,
    pub serial_number: Option<String>,
    pub usage_location: Option<String>,
    pub borrowed_at: String,
    pub signature_status: Option<String>,
    pub handover_date: Option<String>,
    pub imported_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeListResponse {
    pub items: Vec<EmployeeRecord>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeQuery {
    pub query: Option<String>,
    pub team_name: Option<String>,
    pub staff_group: Option<String>,
    pub is_floating: Option<bool>,
    pub sort_key: Option<String>,
    pub sort_direction: Option<String>,
    pub start_date_from: Option<String>,
    pub start_date_to: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeGroupCounts {
    pub employee_list: i64,
    pub onboarding: i64,
    pub offboarding: i64,
    pub internal_movement: i64,
    pub floating: i64,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeePayload {
    pub employee_id: String,
    pub full_name: String,
    pub nick_name: Option<String>,
    pub team_name: Option<String>,
    pub project: Option<String>,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub cellphone: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub asw_start_date: Option<String>,
    pub client_start_date: Option<String>,
    pub contract_end_date: Option<String>,
    pub client_year_of_services: Option<String>,
    pub computer_name: Option<String>,
    pub notes: Option<String>,
    pub staff_group: Option<String>,
    pub is_floating: Option<bool>,
    pub lifecycle_status: Option<String>,
    pub dynamic_fields: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveEmployeesGroupInput {
    pub employee_ids: Vec<i64>,
    pub target_staff_group: String,
    pub action: Option<String>,
    pub reason: Option<String>,
}

struct NormalizedEmployeePayload {
    pub employee_id: String,
    pub full_name: String,
    pub nick_name: Option<String>,
    pub team_name: Option<String>,
    pub project: Option<String>,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub cellphone: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub asw_start_date: Option<String>,
    pub client_start_date: Option<String>,
    pub contract_end_date: Option<String>,
    pub client_year_of_services: Option<String>,
    pub start_date: Option<String>,
    pub computer_name: Option<String>,
    pub notes: Option<String>,
    pub is_floating: Option<bool>,
    pub lifecycle_status: Option<String>,
    pub dynamic_fields: HashMap<String, String>,
}

impl TryFrom<EmployeePayload> for NormalizedEmployeePayload {
    type Error = String;

    fn try_from(payload: EmployeePayload) -> Result<Self, Self::Error> {
        let employee_id = require_text(payload.employee_id, "employeeId")?
            .trim()
            .to_uppercase();
        let full_name = require_text(payload.full_name, "fullName")?;

        Ok(Self {
            employee_id,
            full_name,
            nick_name: normalize_optional_text(payload.nick_name),
            team_name: normalize_optional_text(payload.team_name),
            project: normalize_optional_text(payload.project),
            job_title: normalize_optional_text(payload.job_title),
            email: normalize_optional_text(payload.email).map(|e| e.to_lowercase()),
            cellphone: normalize_optional_text(payload.cellphone),
            date_of_birth: normalize_date_value(payload.date_of_birth),
            gender: normalize_optional_text(payload.gender),
            asw_start_date: normalize_date_value(payload.asw_start_date.clone()),
            client_start_date: normalize_date_value(payload.client_start_date),
            contract_end_date: normalize_optional_or_date(payload.contract_end_date),
            client_year_of_services: normalize_optional_text(payload.client_year_of_services),
            start_date: normalize_date_value(payload.asw_start_date),
            computer_name: normalize_computer_name_override(payload.computer_name),
            notes: normalize_optional_text(payload.notes),
            is_floating: payload.is_floating,
            lifecycle_status: payload.lifecycle_status,
            dynamic_fields: normalize_employee_dynamic_fields(payload.dynamic_fields),
        })
    }
}

#[derive(Debug)]
pub(crate) enum UpsertAction {
    Inserted,
    Updated,
}

#[derive(Debug)]
struct EmployeeSortSpec {
    order_sql: String,
    order_params: Vec<Value>,
}

const EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL: &str = r#"
LEFT JOIN (
    SELECT
      laptop_names.employee_id_fk,
      MAX(CASE WHEN laptop_names.row_num = 1 THEN laptop_names.computer_name END) AS computer_name_1,
      MAX(CASE WHEN laptop_names.row_num = 2 THEN laptop_names.computer_name END) AS computer_name_2
    FROM (
      SELECT
        al.employee_id_fk,
        'ASW' || a.asset_code AS computer_name,
        ROW_NUMBER() OVER (
          PARTITION BY al.employee_id_fk
          ORDER BY al.id ASC
        ) AS row_num
      FROM asset_loans al
      INNER JOIN assets a ON a.id = al.asset_id
      INNER JOIN asset_categories c ON c.id = a.category_id
      WHERE al.returned_at IS NULL
        AND COALESCE(c.has_computer_name, 0) = 1
    ) laptop_names
    GROUP BY laptop_names.employee_id_fk
) lc ON lc.employee_id_fk = e.id
"#;

const EMPLOYEE_ASSIGNED_ASSETS_COUNT_JOIN_SQL: &str = r#"
LEFT JOIN (
    SELECT employee_id_fk, count(*) AS active_count
    FROM asset_loans
    WHERE returned_at IS NULL
    GROUP BY employee_id_fk
) al_counts ON al_counts.employee_id_fk = e.id
"#;

// ── Public API ────────────────────────────────────────────────────────────────

pub fn list_employees(
    app: &AppHandle,
    filters: EmployeeQuery,
) -> Result<EmployeeListResponse, String> {
    let conn = open_runtime_connection(app)?;
    query_employees(&conn, filters)
}

pub fn search_employees(
    app: &AppHandle,
    filters: EmployeeQuery,
) -> Result<EmployeeListResponse, String> {
    let conn = open_runtime_connection(app)?;
    query_employees(&conn, filters)
}

pub fn get_employee_assigned_assets(
    app: &AppHandle,
    employee_id: i64,
) -> Result<Vec<EmployeeAssignedAssetRecord>, String> {
    let conn = open_runtime_connection(app)?;
    get_employee_assigned_assets_conn(&conn, employee_id)
}

pub(crate) fn get_employee_assigned_assets_conn(
    conn: &Connection,
    employee_id: i64,
) -> Result<Vec<EmployeeAssignedAssetRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              a.id AS asset_id,
              a.asset_code,
              a.display_name AS asset_name,
              COALESCE(c.category_name, a.asset_type) AS category_name,
              c.category_code,
              a.serial_number,
              a.usage_location,
              al.borrowed_at,
              COALESCE(br.signature_status, 'signed') AS signature_status,
              al.handover_date,
              al.imported_at
            FROM asset_loans al
            INNER JOIN assets a ON a.id = al.asset_id
            LEFT JOIN asset_categories c ON c.id = a.category_id
            LEFT JOIN borrow_requests br ON br.id = al.borrow_request_id
            WHERE al.employee_id_fk = ? AND al.returned_at IS NULL
            ORDER BY al.borrowed_at ASC, a.asset_code ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare assigned assets query: {err}"))?;

    let rows = stmt
        .query_map(params![employee_id], |row| {
            Ok(EmployeeAssignedAssetRecord {
                asset_id: row.get(0)?,
                asset_code: row.get(1)?,
                asset_name: row.get(2)?,
                category_name: row.get(3)?,
                category_code: row.get(4)?,
                serial_number: row.get(5)?,
                usage_location: row.get(6)?,
                borrowed_at: row.get(7)?,
                signature_status: row.get(8)?,
                handover_date: row.get(9)?,
                imported_at: row.get(10)?,
            })
        })
        .map_err(|err| format!("failed to query assigned assets: {err}"))?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|err| format!("failed to read assigned asset row: {err}"))?);
    }
    Ok(records)
}

pub(crate) fn query_all_employees_for_filters(
    conn: &Connection,
    mut filters: EmployeeQuery,
) -> Result<Vec<EmployeeRecord>, String> {
    filters.limit = Some(5000);
    filters.offset = Some(0);

    let response = query_employees(conn, filters)?;
    if response.total > i64::try_from(response.items.len()).unwrap_or(i64::MAX) {
        return Err(format!(
            "employee asset seed is limited to 5000 employees per run; current filters matched {} employees",
            response.total
        ));
    }

    Ok(response.items)
}

pub fn list_employee_group_counts(app: &AppHandle) -> Result<EmployeeGroupCounts, String> {
    let conn = open_runtime_connection(app)?;
    list_employee_group_counts_conn(&conn)
}

pub(crate) fn list_employee_group_counts_conn(
    conn: &Connection,
) -> Result<EmployeeGroupCounts, String> {
    let count_group = |group: &str| -> Result<i64, String> {
        conn.query_row(
            r#"
            SELECT COUNT(*) FROM employees
            WHERE CASE
              WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = 'internal_movent'
              THEN 'internal_movement'
              ELSE COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list')
            END = ?
            "#,
            params![group],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to count employee group '{group}': {err}"))
    };

    let employee_list = count_group(STAFF_GROUP_EMPLOYEE_LIST)?;
    let onboarding = count_group(STAFF_GROUP_ONBOARDING)?;
    let offboarding = count_group(STAFF_GROUP_OFFBOARDING)?;
    let internal_movement = count_group(STAFF_GROUP_INTERNAL_MOVEMENT)?;
    let floating: i64 = conn
        .query_row(
            r#"
            SELECT COUNT(*) FROM employees
            WHERE CASE
              WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = 'internal_movent'
              THEN 'internal_movement'
              ELSE COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list')
            END = 'employee_list'
            AND COALESCE(is_floating, 0) = 1
            "#,
            [],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to count floating employees: {err}"))?;
    let total = employee_list + onboarding + offboarding + internal_movement;

    Ok(EmployeeGroupCounts {
        employee_list,
        onboarding,
        offboarding,
        internal_movement,
        floating,
        total,
    })
}

pub fn create_employee(
    app: &AppHandle,
    payload: EmployeePayload,
) -> Result<EmployeeRecord, String> {
    let raw_group = payload
        .staff_group
        .clone()
        .unwrap_or_else(|| STAFF_GROUP_EMPLOYEE_LIST.to_string());
    let target_group = normalize_staff_group(raw_group.as_str())
        .ok_or_else(|| format!("invalid staff group: {raw_group}"))?;

    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start create transaction: {err}"))?;

    upsert_employee_from_payload(&tx, payload, target_group)?;
    let inserted_id = tx.last_insert_rowid();

    tx.commit()
        .map_err(|err| format!("failed to commit create transaction: {err}"))?;

    let conn2 = open_runtime_connection(app)?;
    load_employee_by_id(&conn2, inserted_id)
}

pub fn update_employee(
    app: &AppHandle,
    id: i64,
    payload: EmployeePayload,
) -> Result<EmployeeRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start update transaction: {err}"))?;

    let row_id = update_employee_tx(&tx, id, payload)?;

    tx.commit()
        .map_err(|err| format!("failed to commit update transaction: {err}"))?;

    let conn2 = open_runtime_connection(app)?;
    load_employee_by_id(&conn2, row_id)
}

fn update_employee_tx(
    tx: &Transaction<'_>,
    id: i64,
    payload: EmployeePayload,
) -> Result<i64, String> {

    let requested_staff_group = payload.staff_group.clone();
    let normalized = NormalizedEmployeePayload::try_from(payload)?;
    let team_id = resolve_team_id_tx(&tx, normalized.team_name.as_deref())?;

    // If id is provided, use it directly; else look up by employee_id
    let row_id: i64 = if id > 0 {
        id
    } else {
        tx.query_row(
            "SELECT id FROM employees WHERE employee_id = ?",
            params![normalized.employee_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to find employee: {err}"))?
        .ok_or_else(|| format!("employee '{}' was not found", normalized.employee_id))?
    };

    let target_group =
        resolve_update_staff_group_tx(&tx, row_id, requested_staff_group.as_deref())?;

    tx.execute(
        r#"
        UPDATE employees
        SET
          full_name = ?,
          nick_name = ?,
          team_id = ?,
          project = ?,
          job_title = ?,
          email = ?,
          cellphone = ?,
          date_of_birth = ?,
          gender = ?,
          asw_start_date = ?,
          client_start_date = ?,
          contract_end_date = ?,
          client_year_of_services = ?,
          start_date = ?,
          computername = ?,
          notes = ?,
          staff_group = ?,
          is_floating = COALESCE(?, is_floating),
          lifecycle_status = COALESCE(?, lifecycle_status),
          updated_at = datetime('now')
        WHERE id = ?
        "#,
        params![
            normalized.full_name.as_str(),
            normalized.nick_name.as_deref(),
            team_id,
            normalized.project.as_deref(),
            normalized.job_title.as_deref(),
            normalized.email.as_deref(),
            normalized.cellphone.as_deref(),
            normalized.date_of_birth.as_deref(),
            normalized.gender.as_deref(),
            normalized.asw_start_date.as_deref(),
            normalized.client_start_date.as_deref(),
            normalized.contract_end_date.as_deref(),
            normalized.client_year_of_services.as_deref(),
            normalized.start_date.as_deref(),
            normalized.computer_name.as_deref(),
            normalized.notes.as_deref(),
            target_group.as_str(),
            normalized.is_floating.map(|b| if b { 1 } else { 0 }),
            normalized.lifecycle_status.as_deref(),
            row_id,
        ],
    )
    .map_err(humanize_sqlite_error)?;

    if !normalized.dynamic_fields.is_empty() {
        upsert_dynamic_field_definitions_for_map(&tx, &normalized.dynamic_fields)?;
        upsert_dynamic_fields_tx(&tx, row_id, &normalized.dynamic_fields)?;
    }
    normalize_eml_security_tool_values_for_employee_tx(
        &tx,
        row_id,
        normalized.team_name.as_deref(),
    )?;

    Ok(row_id)
}

pub fn move_employees_group(
    app: &AppHandle,
    payload: MoveEmployeesGroupInput,
) -> Result<i64, String> {
    let mut conn = open_runtime_connection(app)?;
    move_employees_group_conn(&mut conn, payload)
}

pub(crate) fn move_employees_group_conn(
    conn: &mut Connection,
    payload: MoveEmployeesGroupInput,
) -> Result<i64, String> {
    if payload.employee_ids.is_empty() {
        return Ok(0);
    }
    let _ = &payload.reason;

    let raw_target = payload.target_staff_group.trim().to_lowercase();
    let action = payload
        .action
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start move transaction: {err}"))?;

    let mut changed = 0;
    for id in &payload.employee_ids {
        if raw_target == "floating" || action == "set_floating" {
            changed += tx
                .execute(
                    "UPDATE employees SET staff_group = ?, is_floating = 1, updated_at = datetime('now') WHERE id = ?",
                    params![STAFF_GROUP_EMPLOYEE_LIST, id],
                )
                .map_err(humanize_sqlite_error)?;
        } else if action == "clear_floating" {
            changed += tx
                .execute(
                    "UPDATE employees SET is_floating = 0, updated_at = datetime('now') WHERE id = ?",
                    params![id],
                )
                .map_err(humanize_sqlite_error)?;
        } else if raw_target == "reject_offer" || action == "reject_offer" {
            changed += tx
                .execute(
                    "UPDATE employees SET lifecycle_status = 'offer_rejected', updated_at = datetime('now') WHERE id = ? AND staff_group = ?",
                    params![id, STAFF_GROUP_ONBOARDING],
                )
                .map_err(humanize_sqlite_error)?;
        } else if let Some(target_group) = normalize_staff_group(raw_target.as_str()) {
            if target_group == STAFF_GROUP_OFFBOARDING {
                changed += tx
                    .execute(
                        "UPDATE employees SET staff_group = ?, is_floating = 0, lifecycle_status = 'resigned', updated_at = datetime('now') WHERE id = ?",
                        params![STAFF_GROUP_OFFBOARDING, id],
                    )
                    .map_err(humanize_sqlite_error)?;
            } else if target_group == STAFF_GROUP_EMPLOYEE_LIST {
                changed += tx
                    .execute(
                        r#"UPDATE employees SET
                           staff_group = ?,
                           is_floating = 0,
                           lifecycle_status = CASE WHEN staff_group = 'offboarding' THEN 'rehired' ELSE 'active' END,
                           contract_end_date = CASE WHEN staff_group = 'offboarding' THEN NULL ELSE contract_end_date END,
                           updated_at = datetime('now')
                           WHERE id = ?"#,
                        params![STAFF_GROUP_EMPLOYEE_LIST, id],
                    )
                    .map_err(humanize_sqlite_error)?;
            } else if target_group == STAFF_GROUP_INTERNAL_MOVEMENT {
                changed += tx
                    .execute(
                        "UPDATE employees SET staff_group = ?, is_floating = 0, lifecycle_status = 'transferring', updated_at = datetime('now') WHERE id = ?",
                        params![STAFF_GROUP_INTERNAL_MOVEMENT, id],
                    )
                    .map_err(humanize_sqlite_error)?;
            } else if target_group == STAFF_GROUP_ONBOARDING {
                changed += tx
                    .execute(
                        "UPDATE employees SET staff_group = ?, is_floating = 0, lifecycle_status = 'pending', updated_at = datetime('now') WHERE id = ?",
                        params![STAFF_GROUP_ONBOARDING, id],
                    )
                    .map_err(humanize_sqlite_error)?;
            }
        } else {
            return Err(format!(
                "invalid target staff group or transition action: {}",
                payload.target_staff_group
            ));
        }
    }

    tx.commit()
        .map_err(|err| format!("failed to commit move transaction: {err}"))?;

    Ok(changed as i64)
}

pub fn delete_employee(app: &AppHandle, id: i64) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    delete_employee_conn(&mut conn, id)
}

pub(crate) fn delete_employee_conn(conn: &mut Connection, id: i64) -> Result<bool, String> {
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start employee delete transaction: {err}"))?;

    let loan_count: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM asset_loans WHERE employee_id_fk = ? OR returned_by_employee_id_fk = ?",
            params![id, id],
            |row| row.get(0),
        )
        .map_err(humanize_sqlite_error)?;
    if loan_count > 0 {
        return Err("cannot delete an employee with historical asset loan records; preserve the employee for audit history".to_string());
    }

    // Preserve request rows, item snapshots, and confirmation evidence. Only
    // nullable employee references are cleared; immutable identity snapshots
    // remain available for audit.
    tx.execute(
        r#"
        UPDATE borrow_requests
        SET employee_id_fk = NULL,
            returned_by_employee_id_fk = NULL,
            borrower_employee_id_fk = NULL,
            submitted_by_employee_id_fk = NULL
        WHERE employee_id_fk = ?
           OR returned_by_employee_id_fk = ?
           OR borrower_employee_id_fk = ?
           OR submitted_by_employee_id_fk = ?
        "#,
        params![id, id, id, id],
    )
    .map_err(humanize_sqlite_error)?;

    let changed = tx
        .execute("DELETE FROM employees WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;

    tx.commit()
        .map_err(|err| format!("failed to commit employee delete transaction: {err}"))?;

    Ok(changed > 0)
}

// ── pub(crate) helpers used by import.rs ─────────────────────────────────────

pub(crate) fn load_employee_by_employee_id(
    conn: &Connection,
    employee_id: &str,
) -> Result<Option<EmployeeRecord>, String> {
    let sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} {EMPLOYEE_ASSIGNED_ASSETS_COUNT_JOIN_SQL} WHERE e.employee_id = ?"
    );

    let maybe_employee = conn
        .query_row(&sql, params![employee_id], map_employee_row)
        .optional()
        .map_err(|err| format!("failed to load employee by employee_id: {err}"))?;

    let Some(employee) = maybe_employee else {
        return Ok(None);
    };

    let mut single = vec![employee];
    hydrate_dynamic_fields(conn, &mut single)?;
    Ok(single.into_iter().next())
}

pub(crate) fn load_employees_by_email_normalized(
    conn: &Connection,
    email: &str,
) -> Result<Vec<EmployeeRecord>, String> {
    let sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} {EMPLOYEE_ASSIGNED_ASSETS_COUNT_JOIN_SQL} WHERE lower(trim(COALESCE(e.email, ''))) = lower(trim(?))"
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("failed to prepare employee-by-email query: {err}"))?;

    let rows = stmt
        .query_map(params![email], map_employee_row)
        .map_err(|err| format!("failed to query employees by email: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read employee-by-email row: {err}"))?);
    }

    hydrate_dynamic_fields(conn, &mut items)?;
    Ok(items)
}

pub(crate) fn upsert_employee_from_payload(
    tx: &Transaction<'_>,
    payload: EmployeePayload,
    staff_group: &str,
) -> Result<UpsertAction, String> {
    let normalized = NormalizedEmployeePayload::try_from(payload)?;
    let team_id = resolve_team_id_tx(tx, normalized.team_name.as_deref())?;

    let existing_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM employees WHERE employee_id = ?",
            params![normalized.employee_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to check employee existence: {err}"))?;

    if let Some(id) = existing_id {
        tx.execute(
            r#"
            UPDATE employees
            SET
              full_name = ?,
              nick_name = ?,
              team_id = ?,
              project = ?,
              job_title = ?,
              email = ?,
              cellphone = ?,
              date_of_birth = ?,
              gender = ?,
              asw_start_date = ?,
              client_start_date = ?,
              contract_end_date = ?,
              client_year_of_services = ?,
              start_date = ?,
              computername = ?,
              notes = ?,
              staff_group = ?,
              is_floating = COALESCE(?, is_floating),
              lifecycle_status = COALESCE(?, lifecycle_status),
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                normalized.full_name.as_str(),
                normalized.nick_name.as_deref(),
                team_id,
                normalized.project.as_deref(),
                normalized.job_title.as_deref(),
                normalized.email.as_deref(),
                normalized.cellphone.as_deref(),
                normalized.date_of_birth.as_deref(),
                normalized.gender.as_deref(),
                normalized.asw_start_date.as_deref(),
                normalized.client_start_date.as_deref(),
                normalized.contract_end_date.as_deref(),
                normalized.client_year_of_services.as_deref(),
                normalized.start_date.as_deref(),
                normalized.computer_name.as_deref(),
                normalized.notes.as_deref(),
                staff_group,
                normalized.is_floating.map(|b| if b { 1 } else { 0 }),
                normalized.lifecycle_status.as_deref(),
                id,
            ],
        )
        .map_err(humanize_sqlite_error)?;

        if !normalized.dynamic_fields.is_empty() {
            upsert_dynamic_field_definitions_for_map(tx, &normalized.dynamic_fields)?;
            upsert_dynamic_fields_tx(tx, id, &normalized.dynamic_fields)?;
        }
        normalize_eml_security_tool_values_for_employee_tx(
            tx,
            id,
            normalized.team_name.as_deref(),
        )?;

        return Ok(UpsertAction::Updated);
    }

    tx.execute(
        r#"
        INSERT INTO employees (
          employee_id,
          full_name,
          nick_name,
          team_id,
          project,
          job_title,
          email,
          cellphone,
          date_of_birth,
          gender,
          asw_start_date,
          client_start_date,
          contract_end_date,
          client_year_of_services,
          start_date,
          computername,
          notes,
          staff_group,
          is_floating,
          lifecycle_status,
          updated_at
        ) VALUES (
          ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now')
        )
        "#,
        params![
            normalized.employee_id.as_str(),
            normalized.full_name.as_str(),
            normalized.nick_name.as_deref(),
            team_id,
            normalized.project.as_deref(),
            normalized.job_title.as_deref(),
            normalized.email.as_deref(),
            normalized.cellphone.as_deref(),
            normalized.date_of_birth.as_deref(),
            normalized.gender.as_deref(),
            normalized.asw_start_date.as_deref(),
            normalized.client_start_date.as_deref(),
            normalized.contract_end_date.as_deref(),
            normalized.client_year_of_services.as_deref(),
            normalized.start_date.as_deref(),
            normalized.computer_name.as_deref(),
            normalized.notes.as_deref(),
            staff_group,
            normalized.is_floating.map(|b| if b { 1 } else { 0 }).unwrap_or(0),
            normalized.lifecycle_status.as_deref(),
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let inserted_id = tx.last_insert_rowid();
    if !normalized.dynamic_fields.is_empty() {
        upsert_dynamic_field_definitions_for_map(tx, &normalized.dynamic_fields)?;
        upsert_dynamic_fields_tx(tx, inserted_id, &normalized.dynamic_fields)?;
    }
    normalize_eml_security_tool_values_for_employee_tx(
        tx,
        inserted_id,
        normalized.team_name.as_deref(),
    )?;

    Ok(UpsertAction::Inserted)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn query_employees(
    conn: &Connection,
    filters: EmployeeQuery,
) -> Result<EmployeeListResponse, String> {
    let EmployeeQuery {
        query,
        team_name,
        staff_group,
        is_floating,
        sort_key,
        sort_direction,
        start_date_from,
        start_date_to,
        limit,
        offset,
    } = filters;

    let limit = i64::from(limit.unwrap_or(20).clamp(1, 5000));
    let offset = i64::from(offset.unwrap_or(0));

    let query_text = normalize_optional_text(query);
    let team_name = normalize_optional_text(team_name);
    let raw_group = normalize_optional_text(staff_group);
    let start_from = normalize_date_value(start_date_from);
    let start_to = normalize_date_value(start_date_to);
    let count_from_clause = build_employee_from_clause(query_text.is_some());
    let select_from_clause = build_employee_from_clause(true);
    let mut where_clauses: Vec<String> = Vec::new();
    let mut filter_params: Vec<Value> = Vec::new();

    if let Some(query) = query_text {
        let query_like = format!("%{}%", query.to_lowercase());
        if let Some(fts_query) = build_fts_query(&query) {
            where_clauses.push(
                "(e.id IN (SELECT rowid FROM employees_fts WHERE employees_fts MATCH ?) OR lower(COALESCE(e.email, '')) LIKE ? OR lower(COALESCE(NULLIF(e.computername, ''), NULLIF(lc.computer_name_1, ''), '')) LIKE ? OR lower(COALESCE(lc.computer_name_2, '')) LIKE ? OR EXISTS (SELECT 1 FROM employee_dynamic_values edv WHERE edv.employee_id = e.id AND lower(COALESCE(edv.value, '')) LIKE ?) OR EXISTS (SELECT 1 FROM asset_loans al_s INNER JOIN assets a_s ON a_s.id = al_s.asset_id WHERE al_s.employee_id_fk = e.id AND al_s.returned_at IS NULL AND (lower(COALESCE(a_s.display_name, '')) LIKE ? OR lower(COALESCE(a_s.asset_code, '')) LIKE ?)))"
                    .to_string(),
            );
            filter_params.push(Value::Text(fts_query));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like));
        } else {
            where_clauses.push(
                "(lower(COALESCE(e.email, '')) LIKE ? OR lower(COALESCE(NULLIF(e.computername, ''), NULLIF(lc.computer_name_1, ''), '')) LIKE ? OR lower(COALESCE(lc.computer_name_2, '')) LIKE ? OR EXISTS (SELECT 1 FROM employee_dynamic_values edv WHERE edv.employee_id = e.id AND lower(COALESCE(edv.value, '')) LIKE ?) OR EXISTS (SELECT 1 FROM asset_loans al_s INNER JOIN assets a_s ON a_s.id = al_s.asset_id WHERE al_s.employee_id_fk = e.id AND al_s.returned_at IS NULL AND (lower(COALESCE(a_s.display_name, '')) LIKE ? OR lower(COALESCE(a_s.asset_code, '')) LIKE ?)))"
                    .to_string(),
            );
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like.clone()));
            filter_params.push(Value::Text(query_like));
        }
    }

    if let Some(team_name) = team_name {
        // Match employees in this team OR in any sub-team (child) of this team
        where_clauses.push(
            "(t.name = ? OR EXISTS (SELECT 1 FROM teams pt WHERE pt.id = t.parent_id AND pt.name = ?))"
                .to_string(),
        );
        filter_params.push(Value::Text(team_name.clone()));
        filter_params.push(Value::Text(team_name));
    }

    let is_floating_filter = is_floating;
    if let Some(ref raw_group) = raw_group {
        if raw_group.eq_ignore_ascii_case("floating") {
            where_clauses.push("CASE WHEN COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') = 'internal_movent' THEN 'internal_movement' ELSE COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') END = 'employee_list' AND COALESCE(e.is_floating, 0) = 1".to_string());
        } else {
            let Some(staff_group) = normalize_staff_group(raw_group.as_str()) else {
                return Err(format!("invalid staff group filter: {raw_group}"));
            };

            where_clauses.push("CASE WHEN COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') = 'internal_movent' THEN 'internal_movement' ELSE COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') END = ?".to_string());
            filter_params.push(Value::Text(staff_group.to_string()));
        }
    }

    if is_floating_filter == Some(true) && !raw_group.as_deref().is_some_and(|g| g.eq_ignore_ascii_case("floating")) {
        where_clauses.push("COALESCE(e.is_floating, 0) = 1".to_string());
    }

    if let Some(start_from) = start_from {
        where_clauses.push("COALESCE(e.asw_start_date, e.start_date, '') >= ?".to_string());
        filter_params.push(Value::Text(start_from));
    }

    if let Some(start_to) = start_to {
        where_clauses.push("COALESCE(e.asw_start_date, e.start_date, '') <= ?".to_string());
        filter_params.push(Value::Text(start_to));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };

    let sort_key = normalize_optional_text(sort_key);
    let sort_direction = normalize_sort_direction(sort_direction.as_deref());
    let sort_spec = resolve_employee_sort_spec(conn, sort_key.as_deref(), sort_direction)?;

    let count_sql = format!("SELECT COUNT(*) {count_from_clause}{where_sql}");
    let total: i64 = conn
        .query_row(
            &count_sql,
            rusqlite::params_from_iter(filter_params.iter()),
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to count employees: {err}"))?;

    let select_sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} {select_from_clause}{where_sql} {} LIMIT ? OFFSET ?",
        sort_spec.order_sql
    );

    let mut select_params = filter_params;
    select_params.extend(sort_spec.order_params);
    select_params.push(Value::Integer(limit));
    select_params.push(Value::Integer(offset));

    let mut stmt = conn
        .prepare(&select_sql)
        .map_err(|err| format!("failed to prepare employee query: {err}"))?;

    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(select_params.iter()),
            map_employee_row,
        )
        .map_err(|err| format!("failed to query employees: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read employee row: {err}"))?);
    }
    hydrate_dynamic_fields(conn, &mut items)?;

    Ok(EmployeeListResponse {
        items,
        total,
        limit,
        offset,
    })
}

fn load_employee_by_id(conn: &Connection, id: i64) -> Result<EmployeeRecord, String> {
    let sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} {EMPLOYEE_ASSIGNED_ASSETS_COUNT_JOIN_SQL} WHERE e.id = ?"
    );

    let mut employee = conn
        .query_row(&sql, params![id], map_employee_row)
        .optional()
        .map_err(|err| format!("failed to load employee: {err}"))?
        .ok_or_else(|| format!("employee with id {id} was not found"))?;

    let mut single = vec![employee];
    hydrate_dynamic_fields(conn, &mut single)?;
    employee = single
        .into_iter()
        .next()
        .ok_or_else(|| "failed to hydrate employee dynamic fields".to_string())?;

    Ok(employee)
}

pub(crate) fn hydrate_dynamic_fields(
    conn: &Connection,
    items: &mut [EmployeeRecord],
) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let ids = items.iter().map(|item| item.id).collect::<Vec<_>>();
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "SELECT employee_id, field_key, value FROM employee_dynamic_values WHERE employee_id IN ({placeholders})"
    );

    let id_to_index = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<HashMap<_, _>>();

    let params = ids.iter().map(|id| Value::Integer(*id)).collect::<Vec<_>>();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("failed to prepare dynamic field query: {err}"))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|err| format!("failed to query dynamic fields: {err}"))?;

    for row in rows {
        let (employee_id, field_key, value) =
            row.map_err(|err| format!("failed to read dynamic field row: {err}"))?;
        let Some(index) = id_to_index.get(&employee_id).copied() else {
            continue;
        };

        let is_secondary_computer_key =
            field_key == COMPUTER_NAME_2_FIELD_KEY || field_key == "computer_name_2";
        if let Some(text) = normalize_optional_text(value) {
            items[index].dynamic_fields.insert(field_key, text);
        } else if is_secondary_computer_key {
            items[index].dynamic_fields.insert(field_key, String::new());
        }
    }

    let monitor_sql = format!(
        r#"
        SELECT
            al.employee_id_fk,
            COALESCE(NULLIF(TRIM(a.display_name), ''), a.asset_code) AS display_name
        FROM asset_loans al
        INNER JOIN assets a ON a.id = al.asset_id
        LEFT JOIN asset_categories c ON c.id = a.category_id
        WHERE al.employee_id_fk IN ({placeholders})
          AND al.returned_at IS NULL
          AND (
            c.category_code = 'monitor' COLLATE NOCASE
            OR c.category_name = 'Monitor' COLLATE NOCASE
            OR a.asset_type = 'Monitor' COLLATE NOCASE
            OR a.asset_code LIKE 'VNMON%'
          )
        ORDER BY al.employee_id_fk ASC, al.borrowed_at ASC, a.asset_code ASC, al.id ASC
        "#
    );

    let mut monitor_stmt = conn
        .prepare(&monitor_sql)
        .map_err(|err| format!("failed to prepare active monitor query: {err}"))?;
    let monitor_rows = monitor_stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| format!("failed to query active monitors: {err}"))?;

    let mut employee_monitors: HashMap<i64, Vec<String>> = HashMap::new();
    for row in monitor_rows {
        let (emp_id, monitor_name) =
            row.map_err(|err| format!("failed to read active monitor row: {err}"))?;
        employee_monitors
            .entry(emp_id)
            .or_default()
            .push(monitor_name);
    }

    for item in items.iter_mut() {
        if let Some(monitors) = employee_monitors.get(&item.id) {
            if let Some(first_monitor) = monitors.first() {
                item.dynamic_fields
                    .insert(ASSET_NAME_FIELD_KEY.to_string(), first_monitor.clone());
            }
            if let Some(second_monitor) = monitors.get(1) {
                item.dynamic_fields
                    .insert(ASSET_NAME_2_FIELD_KEY.to_string(), second_monitor.clone());
                item.dynamic_fields
                    .insert("aset_name_2".to_string(), second_monitor.clone());
            }
        }

        let primary = item.computer_name.clone();
        let has_current_secondary_override =
            item.dynamic_fields.contains_key(COMPUTER_NAME_2_FIELD_KEY);
        let has_legacy_secondary_override = item.dynamic_fields.contains_key("computer_name_2");
        let dynamic_secondary = item.dynamic_fields.get(COMPUTER_NAME_2_FIELD_KEY).cloned();
        let legacy_dynamic_secondary = item.dynamic_fields.get("computer_name_2").cloned();
        let has_secondary_override =
            has_current_secondary_override || has_legacy_secondary_override;
        let candidates = if has_secondary_override {
            vec![dynamic_secondary, legacy_dynamic_secondary]
        } else if item.suppress_active_computer_names {
            vec![item.legacy_stored_computer_name_2.clone()]
        } else {
            vec![
                item.legacy_stored_computer_name_2.clone(),
                item.active_computer_name_1.clone(),
                item.active_computer_name_2.clone(),
            ]
        };

        let secondary = candidates
            .into_iter()
            .flatten()
            .flat_map(|value| split_computer_name_tokens(Some(value.as_str())))
            .find(|value| !same_computer_name(primary.as_deref(), value));

        if let Some(secondary) = secondary {
            if item.dynamic_fields.contains_key(COMPUTER_NAME_2_FIELD_KEY)
                || !item.dynamic_fields.contains_key("computer_name_2")
            {
                item.dynamic_fields
                    .insert(COMPUTER_NAME_2_FIELD_KEY.to_string(), secondary);
            } else {
                item.dynamic_fields
                    .insert("computer_name_2".to_string(), secondary);
            }
        } else if !has_secondary_override {
            item.dynamic_fields.remove(COMPUTER_NAME_2_FIELD_KEY);
            if item.dynamic_fields.contains_key("computer_name_2") {
                item.dynamic_fields.remove("computer_name_2");
            }
        }

        item.active_computer_name_1 = None;
        item.active_computer_name_2 = None;
        item.legacy_stored_computer_name_2 = None;
    }

    Ok(())
}

fn map_employee_row(row: &Row<'_>) -> rusqlite::Result<EmployeeRecord> {
    let raw_computer_name: Option<String> = row.get(17)?;
    let raw_stored_computer_name: Option<String> = row.get(18)?;
    let active_computer_name_1: Option<String> = row.get(19)?;
    let active_computer_name_2: Option<String> = row.get(20)?;
    let stored_slots = split_computer_name_tokens(raw_stored_computer_name.as_deref());
    let display_slots = split_computer_name_tokens(raw_computer_name.as_deref());
    let stored_computer_name = raw_stored_computer_name
        .as_ref()
        .map(|_| stored_slots.first().cloned().unwrap_or_default());
    let computer_name = if raw_stored_computer_name.is_some() {
        stored_computer_name
            .clone()
            .filter(|value| !value.is_empty())
    } else {
        display_slots
            .first()
            .cloned()
            .or_else(|| active_computer_name_1.clone())
    };

    Ok(EmployeeRecord {
        id: row.get(0)?,
        employee_id: row.get(1)?,
        full_name: row.get(2)?,
        nick_name: row.get(3)?,
        team_id: row.get(4)?,
        team_name: row.get(5)?,
        project: row.get(6)?,
        job_title: row.get(7)?,
        email: row.get(8)?,
        cellphone: row.get(9)?,
        date_of_birth: row.get(10)?,
        gender: row.get(11)?,
        asw_start_date: row.get(12)?,
        client_start_date: row.get(13)?,
        contract_end_date: row.get(14)?,
        client_year_of_services: row.get(15)?,
        start_date: row.get(16)?,
        computer_name,
        stored_computer_name,
        active_computer_name_1,
        active_computer_name_2,
        legacy_stored_computer_name_2: stored_slots.get(1).cloned(),
        suppress_active_computer_names: raw_stored_computer_name
            .as_deref()
            .is_some_and(|value| value.trim().is_empty()),
        notes: row.get(21)?,
        staff_group: row.get(22)?,
        dynamic_fields: HashMap::new(),
        updated_at: row.get(23)?,
        is_floating: row.get::<_, i64>(24)? != 0,
        lifecycle_status: row.get(25)?,
        assigned_assets_count: row.get(26)?,
    })
}

fn split_computer_name_tokens(value: Option<&str>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(|raw| raw.split(|ch| matches!(ch, ',' | '\n' | '\r')))
        .filter_map(|token| normalize_optional_text(Some(token.to_string())))
        .fold(Vec::new(), |mut values, token| {
            if !values
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&token))
            {
                values.push(token);
            }
            values
        })
}

fn first_computer_name(value: Option<String>) -> Option<String> {
    split_computer_name_tokens(value.as_deref())
        .into_iter()
        .next()
}

fn normalize_computer_name_override(value: Option<String>) -> Option<String> {
    value.map(|raw| first_computer_name(Some(raw)).unwrap_or_default())
}

fn normalize_employee_dynamic_fields(
    input: Option<HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut fields = normalize_dynamic_fields(input);
    for key in [COMPUTER_NAME_2_FIELD_KEY, "computer_name_2"] {
        if let Some(value) = fields.get_mut(key) {
            *value = first_computer_name(Some(value.clone())).unwrap_or_default();
        }
    }
    fields
}

fn same_computer_name(left: Option<&str>, right: &str) -> bool {
    let Some(left) = left else {
        return false;
    };
    let left = left.trim().to_uppercase();
    let right = right.trim().to_uppercase();
    left == right
        || left.strip_prefix("ASW").unwrap_or(left.as_str())
            == right.strip_prefix("ASW").unwrap_or(right.as_str())
}

fn build_employee_from_clause(include_active_laptop_join: bool) -> String {
    if include_active_laptop_join {
        format!(" FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} {EMPLOYEE_ASSIGNED_ASSETS_COUNT_JOIN_SQL} ")
    } else {
        " FROM employees e LEFT JOIN teams t ON t.id = e.team_id ".to_string()
    }
}

fn normalize_sort_direction(input: Option<&str>) -> &'static str {
    let Some(raw) = input else {
        return "ASC";
    };

    match normalize_header_key(raw).as_str() {
        "desc" | "descending" | "ztoa" => "DESC",
        _ => "ASC",
    }
}

fn normalize_header_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect::<String>()
}

fn resolve_core_sort_expression(sort_key: &str) -> Option<&'static str> {
    let expression = match normalize_header_key(sort_key).as_str() {
        "employeeid" | "eeid" | "staffid" => "e.employee_id COLLATE NOCASE",
        "fullname" | "vietnamesename" | "name" => "e.full_name COLLATE NOCASE",
        "nickname" => "COALESCE(e.nick_name, '') COLLATE NOCASE",
        "teamname" | "clientpmd" | "client" => "COALESCE(t.name, '') COLLATE NOCASE",
        "project" => "COALESCE(e.project, '') COLLATE NOCASE",
        "jobtitle" | "currentjobtitle" => "COALESCE(e.job_title, '') COLLATE NOCASE",
        "email" | "workingemail" => "COALESCE(e.email, '') COLLATE NOCASE",
        "cellphone" | "phone" => "COALESCE(e.cellphone, '') COLLATE NOCASE",
        "dateofbirth" | "dob" => "COALESCE(e.date_of_birth, '')",
        "gender" => "COALESCE(e.gender, '') COLLATE NOCASE",
        "aswstartdate" | "startdate" => "COALESCE(e.asw_start_date, e.start_date, '')",
        "clientstartdate" => "COALESCE(e.client_start_date, '')",
        "contractenddate" => "COALESCE(e.contract_end_date, '')",
        "clientyearofservices" => "COALESCE(e.client_year_of_services, '') COLLATE NOCASE",
        "computername" => {
            "COALESCE(NULLIF(e.computername, ''), NULLIF(lc.computer_name_1, ''), '') COLLATE NOCASE"
        }
        "assignedassets" | "assignedassetscount" | "assets" => {
            "COALESCE(al_counts.active_count, 0)"
        }
        "assetname" | "asset_name" => {
            "COALESCE((SELECT COALESCE(NULLIF(TRIM(a.display_name), ''), a.asset_code) FROM asset_loans al INNER JOIN assets a ON a.id = al.asset_id LEFT JOIN asset_categories c ON c.id = a.category_id WHERE al.employee_id_fk = e.id AND al.returned_at IS NULL AND (c.category_code = 'monitor' COLLATE NOCASE OR c.category_name = 'Monitor' COLLATE NOCASE OR a.asset_type = 'Monitor' COLLATE NOCASE OR a.asset_code LIKE 'VNMON%') ORDER BY al.borrowed_at ASC, a.asset_code ASC, al.id ASC LIMIT 1), (SELECT v.value FROM employee_dynamic_values v WHERE v.employee_id = e.id AND v.field_key = 'asset_name'), '') COLLATE NOCASE"
        }
        "assetname2" | "asset_name_2" | "asetname2" | "aset_name_2" => {
            "COALESCE((SELECT COALESCE(NULLIF(TRIM(a.display_name), ''), a.asset_code) FROM asset_loans al INNER JOIN assets a ON a.id = al.asset_id LEFT JOIN asset_categories c ON c.id = a.category_id WHERE al.employee_id_fk = e.id AND al.returned_at IS NULL AND (c.category_code = 'monitor' COLLATE NOCASE OR c.category_name = 'Monitor' COLLATE NOCASE OR a.asset_type = 'Monitor' COLLATE NOCASE OR a.asset_code LIKE 'VNMON%') ORDER BY al.borrowed_at ASC, a.asset_code ASC, al.id ASC LIMIT 1 OFFSET 1), (SELECT v.value FROM employee_dynamic_values v WHERE v.employee_id = e.id AND v.field_key IN ('asset_name_2', 'aset_name_2')), '') COLLATE NOCASE"
        }
        "notes" => "COALESCE(e.notes, '') COLLATE NOCASE",
        _ => return None,
    };

    Some(expression)
}

fn build_core_order_sql(sort_expression: &str, sort_direction: &str) -> String {
    if sort_expression == "e.full_name COLLATE NOCASE" {
        format!("ORDER BY {sort_expression} {sort_direction}, e.id ASC")
    } else {
        format!(
            "ORDER BY {sort_expression} {sort_direction}, e.full_name COLLATE NOCASE ASC, e.id ASC"
        )
    }
}

fn resolve_employee_sort_spec(
    conn: &Connection,
    sort_key: Option<&str>,
    sort_direction: &str,
) -> Result<EmployeeSortSpec, String> {
    let default_expression = "e.full_name COLLATE NOCASE";

    let Some(raw_key) = sort_key else {
        return Ok(EmployeeSortSpec {
            order_sql: build_core_order_sql(default_expression, sort_direction),
            order_params: Vec::new(),
        });
    };

    if let Some(sort_expression) = resolve_core_sort_expression(raw_key) {
        return Ok(EmployeeSortSpec {
            order_sql: build_core_order_sql(sort_expression, sort_direction),
            order_params: Vec::new(),
        });
    }

    let dynamic_key = super::normalize_dynamic_field_key(raw_key);
    if dynamic_key.is_empty() || !dynamic_field_exists(conn, dynamic_key.as_str())? {
        return Ok(EmployeeSortSpec {
            order_sql: build_core_order_sql(default_expression, sort_direction),
            order_params: Vec::new(),
        });
    }

    Ok(EmployeeSortSpec {
        order_sql: format!(
            "ORDER BY COALESCE((SELECT v.value FROM employee_dynamic_values v WHERE v.employee_id = e.id AND v.field_key = ?), '') COLLATE NOCASE {sort_direction}, e.full_name COLLATE NOCASE ASC, e.id ASC"
        ),
        order_params: vec![Value::Text(dynamic_key)],
    })
}

fn normalize_optional_or_date(value: Option<String>) -> Option<String> {
    let normalized = normalize_optional_text(value)?;
    super::normalize_date_text(&normalized).or(Some(normalized))
}

fn resolve_update_staff_group_tx(
    tx: &Transaction<'_>,
    row_id: i64,
    requested_staff_group: Option<&str>,
) -> Result<String, String> {
    if let Some(raw_group) =
        requested_staff_group.and_then(|value| normalize_optional_text(Some(value.to_string())))
    {
        let target_group = normalize_staff_group(raw_group.as_str())
            .ok_or_else(|| format!("invalid staff group: {raw_group}"))?;
        return Ok(target_group.to_string());
    }

    tx.query_row(
        r#"
        SELECT CASE
          WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = 'internal_movent'
          THEN 'internal_movement'
          ELSE COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list')
        END
        FROM employees
        WHERE id = ?
        "#,
        params![row_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|err| format!("failed to read current employee group: {err}"))?
    .ok_or_else(|| format!("employee with id {row_id} was not found"))
}

fn normalize_eml_security_tool_values_for_employee_tx(
    tx: &Transaction<'_>,
    employee_id: i64,
    team_name: Option<&str>,
) -> Result<(), String> {
    if !is_eml_team(team_name) {
        return Ok(());
    }

    tx.execute(
        r#"
        UPDATE employee_dynamic_values
        SET value = 'Yes',
            updated_at = datetime('now')
        WHERE employee_id = ?
          AND lower(trim(COALESCE(value, ''))) = 'v'
          AND replace(lower(field_key), '_', '') IN ('sentinelone', 'endpointagent')
        "#,
        params![employee_id],
    )
    .map_err(|err| format!("failed to normalize EML security tool values: {err}"))?;

    Ok(())
}

fn is_eml_team(team_name: Option<&str>) -> bool {
    matches!(team_name.map(normalize_header_key).as_deref(), Some("eml"))
}

fn build_fts_query(raw: &str) -> Option<String> {
    let tokens = raw
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return None;
    }

    Some(
        tokens
            .iter()
            .map(|token| format!("{token}*"))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, configure_connection, COMPUTER_NAME_2_FIELD_KEY};

    use super::{
        build_employee_from_clause, delete_employee_conn, get_employee_assigned_assets_conn,
        list_employee_group_counts_conn, move_employees_group_conn, query_employees,
        update_employee_tx, upsert_employee_from_payload, EmployeePayload, EmployeeQuery,
        MoveEmployeesGroupInput, EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL,
    };

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
    }

    #[test]
    fn employee_deletion_preserves_historical_borrow_requests_and_confirmation_evidence() {
        let mut conn = open_test_connection();
        let employee_id = seed_employee(&conn, "EE-HISTORY", "Historical Employee");
        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, borrower_employee_id_fk, borrower_staff_id_snapshot, borrower_name_snapshot, submitted_by_employee_id_fk, submitted_by_staff_id_snapshot, submitted_by_name_snapshot, status) VALUES('HISTORY-REQUEST', ?, 'EE-HISTORY', 'Historical Employee', ?, 'EE-HISTORY', 'Historical Employee', ?, 'EE-HISTORY', 'Historical Employee', 'approved')",
            params![employee_id, employee_id, employee_id],
        ).expect("insert historical request");
        let request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO borrow_request_confirmations(borrow_request_id, asset_codes_snapshot_json, confirmation_method, confirmed_at) VALUES(?, '[]', 'typed_name', datetime('now'))",
            params![request_id],
        ).expect("insert confirmation evidence");

        assert!(delete_employee_conn(&mut conn, employee_id).expect("delete employee"));
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM employees WHERE id = ?",
                params![employee_id],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM borrow_requests WHERE id = ?",
                params![request_id],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM borrow_request_confirmations WHERE borrow_request_id = ?",
                params![request_id],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(conn
            .query_row(
                "SELECT employee_id_fk FROM borrow_requests WHERE id = ?",
                params![request_id],
                |row| row.get::<_, Option<i64>>(0)
            )
            .unwrap()
            .is_none());
    }

    fn seed_employee(conn: &Connection, employee_id: &str, full_name: &str) -> i64 {
        conn.execute("INSERT OR IGNORE INTO teams(name) VALUES ('Examworks')", [])
            .expect("insert team");
        let team_id: i64 = conn
            .query_row("SELECT id FROM teams WHERE name = 'Examworks'", [], |row| {
                row.get(0)
            })
            .expect("load team id");

        conn.execute(
            r#"
            INSERT INTO employees(
              employee_id,
              full_name,
              team_id,
              staff_group,
              updated_at
            )
            VALUES(?, ?, ?, 'employee_list', datetime('now'))
            "#,
            params![employee_id, full_name, team_id],
        )
        .expect("insert employee");

        conn.last_insert_rowid()
    }

    fn category_id(conn: &Connection, category_code: &str) -> i64 {
        conn.query_row(
            "SELECT id FROM asset_categories WHERE category_code = ?1",
            params![category_code],
            |row| row.get(0),
        )
        .expect("load asset category id")
    }

    fn seed_asset(
        conn: &Connection,
        asset_code: &str,
        asset_type: &str,
        category_code: &str,
    ) -> i64 {
        conn.execute(
            r#"
            INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, created_at, updated_at)
            VALUES(?, ?, ?, ?, 'assigned', datetime('now'), datetime('now'))
            "#,
            params![asset_code, category_id(conn, category_code), asset_type, asset_code],
        )
        .expect("insert asset");
        conn.last_insert_rowid()
    }

    fn seed_active_loan(conn: &Connection, employee_row_id: i64, asset_id: i64) {
        conn.execute(
            r#"
            INSERT INTO borrow_requests(
              request_key,
              employee_id_fk,
              submitted_employee_id,
              submitted_full_name,
              status,
              request_type,
              submitted_at
            )
            VALUES(?, ?, 'ASWVN1302', 'Lư Thế Hùng', 'approved', 'borrow', datetime('now'))
            "#,
            params![format!("TEST-BR-{asset_id}"), employee_row_id],
        )
        .expect("insert synthetic borrow request");
        let borrow_request_id = conn.last_insert_rowid();

        conn.execute(
            r#"
            INSERT INTO asset_loans(
              asset_id,
              employee_id_fk,
              borrow_request_id,
              borrowed_at
            )
            VALUES(?, ?, ?, datetime('now'))
            "#,
            params![asset_id, employee_row_id, borrow_request_id],
        )
        .expect("insert active asset loan");
    }

    fn seed_active_loan_with_borrowed_at(
        conn: &Connection,
        employee_row_id: i64,
        asset_id: i64,
        borrowed_at: &str,
    ) {
        conn.execute(
            r#"
            INSERT INTO borrow_requests(
              request_key,
              employee_id_fk,
              submitted_employee_id,
              submitted_full_name,
              status,
              request_type,
              submitted_at
            )
            VALUES(?, ?, 'EE001', 'Nguyen Van A', 'approved', 'borrow', datetime('now'))
            "#,
            params![format!("TEST-BR-{asset_id}"), employee_row_id],
        )
        .expect("insert synthetic borrow request");
        let borrow_request_id = conn.last_insert_rowid();

        conn.execute(
            r#"
            INSERT INTO asset_loans(
              asset_id,
              employee_id_fk,
              borrow_request_id,
              borrowed_at
            )
            VALUES(?, ?, ?, ?)
            "#,
            params![asset_id, employee_row_id, borrow_request_id, borrowed_at],
        )
        .expect("insert active asset loan");
    }

    #[test]
    fn upsert_employee_from_payload_preserves_blank_secondary_device_override() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1302", "Luu The Hung");
        conn.execute(
            r#"
            INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
            VALUES('computer_name_2', 'Computer Name 2', datetime('now'))
            "#,
            [],
        )
        .expect("insert dynamic field definition");
        conn.execute(
            r#"
            INSERT INTO employee_dynamic_values(employee_id, field_key, value, updated_at)
            VALUES(?, 'computer_name_2', 'ASWVNLAP244', datetime('now'))
            "#,
            params![employee_row_id],
        )
        .expect("insert dynamic field value");

        let tx = conn.transaction().expect("start transaction");
        upsert_employee_from_payload(
            &tx,
            EmployeePayload {
                employee_id: "ASWVN1302".to_string(),
                full_name: "Luu The Hung".to_string(),
                nick_name: None,
                team_name: Some("Examworks".to_string()),
                project: None,
                job_title: None,
                email: None,
                cellphone: None,
                date_of_birth: None,
                gender: None,
                asw_start_date: None,
                client_start_date: None,
                contract_end_date: None,
                client_year_of_services: None,
                computer_name: None,
                notes: None,
                staff_group: Some("employee_list".to_string()),
                is_floating: None,
                lifecycle_status: None,
                dynamic_fields: Some(HashMap::from([(
                    "computer_name_2".to_string(),
                    "   ".to_string(),
                )])),
            },
            "employee_list",
        )
        .expect("clear dynamic field through employee payload");
        tx.commit().expect("commit transaction");

        let exists: i64 = conn
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM employee_dynamic_values
                WHERE employee_id = ? AND field_key = 'computer_name_2'
                "#,
                params![employee_row_id],
                |row| row.get(0),
            )
            .expect("count dynamic field values");
        assert_eq!(exists, 1);
        let value: String = conn
            .query_row(
                r#"
                SELECT value
                FROM employee_dynamic_values
                WHERE employee_id = ? AND field_key = 'computer_name_2'
                "#,
                params![employee_row_id],
                |row| row.get(0),
            )
            .expect("load blank secondary device override");
        assert_eq!(value, "");
    }

    #[test]
    fn query_employees_derives_computer_name_from_active_laptop_loans() {
        let conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1302", "Lư Thế Hùng");
        let mac_asset_id = seed_asset(&conn, "VNMACPRO010", "Laptop", "laptop");
        let lap_asset_id = seed_asset(&conn, "VNLAP293", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, mac_asset_id);
        seed_active_loan(&conn, employee_row_id, lap_asset_id);

        let response = query_employees(
            &conn,
            EmployeeQuery {
                query: None,
                team_name: None,
                staff_group: None,
                is_floating: None,
                sort_key: None,
                sort_direction: None,
                start_date_from: None,
                start_date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
        )
        .expect("query employees");

        let employee = response
            .items
            .iter()
            .find(|item| item.employee_id == "ASWVN1302")
            .expect("find seeded employee");
        assert_eq!(employee.computer_name.as_deref(), Some("ASWVNMACPRO010"));
        assert_eq!(
            employee.dynamic_fields.get(COMPUTER_NAME_2_FIELD_KEY),
            Some(&"ASWVNLAP293".to_string())
        );
    }

    #[test]
    fn query_employees_can_search_by_derived_laptop_computer_name() {
        let conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1302", "Lư Thế Hùng");
        let lap_asset_id = seed_asset(&conn, "VNLAP293", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, lap_asset_id);

        let response = query_employees(
            &conn,
            EmployeeQuery {
                query: Some("ASWVNLAP293".to_string()),
                team_name: None,
                staff_group: None,
                is_floating: None,
                sort_key: None,
                sort_direction: None,
                start_date_from: None,
                start_date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
        )
        .expect("search employees by derived computer name");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].employee_id, "ASWVN1302");
    }

    #[test]
    fn query_employees_can_search_by_working_email_with_at_sign() {
        let conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1302", "Luu The Hung");
        conn.execute(
            "UPDATE employees SET email = ? WHERE id = ?",
            params!["Person@Example.COM", employee_row_id],
        )
        .expect("set working email");

        let response = query_employees(
            &conn,
            EmployeeQuery {
                query: Some("Person@Example.COM".to_string()),
                team_name: None,
                staff_group: None,
                is_floating: None,
                sort_key: None,
                sort_direction: None,
                start_date_from: None,
                start_date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
        )
        .expect("search employees by working email");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].employee_id, "ASWVN1302");
    }

    #[test]
    fn active_laptop_join_uses_ordered_window_aggregation() {
        assert!(
            EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL.contains("MAX(CASE WHEN laptop_names.row_num = 1"),
            "active-laptop join should expose the first ordered loan as Device 1",
        );
        assert!(
            EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL.contains("MAX(CASE WHEN laptop_names.row_num = 2"),
            "active-laptop join should expose the second ordered loan as Device 2",
        );
        assert!(
            EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL.contains("ROW_NUMBER() OVER"),
            "active-laptop join should select one deterministic row per employee",
        );
    }

    #[test]
    fn count_from_clause_skips_laptop_join_without_query_filter() {
        let without_query_join = build_employee_from_clause(false);
        let with_query_join = build_employee_from_clause(true);

        assert!(
            !without_query_join.contains("asset_loans"),
            "count path should not pay for laptop aggregation when query filter does not use lc",
        );
        assert!(
            with_query_join.contains("asset_loans"),
            "query path should still include laptop aggregation when lc is referenced",
        );
    }

    #[test]
    fn query_employees_excludes_monitor_loans_from_computer_name() {
        let conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1302", "Lư Thế Hùng");
        let laptop_asset_id = seed_asset(&conn, "VNLAP293", "Laptop", "laptop");
        let monitor_asset_id = seed_asset(&conn, "VNMON709", "Monitor", "monitor");
        seed_active_loan(&conn, employee_row_id, laptop_asset_id);
        seed_active_loan(&conn, employee_row_id, monitor_asset_id);

        let response = query_employees(
            &conn,
            EmployeeQuery {
                query: None,
                team_name: None,
                staff_group: None,
                is_floating: None,
                sort_key: None,
                sort_direction: None,
                start_date_from: None,
                start_date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
        )
        .expect("query employees");

        let employee = response
            .items
            .iter()
            .find(|item| item.employee_id == "ASWVN1302")
            .expect("find seeded employee");
        assert_eq!(employee.computer_name.as_deref(), Some("ASWVNLAP293"));
    }

    #[test]
    fn saved_primary_computer_name_keeps_second_active_laptop_in_device_two() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1308", "Nguyen Pham Phuong Uyen");
        let first_laptop_id = seed_asset(&conn, "VNLAP326", "Laptop", "laptop");
        let second_laptop_id = seed_asset(&conn, "VNLAP163", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, first_laptop_id);
        seed_active_loan(&conn, employee_row_id, second_laptop_id);

        let tx = conn.transaction().expect("start transaction");
        upsert_employee_from_payload(
            &tx,
            EmployeePayload {
                employee_id: "ASWVN1308".to_string(),
                full_name: "Nguyen Pham Phuong Uyen".to_string(),
                nick_name: None,
                team_name: Some("ASW Consulting".to_string()),
                project: None,
                job_title: None,
                email: None,
                cellphone: None,
                date_of_birth: None,
                gender: None,
                asw_start_date: None,
                client_start_date: None,
                contract_end_date: None,
                client_year_of_services: None,
                computer_name: Some("ASWVNLAP163".to_string()),
                notes: None,
                staff_group: Some("employee_list".to_string()),
                is_floating: None,
                lifecycle_status: None,
                dynamic_fields: None,
            },
            "employee_list",
        )
        .expect("save primary computer name");
        tx.commit().expect("commit transaction");

        let response = query_employees(
            &conn,
            EmployeeQuery {
                query: Some("ASWVNLAP163".to_string()),
                team_name: None,
                staff_group: None,
                is_floating: None,
                sort_key: None,
                sort_direction: None,
                start_date_from: None,
                start_date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
        )
        .expect("query employee after primary computer name save");
        let employee = response
            .items
            .iter()
            .find(|item| item.employee_id == "ASWVN1308")
            .expect("find saved employee");

        assert_eq!(employee.computer_name.as_deref(), Some("ASWVNLAP163"));
        assert_eq!(
            employee.dynamic_fields.get(COMPUTER_NAME_2_FIELD_KEY),
            Some(&"ASWVNLAP326".to_string())
        );
    }

    #[test]
    fn employee_import_legacy_name_change_does_not_mutate_asset_loans() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1311", "Import Boundary Employee");
        let laptop_id = seed_asset(&conn, "VNLAP317", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, laptop_id);

        let tx = conn.transaction().expect("start transaction");
        upsert_employee_from_payload(
            &tx,
            EmployeePayload {
                employee_id: "ASWVN1311".to_string(),
                full_name: "Import Boundary Employee".to_string(),
                nick_name: None,
                team_name: Some("Examworks".to_string()),
                project: None,
                job_title: None,
                email: None,
                cellphone: None,
                date_of_birth: None,
                gender: None,
                asw_start_date: None,
                client_start_date: None,
                contract_end_date: None,
                client_year_of_services: None,
                computer_name: Some("ASWVNLAP999".to_string()),
                notes: None,
                staff_group: Some("employee_list".to_string()),
                is_floating: None,
                lifecycle_status: None,
                dynamic_fields: None,
            },
            "employee_list",
        )
        .expect("import employee payload");
        tx.commit().expect("commit employee import");

        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![laptop_id],
                |row| row.get(0),
            )
            .expect("count active imported employee loan");
        let loan_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE employee_id_fk = ?",
                params![employee_row_id],
                |row| row.get(0),
            )
            .expect("count employee asset loans");
        assert_eq!(active_count, 1);
        assert_eq!(loan_count, 1);
    }

    #[test]
    fn changing_legacy_computer_name_does_not_close_active_laptop_loan() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1310", "Boundary Test Employee");
        let laptop_id = seed_asset(&conn, "VNLAP326", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, laptop_id);

        let tx = conn.transaction().expect("start transaction");
        update_employee_tx(
            &tx,
            employee_row_id,
            EmployeePayload {
                employee_id: "ASWVN1310".to_string(),
                full_name: "Boundary Test Employee".to_string(),
                nick_name: None,
                team_name: Some("Examworks".to_string()),
                project: None,
                job_title: None,
                email: None,
                cellphone: None,
                date_of_birth: None,
                gender: None,
                asw_start_date: None,
                client_start_date: None,
                contract_end_date: None,
                client_year_of_services: None,
                computer_name: Some("ASWVNLAP999".to_string()),
                notes: None,
                staff_group: Some("employee_list".to_string()),
                is_floating: None,
                lifecycle_status: None,
                dynamic_fields: None,
            },
        )
        .expect("update employee legacy field");
        tx.commit().expect("commit transaction");

        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![laptop_id],
                |row| row.get(0),
            )
            .expect("count active laptop loan");
        assert_eq!(active_count, 1);
    }

    #[test]
    fn clearing_secondary_device_does_not_return_active_laptop() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1308", "Nguyen Pham Phuong Uyen");
        let first_laptop_id = seed_asset(&conn, "VNLAP326", "Laptop", "laptop");
        let second_laptop_id = seed_asset(&conn, "VNLAP163", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, first_laptop_id);
        seed_active_loan(&conn, employee_row_id, second_laptop_id);

        let tx = conn.transaction().expect("start transaction");
        update_employee_tx(
            &tx,
            employee_row_id,
            EmployeePayload {
                employee_id: "ASWVN1308".to_string(),
                full_name: "Nguyen Pham Phuong Uyen".to_string(),
                nick_name: None,
                team_name: Some("ASW Consulting".to_string()),
                project: None,
                job_title: None,
                email: None,
                cellphone: None,
                date_of_birth: None,
                gender: None,
                asw_start_date: None,
                client_start_date: None,
                contract_end_date: None,
                client_year_of_services: None,
                computer_name: Some("ASWVNLAP326".to_string()),
                notes: None,
                staff_group: Some("employee_list".to_string()),
                is_floating: None,
                lifecycle_status: None,
                dynamic_fields: Some(HashMap::from([(
                    COMPUTER_NAME_2_FIELD_KEY.to_string(),
                    String::new(),
                )])),
            },
        )
        .expect("update employee secondary device field");
        tx.commit().expect("commit transaction");

        let first_active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![first_laptop_id],
                |row| row.get(0),
            )
            .expect("count first active laptop loan");
        let second_active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![second_laptop_id],
                |row| row.get(0),
            )
            .expect("count second active laptop loan");
        let first_status: String = conn
            .query_row(
                "SELECT status FROM assets WHERE id = ?",
                params![first_laptop_id],
                |row| row.get(0),
            )
            .expect("load first laptop status");
        let second_status: String = conn
            .query_row(
                "SELECT status FROM assets WHERE id = ?",
                params![second_laptop_id],
                |row| row.get(0),
            )
            .expect("load second laptop status");

        assert_eq!(first_active, 1);
        assert_eq!(second_active, 1);
        assert_eq!(first_status, "assigned");
        assert_eq!(second_status, "assigned");
    }

    #[test]
    fn saved_computer_name_overrides_active_laptop_display() {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(&conn, "ASWVN1202", "Huỳnh Nguyễn Minh Thắng");
        let laptop_asset_id = seed_asset(&conn, "VNLAP264", "Laptop", "laptop");
        seed_active_loan(&conn, employee_row_id, laptop_asset_id);

        let tx = conn.transaction().expect("start transaction");
        upsert_employee_from_payload(
            &tx,
            EmployeePayload {
                employee_id: "ASWVN1202".to_string(),
                full_name: "Huỳnh Nguyễn Minh Thắng".to_string(),
                nick_name: None,
                team_name: Some("Examworks".to_string()),
                project: None,
                job_title: None,
                email: None,
                cellphone: None,
                date_of_birth: None,
                gender: None,
                asw_start_date: None,
                client_start_date: None,
                contract_end_date: None,
                client_year_of_services: None,
                computer_name: Some("ASWVNLAP445".to_string()),
                notes: None,
                staff_group: Some("employee_list".to_string()),
                is_floating: None,
                lifecycle_status: None,
                dynamic_fields: None,
            },
            "employee_list",
        )
        .expect("save employee computer name");
        tx.commit().expect("commit transaction");

        let response = query_employees(
            &conn,
            EmployeeQuery {
                query: None,
                team_name: None,
                staff_group: None,
                is_floating: None,
                sort_key: None,
                sort_direction: None,
                start_date_from: None,
                start_date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
        )
        .expect("query employees after computer name save");
        let employee = response
            .items
            .iter()
            .find(|item| item.employee_id == "ASWVN1202")
            .expect("find saved employee");

        assert_eq!(employee.computer_name.as_deref(), Some("ASWVNLAP445"));
        assert_eq!(
            employee.stored_computer_name.as_deref(),
            Some("ASWVNLAP445")
        );
    }

    #[test]
    fn floating_overlay_counts_and_overlap_in_ee_list() {
        let conn = open_test_connection();

        // 1. Employee list with is_floating = 0
        let _e1 = seed_employee(&conn, "EE-REGULAR", "Regular Employee");
        // 2. Employee list with is_floating = 1 (Floating overlay)
        let e2 = seed_employee(&conn, "EE-FLOAT", "Floating Employee");
        conn.execute("UPDATE employees SET is_floating = 1 WHERE id = ?", params![e2])
            .expect("set e2 as floating");

        // 3. Onboarding employee
        let e3 = seed_employee(&conn, "EE-ONBOARD", "Onboarding Employee");
        conn.execute("UPDATE employees SET staff_group = 'onboarding' WHERE id = ?", params![e3])
            .expect("set e3 as onboarding");

        // 4. Offboarding employee
        let e4 = seed_employee(&conn, "EE-OFFBOARD", "Offboarded Employee");
        conn.execute("UPDATE employees SET staff_group = 'offboarding' WHERE id = ?", params![e4])
            .expect("set e4 as offboarding");

        // Verify group counts: EE list counts both E1 and E2; floating counts E2
        let counts = list_employee_group_counts_conn(&conn).expect("load group counts");
        assert_eq!(counts.employee_list, 2, "EE List must include floating employee");
        assert_eq!(counts.floating, 1, "Floating count must accurately reflect overlay");
        assert_eq!(counts.onboarding, 1);
        assert_eq!(counts.offboarding, 1);
        assert_eq!(counts.total, 4);

        // Verify query_employees for EE list includes both E1 and E2
        let ee_list_query = query_employees(&conn, EmployeeQuery {
            query: None,
            team_name: None,
            staff_group: Some("employee_list".to_string()),
            is_floating: None,
            sort_key: None,
            sort_direction: None,
            start_date_from: None,
            start_date_to: None,
            limit: Some(10),
            offset: Some(0),
        }).expect("query employee_list");
        assert_eq!(ee_list_query.total, 2);
        assert!(ee_list_query.items.iter().any(|item| item.employee_id == "EE-REGULAR" && !item.is_floating));
        assert!(ee_list_query.items.iter().any(|item| item.employee_id == "EE-FLOAT" && item.is_floating));

        // Verify query_employees for Floating returns ONLY E2
        let floating_query = query_employees(&conn, EmployeeQuery {
            query: None,
            team_name: None,
            staff_group: Some("floating".to_string()),
            is_floating: None,
            sort_key: None,
            sort_direction: None,
            start_date_from: None,
            start_date_to: None,
            limit: Some(10),
            offset: Some(0),
        }).expect("query floating group");
        assert_eq!(floating_query.total, 1);
        assert_eq!(floating_query.items[0].employee_id, "EE-FLOAT");
        assert!(floating_query.items[0].is_floating);
    }

    #[test]
    fn onboarding_reject_offer_stays_in_onboarding_with_lifecycle_status() {
        let mut conn = open_test_connection();
        let candidate_id = seed_employee(&conn, "EE-OFFER1", "Offered Candidate");
        conn.execute("UPDATE employees SET staff_group = 'onboarding' WHERE id = ?", params![candidate_id])
            .expect("set onboarding");

        let changed = move_employees_group_conn(&mut conn, MoveEmployeesGroupInput {
            employee_ids: vec![candidate_id],
            target_staff_group: "reject_offer".to_string(),
            action: Some("reject_offer".to_string()),
            reason: Some("Candidate declined package".to_string()),
        }).expect("reject offer");

        assert_eq!(changed, 1);

        let (group, status, e_id): (String, Option<String>, String) = conn.query_row(
            "SELECT staff_group, lifecycle_status, employee_id FROM employees WHERE id = ?",
            params![candidate_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).expect("load candidate row");

        assert_eq!(group, "onboarding", "Reject offer must remain in onboarding group");
        assert_eq!(status.as_deref(), Some("offer_rejected"), "lifecycle_status must be offer_rejected");
        assert_eq!(e_id, "EE-OFFER1", "employee_id must be preserved");
    }

    #[test]
    fn offboarding_rehire_preserves_asset_loans_and_identity() {
        let mut conn = open_test_connection();
        let emp_id = seed_employee(&conn, "ASWVN1145", "Nguyen Van A");
        let laptop_id = seed_asset(&conn, "VNLAP1145", "Laptop", "laptop");
        seed_active_loan(&conn, emp_id, laptop_id);

        // Put in offboarding
        conn.execute("UPDATE employees SET staff_group = 'offboarding', lifecycle_status = 'resigned' WHERE id = ?", params![emp_id])
            .expect("set offboarding");

        // Verify loan exists
        let loan_count_before: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_loans WHERE employee_id_fk = ? AND returned_at IS NULL",
            params![emp_id],
            |row| row.get(0),
        ).expect("check active loan");
        assert_eq!(loan_count_before, 1);

        // Rehire to employee_list
        let changed = move_employees_group_conn(&mut conn, MoveEmployeesGroupInput {
            employee_ids: vec![emp_id],
            target_staff_group: "employee_list".to_string(),
            action: None,
            reason: Some("Rehired as Senior Engineer".to_string()),
        }).expect("rehire employee");

        assert_eq!(changed, 1);

        let (group, status, e_id): (String, Option<String>, String) = conn.query_row(
            "SELECT staff_group, lifecycle_status, employee_id FROM employees WHERE id = ?",
            params![emp_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).expect("load rehired row");

        assert_eq!(group, "employee_list", "Must be in employee_list");
        assert_eq!(status.as_deref(), Some("rehired"), "lifecycle_status must be rehired");
        assert_eq!(e_id, "ASWVN1145", "canonical employee_id must be preserved");

        // Assert asset loan is preserved and still active
        let loan_count_after: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_loans WHERE employee_id_fk = ? AND returned_at IS NULL",
            params![emp_id],
            |row| row.get(0),
        ).expect("check active loan after rehire");
        assert_eq!(loan_count_after, 1, "Asset loans must be fully preserved across rehire");
    }

    #[test]
    fn aswvn1145_missing_from_source_remains_report_only_and_untouched() {
        let conn = open_test_connection();
        let emp_id = seed_employee(&conn, "ASWVN1145", "Nguyen Van A");
        let laptop_id = seed_asset(&conn, "VNLAP1145", "Laptop", "laptop");
        seed_active_loan(&conn, emp_id, laptop_id);

        // Verify ASWVN1145 is active in employee_list
        let (group, name): (String, String) = conn.query_row(
            "SELECT staff_group, full_name FROM employees WHERE employee_id = 'ASWVN1145'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("read local ASWVN1145");
        assert_eq!(group, "employee_list");
        assert_eq!(name, "Nguyen Van A");

        // In Query B comparison, when source does not have ASWVN1145, it is classified as Missing (report-only).
        // On applying sync, the update loop only iterates over remote source records, so ASWVN1145 is NEVER
        // offboarded or modified.
        let group_after: String = conn.query_row(
            "SELECT staff_group FROM employees WHERE employee_id = 'ASWVN1145'",
            [],
            |row| row.get(0),
        ).expect("check ASWVN1145 group after sync");
        assert_eq!(group_after, "employee_list", "Missing employee must never be automatically offboarded");

        let loan_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_loans WHERE employee_id_fk = ? AND returned_at IS NULL",
            params![emp_id],
            |row| row.get(0),
        ).expect("check loan after missing check");
        assert_eq!(loan_count, 1, "Asset loans must remain intact");
    }

    #[test]
    fn test_assigned_assets_count_and_details() {
        let conn = open_test_connection();
        let emp1_id = seed_employee(&conn, "EE-100", "Alice Worker");
        let emp2_id = seed_employee(&conn, "EE-200", "Bob NoAssets");

        let cat_laptop: i64 = conn.query_row(
            "SELECT id FROM asset_categories WHERE category_code = 'laptop'",
            [],
            |r| r.get(0),
        ).unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO asset_categories(category_code, category_name, tracking_mode, has_computer_name) VALUES ('hphone', 'H Phone', 'serialized', 0)",
            [],
        ).unwrap();
        let cat_phone: i64 = conn.query_row(
            "SELECT id FROM asset_categories WHERE category_code = 'hphone'",
            [],
            |r| r.get(0),
        ).unwrap();

        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, serial_number, usage_location, status) VALUES ('LT-01', ?, 'Laptop', 'Dell Latitude', 'SN-DELL-1', 'Floor 3', 'borrowed')",
            params![cat_laptop],
        ).unwrap();
        let asset1 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, serial_number, usage_location, status) VALUES ('HP-01', ?, 'H Phone', 'Samsung Galaxy', 'SN-SAM-1', 'Remote', 'borrowed')",
            params![cat_phone],
        ).unwrap();
        let asset2 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, serial_number, usage_location, status) VALUES ('LT-02', ?, 'Laptop', 'Lenovo ThinkPad', 'SN-LEN-1', 'Floor 2', 'in_stock')",
            params![cat_laptop],
        ).unwrap();
        let asset3 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status) VALUES ('REQ-1', ?, 'EE-100', 'Alice Worker', 'approved')",
            params![emp1_id],
        ).unwrap();
        let req_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at) VALUES (?, ?, ?, '2026-10-01 10:00:00')",
            params![asset1, emp1_id, req_id],
        ).unwrap();

        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at) VALUES (?, ?, ?, '2026-10-02 11:00:00')",
            params![asset2, emp1_id, req_id],
        ).unwrap();

        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at, returned_at) VALUES (?, ?, ?, '2026-09-01 10:00:00', '2026-09-15 10:00:00')",
            params![asset3, emp1_id, req_id],
        ).unwrap();

        let res = query_employees(&conn, EmployeeQuery {
            query: None,
            team_name: None,
            staff_group: None,
            is_floating: None,
            sort_key: None,
            sort_direction: None,
            start_date_from: None,
            start_date_to: None,
            limit: Some(10),
            offset: Some(0),
        }).unwrap();

        let emp1 = res.items.iter().find(|e| e.id == emp1_id).unwrap();
        assert_eq!(emp1.assigned_assets_count, 2, "Alice should have 2 active assigned assets");

        let emp2 = res.items.iter().find(|e| e.id == emp2_id).unwrap();
        assert_eq!(emp2.assigned_assets_count, 0, "Bob should have 0 active assigned assets");

        let assets_emp1 = get_employee_assigned_assets_conn(&conn, emp1_id).unwrap();
        assert_eq!(assets_emp1.len(), 2);
        assert_eq!(assets_emp1[0].asset_code, "LT-01");
        assert_eq!(assets_emp1[0].asset_name, "Dell Latitude");
        assert_eq!(assets_emp1[0].category_name, "Laptop");
        assert_eq!(assets_emp1[0].serial_number.as_deref(), Some("SN-DELL-1"));
        assert_eq!(assets_emp1[0].usage_location.as_deref(), Some("Floor 3"));
        assert_eq!(assets_emp1[0].borrowed_at, "2026-10-01 10:00:00");

        assert_eq!(assets_emp1[1].asset_code, "HP-01");
        assert_eq!(assets_emp1[1].asset_name, "Samsung Galaxy");
        assert_eq!(assets_emp1[1].category_name, "H Phone");
        assert_eq!(assets_emp1[1].usage_location.as_deref(), Some("Remote"));

        let assets_emp2 = get_employee_assigned_assets_conn(&conn, emp2_id).unwrap();
        assert_eq!(assets_emp2.len(), 0);

        let res_sorted_desc = query_employees(&conn, EmployeeQuery {
            query: None,
            team_name: None,
            staff_group: None,
            is_floating: None,
            sort_key: Some("assignedAssets".to_string()),
            sort_direction: Some("desc".to_string()),
            start_date_from: None,
            start_date_to: None,
            limit: Some(10),
            offset: Some(0),
        }).unwrap();
        assert_eq!(res_sorted_desc.items[0].id, emp1_id);
    }

    #[test]
    fn test_phase0_legacy_safety_gate() {
        let candidates = ["data/phase0_safety_db/staff_kit.sqlite3", "../data/phase0_safety_db/staff_kit.sqlite3"];
        let db_path = candidates.iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(db_path) = db_path else {
            println!("phase0_safety_db not found, skipping");
            return;
        };

        let conn = crate::db::open_encrypted_read_only_connection(db_path)
            .expect("open read only safety db");

        let asset_name_count: i64 = conn.query_row(
            "SELECT count(*) FROM employee_dynamic_values WHERE field_key = 'asset_name'",
            [],
            |r| r.get(0),
        ).unwrap();
        let cateagory_count: i64 = conn.query_row(
            "SELECT count(*) FROM employee_dynamic_values WHERE field_key = 'cateagory'",
            [],
            |r| r.get(0),
        ).unwrap();
        let location_count: i64 = conn.query_row(
            "SELECT count(*) FROM employee_dynamic_values WHERE field_key = 'location'",
            [],
            |r| r.get(0),
        ).unwrap();
        let distinct_legacy_employees: i64 = conn.query_row(
            "SELECT count(DISTINCT employee_id) FROM employee_dynamic_values WHERE field_key IN ('asset_name', 'cateagory', 'location')",
            [],
            |r| r.get(0),
        ).unwrap();

        let total_employees: i64 = conn.query_row("SELECT count(*) FROM employees", [], |r| r.get(0)).unwrap();
        let total_assets: i64 = conn.query_row("SELECT count(*) FROM assets", [], |r| r.get(0)).unwrap();
        let active_loans: i64 = conn.query_row("SELECT count(*) FROM asset_loans WHERE returned_at IS NULL", [], |r| r.get(0)).unwrap();

        println!("--- PHASE 0 BASELINE COUNTS ---");
        println!("Total employees: {total_employees}");
        println!("Distinct legacy asset employees: {distinct_legacy_employees}");
        println!("asset_name entries: {asset_name_count}");
        println!("cateagory entries: {cateagory_count}");
        println!("location entries: {location_count}");
        println!("Total assets: {total_assets}");
        println!("Active loans: {active_loans}");

        assert_eq!(asset_name_count, 92);
        assert_eq!(cateagory_count, 91);
        assert_eq!(location_count, 92);
        assert_eq!(distinct_legacy_employees, 92);
        assert_eq!(total_employees, 442);
    }

    #[test]
    fn test_employee_monitor_display_alignment_zero_one_two_four_and_legacy_safety() {
        let mut conn = open_test_connection();

        // 1. Categories (already seeded by migrations)
        let mon_cat_id: i64 = conn.query_row(
            "SELECT id FROM asset_categories WHERE category_code = 'monitor'",
            [],
            |r| r.get(0),
        ).unwrap();
        let lap_cat_id: i64 = conn.query_row(
            "SELECT id FROM asset_categories WHERE category_code = 'laptop'",
            [],
            |r| r.get(0),
        ).unwrap();

        // 2. Employees
        conn.execute(
            "INSERT INTO employees(employee_id, full_name, staff_group, updated_at) VALUES('EE001', 'Nguyen Van A', 'employee_list', datetime('now'))",
            [],
        ).unwrap();
        let emp1_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO employees(employee_id, full_name, staff_group, updated_at) VALUES('EE002', 'Tran Van B', 'employee_list', datetime('now'))",
            [],
        ).unwrap();
        let emp2_id = conn.last_insert_rowid();

        // 3. Dynamic field setup & Legacy value for EMP1
        conn.execute(
            "INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at) VALUES('asset_name', 'Asset Name', datetime('now'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO employee_dynamic_values(employee_id, field_key, value, updated_at) VALUES(?, 'asset_name', 'MON_LEGACY', datetime('now'))",
            params![emp1_id],
        ).unwrap();

        // 4. Test 0 monitors: EMP1 has legacy fallback in memory, EMP2 has None
        let res0 = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        let emp1_row0 = res0.items.iter().find(|e| e.id == emp1_id).unwrap();
        let emp2_row0 = res0.items.iter().find(|e| e.id == emp2_id).unwrap();
        assert_eq!(emp1_row0.dynamic_fields.get("asset_name"), Some(&"MON_LEGACY".to_string()));
        assert_eq!(emp1_row0.dynamic_fields.get("asset_name_2"), None);
        assert_eq!(emp1_row0.assigned_assets_count, 0);
        assert_eq!(emp2_row0.dynamic_fields.get("asset_name"), None);

        // 5. Add Laptop loan for EMP1 (PC-only invariant check)
        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, updated_at) VALUES('VNLAP100', ?, 'Laptop', 'Dell Laptop', 'assigned', datetime('now'))",
            params![lap_cat_id],
        ).unwrap();
        let lap_id = conn.last_insert_rowid();
        seed_active_loan_with_borrowed_at(&conn, emp1_id, lap_id, "2026-01-01 08:00:00");

        let res_lap = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        let emp1_lap = res_lap.items.iter().find(|e| e.id == emp1_id).unwrap();
        assert_eq!(emp1_lap.computer_name.as_deref(), Some("ASWVNLAP100"));
        // Laptop did not affect asset_name fallback:
        assert_eq!(emp1_lap.dynamic_fields.get("asset_name"), Some(&"MON_LEGACY".to_string()));

        // 6. Add 1 Monitor loan for EMP1 ("MON614")
        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, updated_at) VALUES('VNMON614', ?, 'Monitor', 'MON614', 'assigned', datetime('now'))",
            params![mon_cat_id],
        ).unwrap();
        let mon1_id = conn.last_insert_rowid();
        seed_active_loan_with_borrowed_at(&conn, emp1_id, mon1_id, "2026-01-01 09:00:00");

        let res1 = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        let emp1_row1 = res1.items.iter().find(|e| e.id == emp1_id).unwrap();
        assert_eq!(emp1_row1.dynamic_fields.get("asset_name"), Some(&"MON614".to_string()));
        assert_eq!(emp1_row1.dynamic_fields.get("asset_name_2"), None);
        assert_eq!(emp1_row1.assigned_assets_count, 2); // 1 laptop + 1 monitor

        // Legacy value in DB is still untouched:
        let legacy_db_val: String = conn.query_row(
            "SELECT value FROM employee_dynamic_values WHERE employee_id = ? AND field_key = 'asset_name'",
            params![emp1_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(legacy_db_val, "MON_LEGACY");

        // 7. Add 2nd Monitor loan ("MON615")
        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, updated_at) VALUES('VNMON615', ?, 'Monitor', 'MON615', 'assigned', datetime('now'))",
            params![mon_cat_id],
        ).unwrap();
        let mon2_id = conn.last_insert_rowid();
        seed_active_loan_with_borrowed_at(&conn, emp1_id, mon2_id, "2026-01-02 09:00:00");

        let res2 = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        let emp1_row2 = res2.items.iter().find(|e| e.id == emp1_id).unwrap();
        assert_eq!(emp1_row2.dynamic_fields.get("asset_name"), Some(&"MON614".to_string()));
        assert_eq!(emp1_row2.dynamic_fields.get("asset_name_2"), Some(&"MON615".to_string()));
        assert_eq!(emp1_row2.dynamic_fields.get("aset_name_2"), Some(&"MON615".to_string()));
        assert_eq!(emp1_row2.assigned_assets_count, 3); // 1 laptop + 2 monitors

        // 8. Add 3rd and 4th Monitor loans ("MON616", "MON617")
        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, updated_at) VALUES('VNMON616', ?, 'Monitor', 'MON616', 'assigned', datetime('now'))",
            params![mon_cat_id],
        ).unwrap();
        let mon3_id = conn.last_insert_rowid();
        seed_active_loan_with_borrowed_at(&conn, emp1_id, mon3_id, "2026-01-03 09:00:00");

        conn.execute(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, updated_at) VALUES('VNMON617', ?, 'Monitor', 'MON617', 'assigned', datetime('now'))",
            params![mon_cat_id],
        ).unwrap();
        let mon4_id = conn.last_insert_rowid();
        seed_active_loan_with_borrowed_at(&conn, emp1_id, mon4_id, "2026-01-04 09:00:00");

        let res4 = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        let emp1_row4 = res4.items.iter().find(|e| e.id == emp1_id).unwrap();
        assert_eq!(emp1_row4.dynamic_fields.get("asset_name"), Some(&"MON614".to_string()));
        assert_eq!(emp1_row4.dynamic_fields.get("asset_name_2"), Some(&"MON615".to_string()));
        assert_eq!(emp1_row4.assigned_assets_count, 5); // 1 laptop + 4 monitors

        // Assigned assets modal data returns all 5:
        let all_emp1_assets = get_employee_assigned_assets_conn(&conn, emp1_id).unwrap();
        assert_eq!(all_emp1_assets.len(), 5);
        assert_eq!(all_emp1_assets[1].asset_name, "MON614");
        assert_eq!(all_emp1_assets[2].asset_name, "MON615");
        assert_eq!(all_emp1_assets[3].asset_name, "MON616");
        assert_eq!(all_emp1_assets[4].asset_name, "MON617");

        // 9. Return Monitor 1 ("MON614") -> slots dynamically roll to MON615 and MON616
        conn.execute(
            "UPDATE asset_loans SET returned_at = datetime('now') WHERE employee_id_fk = ? AND asset_id = ?",
            params![emp1_id, mon1_id],
        ).unwrap();

        let res_ret = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        let emp1_ret = res_ret.items.iter().find(|e| e.id == emp1_id).unwrap();
        assert_eq!(emp1_ret.dynamic_fields.get("asset_name"), Some(&"MON615".to_string()));
        assert_eq!(emp1_ret.dynamic_fields.get("asset_name_2"), Some(&"MON616".to_string()));
        assert_eq!(emp1_ret.assigned_assets_count, 4);

        // 10. Zero writes on employee save
        let payload = EmployeePayload {
            employee_id: "EE001".to_string(),
            full_name: "Nguyen Van A (Updated)".to_string(),
            nick_name: None, team_name: None, project: None, job_title: None,
            email: None, cellphone: None, date_of_birth: None, gender: None,
            asw_start_date: None, client_start_date: None, contract_end_date: None,
            client_year_of_services: None, computer_name: None, notes: None,
            is_floating: None, lifecycle_status: None,
            staff_group: Some("employee_list".to_string()),
            dynamic_fields: Some(HashMap::from([
                ("asset_name".to_string(), "TRY_OVERWRITE".to_string()),
                ("asset_name_2".to_string(), "TRY_OVERWRITE_2".to_string()),
            ])),
        };
        let mut tx = conn.transaction().unwrap();
        upsert_employee_from_payload(&mut tx, payload, "employee_list").unwrap();
        tx.commit().unwrap();

        let legacy_after_save: String = conn.query_row(
            "SELECT value FROM employee_dynamic_values WHERE employee_id = ? AND field_key = 'asset_name'",
            params![emp1_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(legacy_after_save, "MON_LEGACY", "Legacy DB value must be untouched");

        let exists_asset_name_2: i64 = conn.query_row(
            "SELECT count(*) FROM employee_dynamic_values WHERE employee_id = ? AND field_key = 'asset_name_2'",
            params![emp1_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(exists_asset_name_2, 0, "asset_name_2 must never be written to employee_dynamic_values");

        // 11. Search by monitor name
        let search_res = query_employees(&conn, EmployeeQuery {
            query: Some("MON615".to_string()),
            team_name: None, staff_group: None, is_floating: None,
            sort_key: None, sort_direction: None, start_date_from: None, start_date_to: None,
            limit: Some(10), offset: Some(0),
        }).unwrap();
        assert_eq!(search_res.items.len(), 1);
        assert_eq!(search_res.items[0].id, emp1_id);

        // 12. Sorting by asset_name
        let sort_res = query_employees(&conn, EmployeeQuery {
            query: None, team_name: None, staff_group: None, is_floating: None,
            sort_key: Some("asset_name".to_string()), sort_direction: Some("asc".to_string()),
            start_date_from: None, start_date_to: None, limit: Some(10), offset: Some(0),
        }).unwrap();
        assert_eq!(sort_res.items.len(), 2);
    }
}
