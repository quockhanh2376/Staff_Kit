use std::collections::HashMap;

use rusqlite::{params, types::Value, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::column::{
    dynamic_field_exists, upsert_dynamic_field_definitions_for_map, upsert_dynamic_fields_tx,
};
use super::schema::{
    EMPLOYEE_SELECT_COLUMNS, STAFF_GROUP_EMPLOYEE_LIST, STAFF_GROUP_INTERNAL_MOVEMENT,
    STAFF_GROUP_OFFBOARDING, STAFF_GROUP_ONBOARDING,
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
    pub notes: Option<String>,
    pub staff_group: String,
    pub dynamic_fields: HashMap<String, String>,
    pub updated_at: String,
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
    pub dynamic_fields: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveEmployeesGroupInput {
    pub employee_ids: Vec<i64>,
    pub target_staff_group: String,
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
            computer_name: normalize_optional_text(payload.computer_name),
            notes: normalize_optional_text(payload.notes),
            dynamic_fields: normalize_dynamic_fields(payload.dynamic_fields),
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
      laptop_names.computer_name
    FROM (
      SELECT
        al.employee_id_fk,
        GROUP_CONCAT('ASW' || a.asset_code, ',' || char(10)) OVER (
          PARTITION BY al.employee_id_fk
          ORDER BY al.id ASC
          ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        ) AS computer_name,
        ROW_NUMBER() OVER (
          PARTITION BY al.employee_id_fk
          ORDER BY al.id DESC
        ) AS row_num
      FROM asset_loans al
      INNER JOIN assets a ON a.id = al.asset_id
    INNER JOIN asset_categories c ON c.id = a.category_id
      WHERE al.returned_at IS NULL
    AND COALESCE(c.has_computer_name, 0) = 1
    ) laptop_names
    WHERE laptop_names.row_num = 1
) lc ON lc.employee_id_fk = e.id
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
    let total = employee_list + onboarding + offboarding + internal_movement;

    Ok(EmployeeGroupCounts {
        employee_list,
        onboarding,
        offboarding,
        internal_movement,
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

    tx.commit()
        .map_err(|err| format!("failed to commit update transaction: {err}"))?;

    let conn2 = open_runtime_connection(app)?;
    load_employee_by_id(&conn2, row_id)
}

pub fn move_employees_group(
    app: &AppHandle,
    payload: MoveEmployeesGroupInput,
) -> Result<i64, String> {
    let target_group = normalize_staff_group(payload.target_staff_group.as_str())
        .ok_or_else(|| format!("invalid target staff group: {}", payload.target_staff_group))?;

    if payload.employee_ids.is_empty() {
        return Ok(0);
    }

    let conn = open_runtime_connection(app)?;
    let placeholders = vec!["?"; payload.employee_ids.len()].join(", ");
    let sql = format!(
        "UPDATE employees SET staff_group = ?, updated_at = datetime('now') WHERE id IN ({placeholders})"
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(target_group.to_string())];
    for id in &payload.employee_ids {
        params_vec.push(Box::new(*id));
    }

    let changed = conn
        .execute(
            &sql,
            rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
        )
        .map_err(humanize_sqlite_error)?;

    Ok(changed as i64)
}

pub fn delete_employee(app: &AppHandle, id: i64) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start employee delete transaction: {err}"))?;

    tx.execute(
        r#"
        UPDATE assets
        SET status = 'in_stock',
            updated_at = datetime('now')
        WHERE id IN (
          SELECT asset_id
          FROM asset_loans
          WHERE employee_id_fk = ?
            AND returned_at IS NULL
        )
        "#,
        params![id],
    )
    .map_err(humanize_sqlite_error)?;

    tx.execute(
        "DELETE FROM asset_loans WHERE employee_id_fk = ?",
        params![id],
    )
    .map_err(humanize_sqlite_error)?;

    tx.execute(
        "DELETE FROM borrow_requests WHERE employee_id_fk = ?",
        params![id],
    )
    .map_err(humanize_sqlite_error)?;

    let changed = tx
        .execute("DELETE FROM employees WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;

    tx.commit()
        .map_err(|err| format!("failed to commit employee delete: {err}"))?;

    Ok(changed > 0)
}

// ── pub(crate) helpers used by import.rs ─────────────────────────────────────

pub(crate) fn load_employee_by_employee_id(
    conn: &Connection,
    employee_id: &str,
) -> Result<Option<EmployeeRecord>, String> {
    let sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} WHERE e.employee_id = ?"
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
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} WHERE lower(trim(COALESCE(e.email, ''))) = lower(trim(?))"
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
          updated_at
        ) VALUES (
          ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now')
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
                "(e.id IN (SELECT rowid FROM employees_fts WHERE employees_fts MATCH ?) OR lower(COALESCE(lc.computer_name, '')) LIKE ?)"
                    .to_string(),
            );
            filter_params.push(Value::Text(fts_query));
            filter_params.push(Value::Text(query_like));
        } else {
            where_clauses.push("lower(COALESCE(lc.computer_name, '')) LIKE ?".to_string());
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

    if let Some(raw_group) = raw_group {
        let Some(staff_group) = normalize_staff_group(raw_group.as_str()) else {
            return Err(format!("invalid staff group filter: {raw_group}"));
        };

        where_clauses.push("CASE WHEN COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') = 'internal_movent' THEN 'internal_movement' ELSE COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') END = ?".to_string());
        filter_params.push(Value::Text(staff_group.to_string()));
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
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} WHERE e.id = ?"
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

        if let Some(text) = normalize_optional_text(value) {
            items[index].dynamic_fields.insert(field_key, text);
        }
    }

    Ok(())
}

fn map_employee_row(row: &Row<'_>) -> rusqlite::Result<EmployeeRecord> {
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
        computer_name: row.get(17)?,
        stored_computer_name: row.get(18)?,
        notes: row.get(19)?,
        staff_group: row.get(20)?,
        dynamic_fields: HashMap::new(),
        updated_at: row.get(21)?,
    })
}

fn build_employee_from_clause(include_active_laptop_join: bool) -> String {
    if include_active_laptop_join {
        format!(" FROM employees e LEFT JOIN teams t ON t.id = e.team_id {EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL} ")
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
            "COALESCE(NULLIF(lc.computer_name, ''), e.computername, '') COLLATE NOCASE"
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

    let dynamic_key = super::normalize_dynamic_key(raw_key);
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

    use crate::db::{apply_migrations, configure_connection};

    use super::{
        build_employee_from_clause, query_employees, upsert_employee_from_payload, EmployeePayload,
        EmployeeQuery, EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL,
    };

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
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

    #[test]
    fn upsert_employee_from_payload_clears_blank_dynamic_fields() {
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
        assert_eq!(exists, 0);
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
        assert_eq!(
            employee.computer_name.as_deref(),
            Some("ASWVNMACPRO010,\nASWVNLAP293")
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
    fn active_laptop_join_uses_ordered_window_aggregation() {
        assert!(
            EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL.contains("GROUP_CONCAT('ASW' || a.asset_code"),
            "active-laptop join should aggregate directly from ordered loan rows",
        );
        assert!(
            EMPLOYEE_ACTIVE_LAPTOP_JOIN_SQL
                .contains("ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING"),
            "active-laptop join should use a window frame to keep GROUP_CONCAT ordering stable",
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
}
