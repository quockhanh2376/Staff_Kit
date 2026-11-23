//! EE List sync staging: HR records are loaded into a persisted, editable batch
//! (`employee_sync_batches` / `employee_sync_rows`) instead of being written straight
//! to `employees`. Task 4 wires these `_conn` functions to Tauri commands.
#![allow(dead_code)] // Task 4 wires the staging functions to commands.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::humanize_sqlite_error;
use super::mssql_import::{
    compare_employee_list, normalize_sync_text, normalized_staff_code, EmployeeSyncCounts,
    LocalEmployeeListRecord, MssqlStaffRecord,
};
use super::schema::{AZURE_AD_ACCOUNT_FIELD_KEY, STAFF_GROUP_EMPLOYEE_LIST};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncFields {
    pub full_name: String,
    pub nick_name: Option<String>,
    pub email: Option<String>,
    pub azure_ad_account: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncStagedRow {
    pub id: i64,
    pub staff_code: String,
    pub kind: String,
    pub local_employee_id: Option<i64>,
    pub local_group: Option<String>,
    pub source: Vec<EmployeeSyncFields>,
    pub local: Option<EmployeeSyncFields>,
    pub edits: Option<EmployeeSyncFields>,
    pub action: String,
    pub notes: Vec<String>,
    pub state: String,
    pub assets_held: i64,
    pub is_floating: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncBatch {
    pub id: i64,
    pub created_at: String,
    pub counts: EmployeeSyncCounts,
    pub rows: Vec<EmployeeSyncStagedRow>,
}

const DUPLICATE_CODE_NOTE: &str = "Duplicate Staff Code in HR source";

/// Local `employees` row as seen at staging time. Serialised into `local_json`;
/// `staffGroup` and `updatedAt` complete the drift fingerprint checked at Approve.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalSnapshot {
    #[serde(flatten)]
    fields: EmployeeSyncFields,
    staff_group: String,
    updated_at: String,
    #[serde(skip)]
    id: i64,
    #[serde(skip)]
    code: String,
    #[serde(skip)]
    is_floating: bool,
    #[serde(skip)]
    assets_held: i64,
}

struct NewRow<'a> {
    staff_code: String,
    kind: &'static str,
    local: Option<&'a LocalSnapshot>,
    source: Vec<EmployeeSyncFields>,
    action: &'static str,
    notes: Vec<String>,
}

fn source_fields(record: &MssqlStaffRecord) -> EmployeeSyncFields {
    EmployeeSyncFields {
        full_name: record.name.trim().to_string(),
        nick_name: normalize_sync_text(record.nick_name.as_deref()),
        email: normalize_sync_text(record.work_email.as_deref()).map(|value| value.to_lowercase()),
        azure_ad_account: normalize_sync_text(record.azure_ad_account.as_deref()),
    }
}

fn read_local_snapshot_conn(conn: &Connection) -> Result<Vec<LocalSnapshot>, String> {
    let mut statement = conn
        .prepare(
            r#"
            SELECT e.id, e.employee_id, e.full_name, e.nick_name, e.email, azure.value,
                   COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list'), e.updated_at,
                   e.is_floating,
                   (SELECT COUNT(*) FROM asset_loans l
                     WHERE l.employee_id_fk = e.id AND l.returned_at IS NULL)
            FROM employees e
            LEFT JOIN employee_dynamic_values azure
              ON azure.employee_id = e.id AND azure.field_key = ?1
            ORDER BY UPPER(TRIM(e.employee_id)), e.id
            "#,
        )
        .map_err(|err| format!("failed to read EE List sync snapshot: {err}"))?;
    let rows = statement
        .query_map(params![AZURE_AD_ACCOUNT_FIELD_KEY], |row| {
            Ok(LocalSnapshot {
                id: row.get(0)?,
                code: row.get(1)?,
                fields: EmployeeSyncFields {
                    full_name: row.get(2)?,
                    nick_name: row.get(3)?,
                    email: row.get(4)?,
                    azure_ad_account: row.get(5)?,
                },
                staff_group: row.get(6)?,
                updated_at: row.get(7)?,
                is_floating: row.get::<_, i64>(8)? != 0,
                assets_held: row.get(9)?,
            })
        })
        .map_err(|err| format!("failed to read EE List sync snapshot: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read EE List sync snapshot row: {err}"))
}

fn open_batch_id(conn: &Connection) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT id FROM employee_sync_batches ORDER BY id DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(humanize_sqlite_error)
}

fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("failed to encode EE List sync row: {err}"))
}

pub(crate) fn stage_employee_sync_batch_conn(
    conn: &mut Connection,
    source: &[MssqlStaffRecord],
    replace: bool,
    actor_ref: Option<&str>,
) -> Result<EmployeeSyncBatch, String> {
    // An empty HR result would stage every EE List employee for offboarding.
    if source.is_empty() {
        return Err("Cannot load: the HR source returned no employees".to_string());
    }
    let snapshot = read_local_snapshot_conn(conn)?;
    let (ee_list, others): (Vec<&LocalSnapshot>, Vec<&LocalSnapshot>) = snapshot
        .iter()
        .partition(|local| local.staff_group == STAFF_GROUP_EMPLOYEE_LIST);
    // Last wins, the same as compare_employee_list's own lookup.
    let ee_by_code: HashMap<String, &LocalSnapshot> = ee_list
        .iter()
        .map(|local| (normalized_staff_code(&local.code), *local))
        .collect();
    let mut other_by_code: HashMap<String, &LocalSnapshot> = HashMap::new();
    for local in &others {
        other_by_code
            .entry(normalized_staff_code(&local.code))
            .or_insert(*local);
    }

    // Duplicate detection runs before classification: only the first record per code is compared.
    let mut by_code: HashMap<String, Vec<&MssqlStaffRecord>> = HashMap::new();
    let mut unique_source: Vec<MssqlStaffRecord> = Vec::new();
    for record in source {
        let records = by_code
            .entry(normalized_staff_code(&record.code))
            .or_default();
        if records.is_empty() {
            unique_source.push(record.clone());
        }
        records.push(record);
    }

    let compare_local: Vec<LocalEmployeeListRecord> = ee_list
        .iter()
        .map(|local| LocalEmployeeListRecord {
            employee_id: local.code.clone(),
            full_name: local.fields.full_name.clone(),
            nick_name: local.fields.nick_name.clone(),
            email: local.fields.email.clone(),
            azure_ad_account: local.fields.azure_ad_account.clone(),
        })
        .collect();
    let other_codes: Vec<String> = others.iter().map(|local| local.code.clone()).collect();
    let diff = compare_employee_list(&unique_source, &compare_local, &other_codes);

    // compare_employee_list emits one row per source record, in order, then one row per
    // EE List employee absent from the source, in local order.
    let (source_rows, missing_rows) = diff.rows.split_at(unique_source.len());
    let missing_locals = ee_list
        .iter()
        .copied()
        .filter(|local| !by_code.contains_key(&normalized_staff_code(&local.code)));
    let mut unchanged = 0usize;
    let mut new_rows: Vec<NewRow> = Vec::new();
    for (record, compared) in unique_source.iter().zip(source_rows) {
        let code = normalized_staff_code(&record.code);
        let records = &by_code[&code];
        let source = records.iter().map(|record| source_fields(record)).collect();
        if records.len() > 1 {
            new_rows.push(NewRow {
                local: ee_by_code
                    .get(&code)
                    .or_else(|| other_by_code.get(&code))
                    .copied(),
                staff_code: code,
                kind: "conflict",
                source,
                action: "skip",
                notes: vec![DUPLICATE_CODE_NOTE.to_string()],
            });
            continue;
        }
        let (kind, action, local) = match compared.status.as_str() {
            "unchanged" => {
                unchanged += 1;
                continue;
            }
            "new" => ("new", "apply", None),
            "changed" => ("changed", "apply", ee_by_code.get(&code).copied()),
            "conflict" => (
                "conflict",
                "move_to_employee_list",
                other_by_code.get(&code).copied(),
            ),
            other => return Err(format!("Invalid EE List sync row kind: {other}")),
        };
        new_rows.push(NewRow {
            staff_code: code,
            kind,
            local,
            source,
            action,
            notes: Vec::new(),
        });
    }
    for (local, compared) in missing_locals.zip(missing_rows) {
        debug_assert_eq!(compared.status, "missing");
        new_rows.push(NewRow {
            staff_code: normalized_staff_code(&local.code),
            kind: "missing",
            local: Some(local),
            source: Vec::new(),
            action: if local.is_floating {
                "none"
            } else {
                "move_to_offboarding"
            },
            notes: Vec::new(),
        });
    }

    let tx = conn.transaction().map_err(humanize_sqlite_error)?;
    if open_batch_id(&tx)?.is_some() {
        if !replace {
            return Err(
                "Cannot load: an EE List sync is already open. Continue it or discard it first."
                    .to_string(),
            );
        }
        // Rows go with their batch through ON DELETE CASCADE.
        tx.execute("DELETE FROM employee_sync_batches", [])
            .map_err(humanize_sqlite_error)?;
    }
    tx.execute(
        "INSERT INTO employee_sync_batches (created_by, source_row_count, unchanged_count) VALUES (?1, ?2, ?3)",
        params![actor_ref, source.len() as i64, unchanged as i64],
    )
    .map_err(humanize_sqlite_error)?;
    let batch_id = tx.last_insert_rowid();
    {
        let mut insert = tx
            .prepare(
                r#"INSERT INTO employee_sync_rows
                   (batch_id, staff_code, kind, local_employee_id, local_group, source_json,
                    local_json, action, notes_json, assets_held, is_floating)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
            )
            .map_err(humanize_sqlite_error)?;
        for row in &new_rows {
            insert
                .execute(params![
                    batch_id,
                    row.staff_code,
                    row.kind,
                    row.local.map(|local| local.id),
                    row.local.map(|local| local.staff_group.as_str()),
                    to_json(&row.source)?,
                    row.local.map(to_json).transpose()?,
                    row.action,
                    to_json(&row.notes)?,
                    row.local.map_or(0, |local| local.assets_held),
                    row.local.is_some_and(|local| local.is_floating),
                ])
                .map_err(humanize_sqlite_error)?;
        }
    }
    let batch = load_batch(&tx, batch_id)?;
    tx.commit().map_err(humanize_sqlite_error)?;
    Ok(batch)
}

fn parse_json<T: serde::de::DeserializeOwned>(text: &str) -> rusqlite::Result<T> {
    serde_json::from_str(text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn load_batch(conn: &Connection, batch_id: i64) -> Result<EmployeeSyncBatch, String> {
    let (created_at, unchanged): (String, i64) = conn
        .query_row(
            "SELECT created_at, unchanged_count FROM employee_sync_batches WHERE id = ?1",
            [batch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(humanize_sqlite_error)?;
    let mut statement = conn
        .prepare(
            r#"SELECT id, staff_code, kind, local_employee_id, local_group, source_json, local_json,
                      edited_json, action, notes_json, state, assets_held, is_floating
               FROM employee_sync_rows WHERE batch_id = ?1 ORDER BY id"#,
        )
        .map_err(humanize_sqlite_error)?;
    let rows = statement
        .query_map([batch_id], |row| {
            Ok(EmployeeSyncStagedRow {
                id: row.get(0)?,
                staff_code: row.get(1)?,
                kind: row.get(2)?,
                local_employee_id: row.get(3)?,
                local_group: row.get(4)?,
                source: parse_json(&row.get::<_, String>(5)?)?,
                local: row
                    .get::<_, Option<String>>(6)?
                    .as_deref()
                    .map(parse_json)
                    .transpose()?,
                edits: row
                    .get::<_, Option<String>>(7)?
                    .as_deref()
                    .map(parse_json)
                    .transpose()?,
                action: row.get(8)?,
                notes: parse_json(&row.get::<_, String>(9)?)?,
                state: row.get(10)?,
                assets_held: row.get(11)?,
                is_floating: row.get::<_, i64>(12)? != 0,
            })
        })
        .map_err(humanize_sqlite_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read EE List sync row: {err}"))?;
    let mut counts = EmployeeSyncCounts {
        unchanged: unchanged as usize,
        ..EmployeeSyncCounts::default()
    };
    for row in &rows {
        match row.kind.as_str() {
            "new" => counts.new += 1,
            "changed" => counts.changed += 1,
            "missing" => counts.missing += 1,
            _ => counts.conflicts += 1,
        }
    }
    Ok(EmployeeSyncBatch {
        id: batch_id,
        created_at,
        counts,
        rows,
    })
}

pub(crate) fn load_open_employee_sync_batch_conn(
    conn: &Connection,
) -> Result<Option<EmployeeSyncBatch>, String> {
    open_batch_id(conn)?
        .map(|batch_id| load_batch(conn, batch_id))
        .transpose()
}

pub(crate) fn discard_employee_sync_batch_conn(
    conn: &mut Connection,
    batch_id: i64,
) -> Result<(), String> {
    // Rows go with their batch through ON DELETE CASCADE.
    let deleted = conn
        .execute(
            "DELETE FROM employee_sync_batches WHERE id = ?1",
            [batch_id],
        )
        .map_err(humanize_sqlite_error)?;
    if deleted == 0 {
        return Err("Not found: EE List sync".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        super::super::configure_connection(&conn).expect("configure sqlite pragmas");
        super::super::apply_migrations(&conn).expect("apply migrations");
        conn
    }

    fn hr(code: &str, name: &str) -> MssqlStaffRecord {
        MssqlStaffRecord {
            code: code.to_string(),
            name: name.to_string(),
            nick_name: None,
            work_email: None,
            azure_ad_account: None,
        }
    }

    fn seed(conn: &Connection, code: &str, name: &str, group: &str, floating: bool) -> i64 {
        conn.execute(
            "INSERT INTO employees (employee_id, full_name, staff_group, is_floating) VALUES (?1, ?2, ?3, ?4)",
            params![code, name, group, floating],
        )
        .expect("seed employee");
        conn.last_insert_rowid()
    }

    fn row<'a>(batch: &'a EmployeeSyncBatch, code: &str) -> &'a EmployeeSyncStagedRow {
        batch
            .rows
            .iter()
            .find(|row| row.staff_code == code)
            .unwrap_or_else(|| panic!("row {code} staged"))
    }

    fn table_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
    }

    #[test]
    fn stage_with_conflicts_creates_batch_with_default_actions() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-CHANGED", "Old Name", "employee_list", false);
        seed(&conn, "EE-SAME", "Same Name", "employee_list", false);
        seed(&conn, "EE-ONB", "Onboarding Person", "onboarding", false);
        seed(&conn, "EE-GONE", "Gone Person", "employee_list", false);
        seed(&conn, "EE-FLOAT", "Floating Person", "employee_list", true);
        let source = vec![
            hr("EE-NEW", "New Person"),
            hr("EE-CHANGED", "New Name"),
            hr("EE-SAME", "Same Name"),
            hr("ee-onb ", "Onboarding Person HR"),
        ];

        let batch = stage_employee_sync_batch_conn(&mut conn, &source, false, Some("admin"))
            .expect("stage batch");

        let expect = [
            ("EE-NEW", "new", "apply"),
            ("EE-CHANGED", "changed", "apply"),
            ("EE-ONB", "conflict", "move_to_employee_list"),
            ("EE-GONE", "missing", "move_to_offboarding"),
            ("EE-FLOAT", "missing", "none"),
        ];
        assert_eq!(batch.rows.len(), expect.len());
        for (code, kind, action) in expect {
            let staged = row(&batch, code);
            assert_eq!(staged.kind, kind, "{code} kind");
            assert_eq!(staged.action, action, "{code} action");
            assert_eq!(staged.state, "pending");
        }
        assert_eq!(batch.counts.unchanged, 1);
        assert_eq!(batch.counts.new, 1);
        assert_eq!(batch.counts.changed, 1);
        assert_eq!(batch.counts.missing, 2);
        assert_eq!(batch.counts.conflicts, 1);

        let conflict = row(&batch, "EE-ONB");
        assert_eq!(conflict.local_group.as_deref(), Some("onboarding"));
        assert_eq!(
            conflict
                .local
                .as_ref()
                .map(|local| local.full_name.as_str()),
            Some("Onboarding Person")
        );
        assert_eq!(conflict.source[0].full_name, "Onboarding Person HR");
        assert!(row(&batch, "EE-FLOAT").is_floating);
        assert!(row(&batch, "EE-NEW").local.is_none());
    }

    #[test]
    fn stage_normalises_source_values() {
        let mut conn = open_test_connection();
        let source = vec![MssqlStaffRecord {
            code: " ee-1 ".to_string(),
            name: "  Ann  ".to_string(),
            nick_name: Some("   ".to_string()),
            work_email: Some(" Ann@Example.COM ".to_string()),
            azure_ad_account: Some(" ann@corp ".to_string()),
        }];
        let batch = stage_employee_sync_batch_conn(&mut conn, &source, false, None).expect("stage");
        let staged = row(&batch, "EE-1");
        assert_eq!(
            staged.source,
            vec![EmployeeSyncFields {
                full_name: "Ann".to_string(),
                nick_name: None,
                email: Some("ann@example.com".to_string()),
                azure_ad_account: Some("ann@corp".to_string()),
            }]
        );
    }

    #[test]
    fn stage_missing_row_reports_assets_held() {
        let mut conn = open_test_connection();
        let employee = seed(&conn, "EE-HOLDER", "Holder", "employee_list", false);
        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status) VALUES ('REQ-1', ?1, 'EE-HOLDER', 'Holder', 'approved')",
            params![employee],
        )
        .expect("seed request");
        let request = conn.last_insert_rowid();
        for (code, returned) in [("LT-1", None), ("LT-2", None), ("LT-3", Some("2026-11-01"))] {
            conn.execute(
                "INSERT INTO assets(asset_code, asset_type, display_name) VALUES (?1, 'Laptop', ?1)",
                params![code],
            )
            .expect("seed asset");
            let asset = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, returned_at) VALUES (?1, ?2, ?3, ?4)",
                params![asset, employee, request, returned],
            )
            .expect("seed loan");
        }

        let batch =
            stage_employee_sync_batch_conn(&mut conn, &[hr("EE-OTHER", "Other")], false, None)
                .expect("stage");

        let missing = row(&batch, "EE-HOLDER");
        assert_eq!(missing.kind, "missing");
        assert_eq!(missing.assets_held, 2);
        assert_eq!(missing.local_employee_id, Some(employee));
    }

    #[test]
    fn stage_refuses_empty_hr_source_when_local_list_exists() {
        let mut conn = open_test_connection();
        // An empty local EE List is refused too: an empty HR result is never a usable list.
        assert_eq!(
            stage_employee_sync_batch_conn(&mut conn, &[], false, None),
            Err("Cannot load: the HR source returned no employees".to_string())
        );

        seed(&conn, "EE-1", "One", "employee_list", false);
        assert_eq!(
            stage_employee_sync_batch_conn(&mut conn, &[], true, None),
            Err("Cannot load: the HR source returned no employees".to_string())
        );
        assert_eq!(table_count(&conn, "employee_sync_batches"), 0);
        assert_eq!(table_count(&conn, "employee_sync_rows"), 0);
    }

    #[test]
    fn stage_duplicate_hr_code_is_single_skip_only_row() {
        let mut conn = open_test_connection();
        let source = vec![
            hr("EE-DUP", "First"),
            hr("ee-dup", "Second"),
            hr("EE-OK", "Ok"),
        ];

        let batch = stage_employee_sync_batch_conn(&mut conn, &source, false, None).expect("stage");

        let duplicates: Vec<_> = batch
            .rows
            .iter()
            .filter(|row| row.staff_code == "EE-DUP")
            .collect();
        assert_eq!(duplicates.len(), 1);
        let duplicate = duplicates[0];
        assert_eq!(duplicate.kind, "conflict");
        assert_eq!(duplicate.action, "skip");
        assert_eq!(duplicate.source.len(), 2);
        assert!(duplicate
            .notes
            .iter()
            .any(|note| note == "Duplicate Staff Code in HR source"));
        assert_eq!(row(&batch, "EE-OK").kind, "new");
        assert_eq!(batch.counts.conflicts, 1);
        assert_eq!(batch.counts.new, 1);
    }

    #[test]
    fn stage_refuses_when_open_batch_exists_unless_replace() {
        let mut conn = open_test_connection();
        let source = vec![hr("EE-1", "One")];
        stage_employee_sync_batch_conn(&mut conn, &source, false, None).expect("first stage");

        let err = stage_employee_sync_batch_conn(&mut conn, &source, false, None)
            .expect_err("second stage refused");
        assert!(
            err.starts_with("Cannot load: an EE List sync is already open"),
            "{err}"
        );

        let replaced =
            stage_employee_sync_batch_conn(&mut conn, &source, true, None).expect("replace stage");
        assert_eq!(table_count(&conn, "employee_sync_batches"), 1);
        assert_eq!(table_count(&conn, "employee_sync_rows"), 1);
        let open = load_open_employee_sync_batch_conn(&conn)
            .expect("load")
            .expect("open batch");
        assert_eq!(open.id, replaced.id);
    }

    #[test]
    fn discard_deletes_batch_and_rows() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-GONE", "Gone", "employee_list", false);
        let batch = stage_employee_sync_batch_conn(&mut conn, &[hr("EE-1", "One")], false, None)
            .expect("stage");

        discard_employee_sync_batch_conn(&mut conn, batch.id).expect("discard");

        assert_eq!(table_count(&conn, "employee_sync_batches"), 0);
        assert_eq!(table_count(&conn, "employee_sync_rows"), 0);
        assert_eq!(
            discard_employee_sync_batch_conn(&mut conn, batch.id),
            Err("Not found: EE List sync".to_string())
        );
        assert_eq!(load_open_employee_sync_batch_conn(&conn), Ok(None));
    }

    #[test]
    fn load_open_batch_round_trips_rows() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-CHANGED", "Old Name", "employee_list", false);
        seed(&conn, "EE-ONB", "Onboarding", "onboarding", false);
        seed(&conn, "EE-GONE", "Gone", "employee_list", true);
        let source = vec![
            hr("EE-NEW", "New"),
            hr("EE-CHANGED", "New Name"),
            hr("EE-ONB", "Onboarding"),
            hr("EE-DUP", "A"),
            hr("EE-DUP", "B"),
        ];
        let staged = stage_employee_sync_batch_conn(&mut conn, &source, false, Some("admin"))
            .expect("stage");

        let loaded = load_open_employee_sync_batch_conn(&conn)
            .expect("load")
            .expect("open batch");

        assert_eq!(loaded, staged);
    }
}
