//! EE List sync staging: HR records are loaded into a persisted, editable batch
//! (`employee_sync_batches` / `employee_sync_rows`) instead of being written straight
//! to `employees`; Approve writes the reviewed rows in one transaction.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::employee::{move_employees_group_tx, MoveEmployeesGroupInput};
use super::humanize_sqlite_error;
use super::mssql_import::{
    compare_employee_list, insert_employee_list_record, normalize_sync_text,
    normalized_staff_code, update_employee_list_record, EmployeeSyncCounts,
    LocalEmployeeListRecord, MssqlStaffRecord,
};
use super::schema::{
    AZURE_AD_ACCOUNT_FIELD_KEY, STAFF_GROUP_EMPLOYEE_LIST, STAFF_GROUP_OFFBOARDING,
};

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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncRowUpdate {
    pub action: Option<String>,
    /// `Some` replaces the stored edits; `None` keeps them.
    pub edits: Option<EmployeeSyncFields>,
}

const DUPLICATE_CODE_NOTE: &str = "Duplicate Staff Code in HR source";
const STALE_NOTE: &str = "Changed in the app after this list was loaded — review again";

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

fn normalise_fields(fields: &EmployeeSyncFields) -> EmployeeSyncFields {
    EmployeeSyncFields {
        full_name: fields.full_name.trim().to_string(),
        nick_name: normalize_sync_text(fields.nick_name.as_deref()),
        email: normalize_sync_text(fields.email.as_deref()).map(|value| value.to_lowercase()),
        azure_ad_account: normalize_sync_text(fields.azure_ad_account.as_deref()),
    }
}

fn source_fields(record: &MssqlStaffRecord) -> EmployeeSyncFields {
    normalise_fields(&EmployeeSyncFields {
        full_name: record.name.clone(),
        nick_name: record.nick_name.clone(),
        email: record.work_email.clone(),
        azure_ad_account: record.azure_ad_account.clone(),
    })
}

fn default_action(kind: &str, is_floating: bool) -> &'static str {
    match kind {
        "conflict" => "move_to_employee_list",
        "missing" if is_floating => "none",
        "missing" => "move_to_offboarding",
        _ => "apply",
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
        let (kind, local) = match compared.status.as_str() {
            "unchanged" => {
                unchanged += 1;
                continue;
            }
            "new" => ("new", None),
            "changed" => ("changed", ee_by_code.get(&code).copied()),
            "conflict" => ("conflict", other_by_code.get(&code).copied()),
            other => return Err(format!("Invalid EE List sync row kind: {other}")),
        };
        new_rows.push(NewRow {
            staff_code: code,
            kind,
            local,
            source,
            action: default_action(kind, false),
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
            action: default_action("missing", local.is_floating),
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

const ROW_COLUMNS: &str = "id, staff_code, kind, local_employee_id, local_group, source_json, \
     local_json, edited_json, action, notes_json, state, assets_held, is_floating";

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<EmployeeSyncStagedRow> {
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
}

fn load_row(conn: &Connection, row_id: i64) -> Result<EmployeeSyncStagedRow, String> {
    conn.query_row(
        &format!("SELECT {ROW_COLUMNS} FROM employee_sync_rows WHERE id = ?1"),
        [row_id],
        map_row,
    )
    .optional()
    .map_err(humanize_sqlite_error)?
    .ok_or_else(|| format!("Not found: EE List sync row {row_id}"))
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
        .prepare(&format!(
            "SELECT {ROW_COLUMNS} FROM employee_sync_rows WHERE batch_id = ?1 ORDER BY id"
        ))
        .map_err(humanize_sqlite_error)?;
    let rows = statement
        .query_map([batch_id], map_row)
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

fn is_duplicate(row: &EmployeeSyncStagedRow) -> bool {
    row.notes.iter().any(|note| note == DUPLICATE_CODE_NOTE)
}

fn allowed_actions(row: &EmployeeSyncStagedRow) -> &'static [&'static str] {
    if is_duplicate(row) {
        return &["skip"];
    }
    match row.kind.as_str() {
        "new" | "changed" => &["apply", "skip"],
        "conflict" => &["move_to_employee_list", "skip"],
        "missing" => &["move_to_offboarding", "none"],
        _ => &[],
    }
}

pub(crate) fn update_employee_sync_row_conn(
    conn: &mut Connection,
    row_id: i64,
    update: EmployeeSyncRowUpdate,
) -> Result<EmployeeSyncStagedRow, String> {
    let row = load_row(conn, row_id)?;
    let action = update.action.unwrap_or_else(|| row.action.clone());
    if !allowed_actions(&row).contains(&action.as_str()) {
        return Err(format!("Invalid action for {} row: {action}", row.kind));
    }
    let edits = update
        .edits
        .as_ref()
        .map(normalise_fields)
        .or_else(|| row.edits.clone());
    let writes = matches!(action.as_str(), "apply" | "move_to_employee_list");
    // The written values are the edits when present, else the HR record.
    if writes
        && edits
            .as_ref()
            .or(row.source.first())
            .is_some_and(|fields| fields.full_name.is_empty())
    {
        return Err("Invalid full name: required".to_string());
    }
    conn.execute(
        "UPDATE employee_sync_rows SET action = ?1, edited_json = ?2 WHERE id = ?3",
        params![action, edits.as_ref().map(to_json).transpose()?, row_id],
    )
    .map_err(humanize_sqlite_error)?;
    load_row(conn, row_id)
}

pub(crate) fn refresh_employee_sync_row_conn(
    conn: &mut Connection,
    row_id: i64,
) -> Result<EmployeeSyncStagedRow, String> {
    let row = load_row(conn, row_id)?;
    let snapshot = read_local_snapshot_conn(conn)?;
    let by_id = row
        .local_employee_id
        .and_then(|id| snapshot.iter().find(|local| local.id == id));
    // HR rows without a (surviving) local match look the code up again, preferring the
    // EE List employee like staging does. A missing row has nothing to look up.
    let local = by_id.or_else(|| {
        if row.source.is_empty() {
            return None;
        }
        let same_code = || {
            snapshot
                .iter()
                .filter(|local| normalized_staff_code(&local.code) == row.staff_code)
        };
        same_code()
            .filter(|local| local.staff_group == STAFF_GROUP_EMPLOYEE_LIST)
            .last()
            .or_else(|| same_code().next())
    });
    let kind = match local {
        // Duplicates stay skip-only conflicts; a missing row stays missing.
        _ if is_duplicate(&row) || row.kind == "missing" => row.kind.as_str(),
        None => "new",
        Some(local) if local.staff_group == STAFF_GROUP_EMPLOYEE_LIST => "changed",
        Some(_) => "conflict",
    };
    let action = if row.kind == "missing" && local.is_none() {
        // The employee is gone: there is no one left to offboard.
        "none"
    } else if kind != row.kind {
        default_action(kind, local.is_some_and(|local| local.is_floating))
    } else {
        row.action.as_str()
    };
    let notes: Vec<&String> = row
        .notes
        .iter()
        .filter(|note| *note != STALE_NOTE)
        .collect();
    conn.execute(
        r#"UPDATE employee_sync_rows
           SET kind = ?1, local_employee_id = ?2, local_group = ?3, local_json = ?4,
               assets_held = ?5, is_floating = ?6, action = ?7, notes_json = ?8, state = 'pending'
           WHERE id = ?9"#,
        params![
            kind,
            local.map(|local| local.id),
            local.map(|local| local.staff_group.as_str()),
            local.map(to_json).transpose()?,
            local.map_or(0, |local| local.assets_held),
            local.is_some_and(|local| local.is_floating),
            action,
            to_json(&notes)?,
            row_id,
        ],
    )
    .map_err(humanize_sqlite_error)?;
    load_row(conn, row_id)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncApplyResult {
    pub added: usize,
    pub updated: usize,
    pub moved_to_employee_list: usize,
    pub offboarded: usize,
    pub skipped: usize,
    pub completed_at: String,
}

fn writes(row: &EmployeeSyncStagedRow) -> bool {
    matches!(
        row.action.as_str(),
        "apply" | "move_to_employee_list" | "move_to_offboarding"
    )
}

/// The values a writing row puts on the employee: the edits when present, else the HR record.
fn written_fields(row: &EmployeeSyncStagedRow) -> Result<&EmployeeSyncFields, String> {
    row.edits
        .as_ref()
        .or(row.source.first())
        .filter(|fields| !fields.full_name.trim().is_empty())
        .ok_or_else(|| "Invalid full name: required".to_string())
}

/// True when the app changed what this writing row was staged against.
fn is_stale(
    row: &EmployeeSyncStagedRow,
    staged_local: Option<&str>,
    snapshot: &[LocalSnapshot],
) -> Result<bool, String> {
    if row.kind == "new" {
        return Ok(snapshot
            .iter()
            .any(|local| normalized_staff_code(&local.code) == row.staff_code));
    }
    let current = row
        .local_employee_id
        .and_then(|id| snapshot.iter().find(|local| local.id == id));
    let (Some(current), Some(staged)) = (current, staged_local) else {
        return Ok(true);
    };
    // Every fingerprint field counts: Azure AD edits do not bump updated_at.
    let staged: serde_json::Value = serde_json::from_str(staged)
        .map_err(|err| format!("failed to read EE List sync row: {err}"))?;
    let current = serde_json::to_value(current)
        .map_err(|err| format!("failed to encode EE List sync row: {err}"))?;
    Ok(staged != current)
}

fn write_row(
    tx: &rusqlite::Transaction<'_>,
    row: &EmployeeSyncStagedRow,
    result: &mut EmployeeSyncApplyResult,
) -> Result<(), String> {
    let move_to = |target: &str, id: i64| {
        move_employees_group_tx(
            tx,
            &MoveEmployeesGroupInput {
                employee_ids: vec![id],
                target_staff_group: target.to_string(),
                action: None,
                reason: None,
            },
        )
    };
    // The drift check guarantees a local employee for every non-new writing row.
    let local_id = row
        .local_employee_id
        .ok_or_else(|| format!("Cannot apply: {} has no local employee", row.staff_code));
    match (row.kind.as_str(), row.action.as_str()) {
        ("new", "apply") => {
            insert_employee_list_record(tx, &row.staff_code, written_fields(row)?)?;
            result.added += 1;
        }
        ("changed", "apply") => {
            update_employee_list_record(tx, local_id?, written_fields(row)?)?;
            result.updated += 1;
        }
        ("conflict", "move_to_employee_list") => {
            let id = local_id?;
            move_to(STAFF_GROUP_EMPLOYEE_LIST, id)?;
            update_employee_list_record(tx, id, written_fields(row)?)?;
            result.moved_to_employee_list += 1;
        }
        ("missing", "move_to_offboarding") => {
            move_to(STAFF_GROUP_OFFBOARDING, local_id?)?;
            result.offboarded += 1;
        }
        _ => result.skipped += 1,
    }
    Ok(())
}

pub(crate) fn apply_employee_sync_batch_conn(
    conn: &mut Connection,
    batch_id: i64,
    actor_ref: Option<&str>,
) -> Result<EmployeeSyncApplyResult, String> {
    let tx = conn.transaction().map_err(humanize_sqlite_error)?;
    let exists = tx
        .query_row(
            "SELECT 1 FROM employee_sync_batches WHERE id = ?1",
            [batch_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(humanize_sqlite_error)?;
    if exists.is_none() {
        return Err("Not found: EE List sync".to_string());
    }
    // local_json is read raw: it holds the full drift fingerprint (group, updatedAt).
    let rows: Vec<(EmployeeSyncStagedRow, Option<String>)> = {
        let mut statement = tx
            .prepare(&format!(
                "SELECT {ROW_COLUMNS}, local_json FROM employee_sync_rows WHERE batch_id = ?1 ORDER BY id"
            ))
            .map_err(humanize_sqlite_error)?;
        let rows = statement
            .query_map([batch_id], |row| Ok((map_row(row)?, row.get(13)?)))
            .map_err(humanize_sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("failed to read EE List sync row: {err}"))?;
        rows
    };
    if !rows.iter().any(|(row, _)| writes(row)) {
        return Err("Cannot apply: nothing to write".to_string());
    }
    // Refresh can leave a writing row with a blank edited name: check before any write.
    for (row, _) in &rows {
        if matches!(row.action.as_str(), "apply" | "move_to_employee_list") {
            written_fields(row)?;
        }
    }

    let snapshot = read_local_snapshot_conn(&tx)?;
    let mut stale = 0usize;
    for (row, staged_local) in rows.iter().filter(|(row, _)| writes(row)) {
        if !is_stale(row, staged_local.as_deref(), &snapshot)? {
            continue;
        }
        stale += 1;
        let mut notes = row.notes.clone();
        if !notes.iter().any(|note| note == STALE_NOTE) {
            notes.push(STALE_NOTE.to_string());
        }
        tx.execute(
            "UPDATE employee_sync_rows SET state = 'stale', notes_json = ?1 WHERE id = ?2",
            params![to_json(&notes)?, row.id],
        )
        .map_err(humanize_sqlite_error)?;
    }
    if stale > 0 {
        // All-or-nothing: commit only the stale marks so the owner can review them.
        tx.commit().map_err(humanize_sqlite_error)?;
        return Err(format!(
            "Cannot apply: {stale} rows changed in the app — review them"
        ));
    }

    let mut result = EmployeeSyncApplyResult::default();
    for (row, _) in &rows {
        // Dropping tx on error rolls every write back; the batch stays.
        write_row(&tx, row, &mut result).map_err(|err| {
            if err.starts_with("Cannot") || err.starts_with("Invalid") {
                err
            } else {
                format!("Cannot apply: {err}")
            }
        })?;
    }
    let audit_payload = serde_json::json!({
        "added": result.added,
        "updated": result.updated,
        "movedToEmployeeList": result.moved_to_employee_list,
        "offboarded": result.offboarded,
        "skipped": result.skipped,
        "status": "SUCCESS",
    })
    .to_string();
    super::audit::insert_audit_log_tx(
        &tx,
        "business.import.mssql",
        "account",
        actor_ref,
        "sync",
        "mssql_employee_list",
        Some(&audit_payload),
    )?;
    // Rows go with their batch through ON DELETE CASCADE.
    tx.execute("DELETE FROM employee_sync_batches WHERE id = ?1", [batch_id])
        .map_err(humanize_sqlite_error)?;
    tx.commit().map_err(humanize_sqlite_error)?;
    result.completed_at = chrono::Utc::now().to_rfc3339();
    Ok(result)
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

    fn stage(conn: &mut Connection, source: &[MssqlStaffRecord]) -> EmployeeSyncBatch {
        stage_employee_sync_batch_conn(conn, source, false, None).expect("stage")
    }

    fn fields(full_name: &str) -> EmployeeSyncFields {
        EmployeeSyncFields {
            full_name: full_name.to_string(),
            nick_name: None,
            email: None,
            azure_ad_account: None,
        }
    }

    fn set_action(action: &str) -> EmployeeSyncRowUpdate {
        EmployeeSyncRowUpdate {
            action: Some(action.to_string()),
            edits: None,
        }
    }

    #[test]
    fn update_row_sets_action_and_edits() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-1", "Old", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-1", "New")]);
        let id = row(&batch, "EE-1").id;

        let updated = update_employee_sync_row_conn(
            &mut conn,
            id,
            EmployeeSyncRowUpdate {
                action: Some("skip".to_string()),
                edits: Some(EmployeeSyncFields {
                    full_name: "  Edited  ".to_string(),
                    nick_name: Some("  ".to_string()),
                    email: Some(" Ed@Example.COM ".to_string()),
                    azure_ad_account: Some(" ed@corp ".to_string()),
                }),
            },
        )
        .expect("update");

        assert_eq!(updated.action, "skip");
        assert_eq!(
            updated.edits,
            Some(EmployeeSyncFields {
                full_name: "Edited".to_string(),
                nick_name: None,
                email: Some("ed@example.com".to_string()),
                azure_ad_account: Some("ed@corp".to_string()),
            })
        );
        let loaded = load_open_employee_sync_batch_conn(&conn)
            .expect("load")
            .expect("open");
        assert_eq!(row(&loaded, "EE-1"), &updated);

        // Edits are kept when only the action changes.
        let updated =
            update_employee_sync_row_conn(&mut conn, id, set_action("apply")).expect("update");
        assert_eq!(updated.action, "apply");
        assert_eq!(
            updated.edits.map(|edits| edits.full_name),
            Some("Edited".to_string())
        );

        assert_eq!(
            update_employee_sync_row_conn(&mut conn, 9999, set_action("skip")),
            Err("Not found: EE List sync row 9999".to_string())
        );
    }

    #[test]
    fn update_row_rejects_action_not_allowed_for_kind() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-GONE", "Gone", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-1", "One")]);
        let id = row(&batch, "EE-GONE").id;

        assert_eq!(
            update_employee_sync_row_conn(&mut conn, id, set_action("apply")),
            Err("Invalid action for missing row: apply".to_string())
        );
        assert_eq!(
            update_employee_sync_row_conn(&mut conn, id, set_action("none"))
                .expect("none allowed")
                .action,
            "none"
        );
    }

    #[test]
    fn update_row_rejects_blank_full_name_for_writing_row() {
        let mut conn = open_test_connection();
        let batch = stage(&mut conn, &[hr("EE-NEW", "New")]);
        let id = row(&batch, "EE-NEW").id;
        let blank = EmployeeSyncRowUpdate {
            action: None,
            edits: Some(fields("   ")),
        };

        assert_eq!(
            update_employee_sync_row_conn(&mut conn, id, blank.clone()),
            Err("Invalid full name: required".to_string())
        );
        // A skipped row writes nothing, so a blank name is allowed there.
        let skipped = update_employee_sync_row_conn(
            &mut conn,
            id,
            EmployeeSyncRowUpdate {
                action: Some("skip".to_string()),
                ..blank
            },
        )
        .expect("skip with blank name");
        assert_eq!(
            skipped.edits.map(|edits| edits.full_name),
            Some(String::new())
        );
        // Switching back to a writing action re-checks the stored edits.
        assert_eq!(
            update_employee_sync_row_conn(&mut conn, id, set_action("apply")),
            Err("Invalid full name: required".to_string())
        );
    }

    #[test]
    fn update_duplicate_row_only_allows_skip() {
        let mut conn = open_test_connection();
        let batch = stage(&mut conn, &[hr("EE-DUP", "A"), hr("EE-DUP", "B")]);
        let id = row(&batch, "EE-DUP").id;

        for action in ["move_to_employee_list", "apply"] {
            assert_eq!(
                update_employee_sync_row_conn(&mut conn, id, set_action(action)),
                Err(format!("Invalid action for conflict row: {action}"))
            );
        }
        assert_eq!(
            update_employee_sync_row_conn(&mut conn, id, set_action("skip"))
                .expect("skip")
                .action,
            "skip"
        );
    }

    #[test]
    fn refresh_stale_row_resets_state_keeps_edits() {
        let mut conn = open_test_connection();
        let employee = seed(&conn, "EE-1", "Old", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-1", "New")]);
        let id = row(&batch, "EE-1").id;
        update_employee_sync_row_conn(
            &mut conn,
            id,
            EmployeeSyncRowUpdate {
                action: Some("skip".to_string()),
                edits: Some(fields("Edited")),
            },
        )
        .expect("update");
        conn.execute(
            "UPDATE employee_sync_rows SET state = 'stale', notes_json = ?1 WHERE id = ?2",
            params![to_json(&[STALE_NOTE]).unwrap(), id],
        )
        .expect("mark stale");
        conn.execute(
            "UPDATE employees SET full_name = 'Renamed In App', is_floating = 1 WHERE id = ?1",
            [employee],
        )
        .expect("edit employee");

        let refreshed = refresh_employee_sync_row_conn(&mut conn, id).expect("refresh");

        assert_eq!(refreshed.state, "pending");
        assert!(refreshed.notes.is_empty());
        assert_eq!(refreshed.kind, "changed");
        assert_eq!(refreshed.action, "skip", "kind unchanged keeps the action");
        assert_eq!(refreshed.edits, Some(fields("Edited")));
        assert_eq!(
            refreshed.local.map(|local| local.full_name),
            Some("Renamed In App".to_string())
        );
        assert!(refreshed.is_floating);
        let local_json: String = conn
            .query_row(
                "SELECT local_json FROM employee_sync_rows WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("local_json");
        assert!(
            local_json.contains("\"staffGroup\":\"employee_list\""),
            "{local_json}"
        );
        assert!(local_json.contains("\"updatedAt\""), "{local_json}");

        assert_eq!(
            refresh_employee_sync_row_conn(&mut conn, 9999),
            Err("Not found: EE List sync row 9999".to_string())
        );
    }

    #[test]
    fn refresh_new_row_whose_code_now_exists_becomes_changed_or_conflict() {
        let mut conn = open_test_connection();
        let batch = stage(&mut conn, &[hr("EE-A", "A"), hr("EE-B", "B")]);
        let a = row(&batch, "EE-A").id;
        let b = row(&batch, "EE-B").id;
        let onboarding = seed(&conn, " ee-a", "A Local", "onboarding", false);
        let listed = seed(&conn, "EE-B", "B Local", "employee_list", false);

        let conflict = refresh_employee_sync_row_conn(&mut conn, a).expect("refresh a");
        assert_eq!(conflict.kind, "conflict");
        assert_eq!(conflict.action, "move_to_employee_list");
        assert_eq!(conflict.local_employee_id, Some(onboarding));
        assert_eq!(conflict.local_group.as_deref(), Some("onboarding"));

        let changed = refresh_employee_sync_row_conn(&mut conn, b).expect("refresh b");
        assert_eq!(changed.kind, "changed");
        assert_eq!(changed.action, "apply");
        assert_eq!(changed.local_employee_id, Some(listed));
        assert_eq!(
            changed.local.map(|local| local.full_name),
            Some("B Local".to_string())
        );
    }

    #[test]
    fn refresh_row_whose_employee_was_deleted() {
        let mut conn = open_test_connection();
        let changed = seed(&conn, "EE-1", "Old", "employee_list", false);
        let gone = seed(&conn, "EE-GONE", "Gone", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-1", "New")]);
        conn.execute(
            "DELETE FROM employees WHERE id IN (?1, ?2)",
            [changed, gone],
        )
        .expect("delete employees");

        let became_new =
            refresh_employee_sync_row_conn(&mut conn, row(&batch, "EE-1").id).expect("refresh");
        assert_eq!(
            (became_new.kind.as_str(), became_new.action.as_str()),
            ("new", "apply")
        );
        assert_eq!(became_new.local_employee_id, None);
        assert_eq!(became_new.local, None);

        let missing =
            refresh_employee_sync_row_conn(&mut conn, row(&batch, "EE-GONE").id).expect("refresh");
        assert_eq!(
            (missing.kind.as_str(), missing.action.as_str()),
            ("missing", "none")
        );
        assert_eq!(missing.local, None);
        assert_eq!(missing.local_group, None);
    }

    fn employee(conn: &Connection, code: &str) -> Option<(i64, String, Option<String>, String, Option<String>)> {
        conn.query_row(
            "SELECT id, full_name, email, staff_group, lifecycle_status FROM employees WHERE employee_id = ?1",
            [code],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .expect("read employee")
    }

    fn employees_dump(conn: &Connection) -> String {
        conn.query_row(
            "SELECT COALESCE(group_concat(employee_id || '|' || full_name || '|' || IFNULL(email, '') || '|' || staff_group || '|' || updated_at, ';'), '') FROM (SELECT * FROM employees ORDER BY id)",
            [],
            |row| row.get(0),
        )
        .expect("dump employees")
    }

    fn stale_message(n: usize) -> String {
        format!("Cannot apply: {n} rows changed in the app — review them")
    }

    #[test]
    fn apply_writes_new_changed_with_edits_and_deletes_batch() {
        let mut conn = open_test_connection();
        let changed = seed(&conn, "EE-1", "Old", "employee_list", false);
        let mut new_hr = hr("EE-NEW", "New");
        new_hr.azure_ad_account = Some("new@corp".to_string());
        let batch = stage(&mut conn, &[new_hr, hr("EE-1", "New Name")]);
        update_employee_sync_row_conn(
            &mut conn,
            row(&batch, "EE-NEW").id,
            EmployeeSyncRowUpdate {
                action: None,
                edits: Some(EmployeeSyncFields {
                    full_name: " Edited New ".to_string(),
                    nick_name: None,
                    email: Some(" New@Example.COM ".to_string()),
                    azure_ad_account: Some("edited@corp".to_string()),
                }),
            },
        )
        .expect("edit new row");

        let result =
            apply_employee_sync_batch_conn(&mut conn, batch.id, Some("admin")).expect("apply");

        assert_eq!(
            (result.added, result.updated, result.moved_to_employee_list, result.offboarded, result.skipped),
            (1, 1, 0, 0, 0)
        );
        assert!(!result.completed_at.is_empty());
        let (new_id, name, email, group, _) = employee(&conn, "EE-NEW").expect("inserted");
        assert_eq!(name, "Edited New");
        assert_eq!(email.as_deref(), Some("new@example.com"));
        assert_eq!(group, "employee_list");
        let azure: String = conn
            .query_row(
                "SELECT value FROM employee_dynamic_values WHERE employee_id = ?1 AND field_key = ?2",
                params![new_id, AZURE_AD_ACCOUNT_FIELD_KEY],
                |row| row.get(0),
            )
            .expect("azure");
        assert_eq!(azure, "edited@corp");
        let (id, name, ..) = employee(&conn, "EE-1").expect("changed");
        assert_eq!((id, name.as_str()), (changed, "New Name"));
        assert_eq!(table_count(&conn, "employee_sync_batches"), 0);
        assert_eq!(table_count(&conn, "employee_sync_rows"), 0);
        let audits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE event_type = 'business.import.mssql' AND actor_ref = 'admin'",
                [],
                |row| row.get(0),
            )
            .expect("audit count");
        assert_eq!(audits, 1);
    }

    #[test]
    fn apply_conflict_move_keeps_employee_id_and_loans() {
        let mut conn = open_test_connection();
        let onboarding = seed(&conn, "EE-ONB", "Onb Local", "onboarding", false);
        let offboarded = seed(&conn, "EE-OFF", "Off Local", "offboarding", false);
        conn.execute(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status) VALUES ('REQ-1', ?1, 'EE-OFF', 'Off', 'approved')",
            params![offboarded],
        )
        .expect("seed request");
        let request = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO assets(asset_code, asset_type, display_name) VALUES ('LT-1', 'Laptop', 'LT-1')",
            [],
        )
        .expect("seed asset");
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES (?1, ?2, ?3)",
            params![conn.last_insert_rowid(), offboarded, request],
        )
        .expect("seed loan");
        let batch = stage(&mut conn, &[hr("EE-ONB", "Onb HR"), hr("EE-OFF", "Off HR")]);

        let result = apply_employee_sync_batch_conn(&mut conn, batch.id, None).expect("apply");

        assert_eq!(result.moved_to_employee_list, 2);
        let (id, name, _, group, lifecycle) = employee(&conn, "EE-ONB").expect("onb");
        assert_eq!((id, name.as_str(), group.as_str()), (onboarding, "Onb HR", "employee_list"));
        assert_eq!(lifecycle.as_deref(), Some("active"));
        let (id, name, _, group, lifecycle) = employee(&conn, "EE-OFF").expect("off");
        assert_eq!((id, name.as_str(), group.as_str()), (offboarded, "Off HR", "employee_list"));
        assert_eq!(lifecycle.as_deref(), Some("rehired"));
        let loans: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE employee_id_fk = ?1",
                [offboarded],
                |row| row.get(0),
            )
            .expect("loans");
        assert_eq!(loans, 1);
    }

    #[test]
    fn apply_missing_offboard_sets_resigned() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-GONE", "Gone", "employee_list", false);
        seed(&conn, "EE-FLOAT", "Float", "employee_list", true);
        let batch = stage(&mut conn, &[hr("EE-OTHER", "Other")]);

        let result = apply_employee_sync_batch_conn(&mut conn, batch.id, None).expect("apply");

        assert_eq!((result.added, result.offboarded, result.skipped), (1, 1, 1));
        let (_, _, _, group, lifecycle) = employee(&conn, "EE-GONE").expect("gone");
        assert_eq!(group, "offboarding");
        assert_eq!(lifecycle.as_deref(), Some("resigned"));
        let (_, _, _, group, lifecycle) = employee(&conn, "EE-FLOAT").expect("float");
        assert_eq!(group, "employee_list");
        assert_eq!(lifecycle, None);
    }

    #[test]
    fn apply_missing_row_whose_employee_left_ee_list_is_stale() {
        let mut conn = open_test_connection();
        let gone = seed(&conn, "EE-GONE", "Gone", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-OTHER", "Other")]);
        conn.execute("UPDATE employees SET staff_group = 'internal_movement' WHERE id = ?1", [gone])
            .expect("move locally");

        assert_eq!(
            apply_employee_sync_batch_conn(&mut conn, batch.id, None),
            Err(stale_message(1))
        );
        assert_eq!(employee(&conn, "EE-OTHER"), None);
        assert_eq!(employee(&conn, "EE-GONE").expect("gone").3, "internal_movement");
    }

    #[test]
    fn apply_skipped_rows_write_nothing() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-1", "Old", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-1", "New"), hr("EE-NEW", "N")]);
        update_employee_sync_row_conn(&mut conn, row(&batch, "EE-1").id, set_action("skip"))
            .expect("skip");

        let result = apply_employee_sync_batch_conn(&mut conn, batch.id, None).expect("apply");

        assert_eq!((result.added, result.updated, result.skipped), (1, 0, 1));
        assert_eq!(employee(&conn, "EE-1").expect("kept").1, "Old");
    }

    #[test]
    fn apply_changed_row_edited_locally_after_staging_is_stale() {
        let mut conn = open_test_connection();
        let id = seed(&conn, "EE-1", "Old", "employee_list", false);
        let batch = stage(&mut conn, &[hr("EE-1", "New"), hr("EE-NEW", "N")]);
        // updated_at is not bumped: every fingerprint field counts.
        conn.execute("UPDATE employees SET nick_name = 'Local' WHERE id = ?1", [id])
            .expect("local edit");

        for _ in 0..2 {
            assert_eq!(
                apply_employee_sync_batch_conn(&mut conn, batch.id, None),
                Err(stale_message(1))
            );
        }

        assert_eq!(employee(&conn, "EE-1").expect("kept").1, "Old");
        assert_eq!(employee(&conn, "EE-NEW"), None);
        let open = load_open_employee_sync_batch_conn(&conn)
            .expect("load")
            .expect("batch kept");
        let stale = row(&open, "EE-1");
        assert_eq!(stale.state, "stale");
        assert_eq!(stale.notes, vec![STALE_NOTE.to_string()]);
        assert_eq!(row(&open, "EE-NEW").state, "pending");
    }

    #[test]
    fn apply_new_row_code_added_locally_is_stale() {
        let mut conn = open_test_connection();
        let batch = stage(&mut conn, &[hr("EE-NEW", "New")]);
        seed(&conn, " ee-new", "Local", "onboarding", false);

        assert_eq!(
            apply_employee_sync_batch_conn(&mut conn, batch.id, None),
            Err(stale_message(1))
        );
        assert_eq!(table_count(&conn, "employees"), 1);
        let open = load_open_employee_sync_batch_conn(&conn)
            .expect("load")
            .expect("batch kept");
        assert_eq!(row(&open, "EE-NEW").state, "stale");
    }

    #[test]
    fn apply_error_rolls_back_and_keeps_batch() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-X", "Holder", "employee_list", false);
        conn.execute("UPDATE employees SET email = 'taken@example.com'", [])
            .expect("seed email");
        let mut holder = hr("EE-X", "Holder");
        holder.work_email = Some("taken@example.com".to_string());
        // EE-X is unchanged, so only EE-A and EE-B are staged.
        let batch = stage(&mut conn, &[holder, hr("EE-A", "A"), hr("EE-B", "B")]);
        // The unique email clash is not a drift: it only surfaces on insert.
        update_employee_sync_row_conn(
            &mut conn,
            row(&batch, "EE-B").id,
            EmployeeSyncRowUpdate {
                action: None,
                edits: Some(EmployeeSyncFields {
                    email: Some("taken@example.com".to_string()),
                    ..fields("B")
                }),
            },
        )
        .expect("edit");
        let before = employees_dump(&conn);

        let err = apply_employee_sync_batch_conn(&mut conn, batch.id, None).expect_err("fails");

        assert!(err.starts_with("Cannot apply: "), "{err}");
        assert_eq!(employees_dump(&conn), before);
        assert_eq!(table_count(&conn, "employee_sync_batches"), 1);
        assert_eq!(table_count(&conn, "employee_sync_rows"), 2);
        assert_eq!(table_count(&conn, "audit_logs"), 0);
    }

    #[test]
    fn apply_rejects_blank_full_name_on_writing_row() {
        let mut conn = open_test_connection();
        let batch = stage(&mut conn, &[hr("EE-NEW", "New")]);
        conn.execute(
            "UPDATE employee_sync_rows SET edited_json = ?1",
            [to_json(&fields(" ")).unwrap()],
        )
        .expect("blank edit");

        assert_eq!(
            apply_employee_sync_batch_conn(&mut conn, batch.id, None),
            Err("Invalid full name: required".to_string())
        );
        assert_eq!(table_count(&conn, "employees"), 0);
        assert_eq!(table_count(&conn, "employee_sync_batches"), 1);
    }

    #[test]
    fn apply_nothing_to_write_is_refused() {
        let mut conn = open_test_connection();
        seed(&conn, "EE-FLOAT", "Float", "employee_list", true);
        let batch = stage(&mut conn, &[hr("EE-1", "One")]);
        update_employee_sync_row_conn(&mut conn, row(&batch, "EE-1").id, set_action("skip"))
            .expect("skip");

        assert_eq!(
            apply_employee_sync_batch_conn(&mut conn, batch.id, None),
            Err("Cannot apply: nothing to write".to_string())
        );
        assert_eq!(table_count(&conn, "employee_sync_batches"), 1);
        assert_eq!(
            apply_employee_sync_batch_conn(&mut conn, 9999, None),
            Err("Not found: EE List sync".to_string())
        );
    }
}
