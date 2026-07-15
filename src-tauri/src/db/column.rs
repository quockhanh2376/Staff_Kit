use std::collections::HashMap;

use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::schema::CORE_COLUMN_DEFINITIONS;
use super::{
    dynamic_key_to_label, humanize_sqlite_error, is_reserved_column_key,
    normalize_dynamic_field_key, open_runtime_connection, require_text,
};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeColumnDefinition {
    pub key: String,
    pub label: String,
    pub source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeColumnUpsertInput {
    pub key: Option<String>,
    pub label: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn list_employee_columns(app: &AppHandle) -> Result<Vec<EmployeeColumnDefinition>, String> {
    let conn = open_runtime_connection(app)?;
    let mut columns = CORE_COLUMN_DEFINITIONS
        .iter()
        .map(|(key, label)| EmployeeColumnDefinition {
            key: (*key).to_string(),
            label: (*label).to_string(),
            source: "core".to_string(),
        })
        .collect::<Vec<_>>();

    let mut stmt = conn
        .prepare(
            "SELECT field_key, field_label FROM employee_dynamic_fields ORDER BY field_label COLLATE NOCASE ASC",
        )
        .map_err(|err| format!("failed to prepare employee dynamic columns query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(EmployeeColumnDefinition {
                key: row.get::<_, String>(0)?,
                label: row.get::<_, String>(1)?,
                source: "dynamic".to_string(),
            })
        })
        .map_err(|err| format!("failed to query employee dynamic columns: {err}"))?;

    for row in rows {
        let column =
            row.map_err(|err| format!("failed to read employee dynamic column row: {err}"))?;
        if !columns.iter().any(|item| item.key == column.key) {
            columns.push(column);
        }
    }

    Ok(columns)
}

pub fn upsert_employee_column(
    app: &AppHandle,
    payload: EmployeeColumnUpsertInput,
) -> Result<EmployeeColumnDefinition, String> {
    let conn = open_runtime_connection(app)?;
    let label = require_text(payload.label, "label")?;

    let key = if let Some(raw_key) = payload.key {
        let normalized_key = normalize_dynamic_field_key(raw_key.as_str());
        if normalized_key.is_empty() {
            return Err("column key is invalid".to_string());
        }

        if is_reserved_column_key(normalized_key.as_str()) {
            return Err("cannot override a reserved core/system column".to_string());
        }

        normalized_key
    } else {
        generate_dynamic_field_key(&conn, label.as_str())?
    };

    conn.execute(
        r#"
        INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
        VALUES(?, ?, datetime('now'))
        ON CONFLICT(field_key) DO UPDATE SET
          field_label = excluded.field_label,
          updated_at = datetime('now')
        "#,
        params![key.as_str(), label.as_str()],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(EmployeeColumnDefinition {
        key,
        label,
        source: "dynamic".to_string(),
    })
}

pub fn delete_employee_column(app: &AppHandle, key: String) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let normalized = normalize_dynamic_field_key(key.as_str());
    if normalized.is_empty() {
        return Err("column key is invalid".to_string());
    }

    if is_reserved_column_key(normalized.as_str()) {
        return Err("cannot delete a reserved core/system column".to_string());
    }

    let changed = conn
        .execute(
            "DELETE FROM employee_dynamic_fields WHERE field_key = ?",
            params![normalized.as_str()],
        )
        .map_err(humanize_sqlite_error)?;

    Ok(changed > 0)
}

// ── pub(crate) helpers used by employee.rs and import.rs ─────────────────────

pub(crate) fn dynamic_field_exists(conn: &Connection, key: &str) -> Result<bool, String> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM employee_dynamic_fields WHERE field_key = ?)",
            params![key],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("failed to check dynamic field existence: {err}"))?;

    Ok(exists > 0)
}

pub(crate) fn upsert_dynamic_field_definitions_tx(
    tx: &Transaction<'_>,
    columns: &[super::import::DynamicImportColumn],
) -> Result<(), String> {
    for column in columns {
        tx.execute(
            r#"
            INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
            VALUES(?, ?, datetime('now'))
            ON CONFLICT(field_key) DO UPDATE SET
              field_label = excluded.field_label,
              updated_at = datetime('now')
            "#,
            params![column.field_key.as_str(), column.field_label.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

pub(crate) fn upsert_dynamic_field_definitions_for_map(
    tx: &Transaction<'_>,
    fields: &HashMap<String, String>,
) -> Result<(), String> {
    for key in fields.keys() {
        let label = dynamic_key_to_label(key);
        tx.execute(
            r#"
            INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
            VALUES(?, ?, datetime('now'))
            ON CONFLICT(field_key) DO NOTHING
            "#,
            params![key.as_str(), label.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

pub(crate) fn upsert_dynamic_fields_tx(
    tx: &Transaction<'_>,
    employee_id: i64,
    fields: &HashMap<String, String>,
) -> Result<(), String> {
    for (field_key, value) in fields {
        if value.trim().is_empty() && !is_computer_name_2_key(field_key.as_str()) {
            tx.execute(
                "DELETE FROM employee_dynamic_values WHERE employee_id = ? AND field_key = ?",
                params![employee_id, field_key.as_str()],
            )
            .map_err(humanize_sqlite_error)?;
            continue;
        }

        tx.execute(
            r#"
            INSERT INTO employee_dynamic_values(employee_id, field_key, value, updated_at)
            VALUES(?, ?, ?, datetime('now'))
            ON CONFLICT(employee_id, field_key) DO UPDATE SET
              value = excluded.value,
              updated_at = datetime('now')
            "#,
            params![employee_id, field_key.as_str(), value.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

fn is_computer_name_2_key(field_key: &str) -> bool {
    matches!(field_key, "computer_2" | "computer_name_2")
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn generate_dynamic_field_key(conn: &Connection, label: &str) -> Result<String, String> {
    let base_key = normalize_dynamic_field_key(label);
    if base_key.is_empty() {
        return Err("column title is invalid".to_string());
    }

    if !is_reserved_column_key(base_key.as_str()) && !dynamic_field_exists(conn, base_key.as_str())?
    {
        return Ok(base_key);
    }

    for index in 2..=9999 {
        let candidate = format!("{base_key}_{index}");
        if is_reserved_column_key(candidate.as_str()) {
            continue;
        }

        if !dynamic_field_exists(conn, candidate.as_str())? {
            return Ok(candidate);
        }
    }

    Err("failed to allocate a unique dynamic column key".to_string())
}
