//! One-time laptop baseline (2026-11-26, owner request).
//!
//! Reads the "Employees" sheet of the IT laptop list (EE. ID, Computer Name, Computer (2), Status)
//! and makes the app match it:
//! - a computer listed next to an existing employee is assigned to that employee;
//! - every other laptop is returned to the IT store (in stock, office), Broken ones stay Broken;
//! - computers in the sheet that the app does not know are created (Laptop category, name only);
//! - the 377 monitors wrongly created from person names on 2026-11-25 are deleted.
//!
//! Run with the app closed: `laptop_baseline <db> <xlsx>` prints the plan, `--apply` writes it
//! in one transaction after a safety snapshot.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;

use super::{asset, asset_import, audit};

/// The bad monitor import wrote all its rows in this one second.
const JUNK_MONITOR_CREATED_AT: &str = "2026-11-25 08:14:35";
/// Owner-confirmed fixes to sheet typos: (staff id, computer in sheet) -> real computer.
const SHEET_FIXES: &[(&str, &str, &str)] = &[("ASWVN992", "ASWVNLAP1", "ASWVNLAP237")];
const STORE_LOCATION: &str = "office";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetEntry {
    pub row_number: usize,
    pub staff_id: String,
    pub computer: String,
    pub status: String,
}

#[derive(Debug, Default)]
pub struct BaselineReport {
    pub junk_monitors_deleted: usize,
    pub created: Vec<String>,
    pub assigned: Vec<String>,
    pub kept: usize,
    pub reassigned: Vec<String>,
    pub returned_to_store: Vec<String>,
    pub set_broken: Vec<String>,
    pub unknown_employees: Vec<String>,
    pub conflicts: Vec<String>,
}

impl BaselineReport {
    pub fn print(&self) {
        println!("Junk monitors deleted:   {}", self.junk_monitors_deleted);
        println!("Computers created:       {}", self.created.len());
        println!("Assigned (new loan):     {}", self.assigned.len());
        println!("Already correct (kept):  {}", self.kept);
        println!("Moved to another person: {}", self.reassigned.len());
        for line in &self.reassigned {
            println!("  {line}");
        }
        println!("Returned to IT store:    {}", self.returned_to_store.len());
        for line in &self.returned_to_store {
            println!("  {line}");
        }
        println!("Marked Broken:           {}", self.set_broken.len());
        println!("Staff ID not in app (computer left in store): {}", self.unknown_employees.len());
        for line in &self.unknown_employees {
            println!("  {line}");
        }
        println!("Conflicts (skipped):     {}", self.conflicts.len());
        for line in &self.conflicts {
            println!("  {line}");
        }
    }
}

fn normalize(value: &str) -> String {
    value.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

fn cell_text(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.trim().to_string(),
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Float(f)) => f.to_string(),
        _ => String::new(),
    }
}

/// One entry per computer cell (Computer Name and Computer (2)) of the "Employees" sheet.
pub fn parse_laptop_sheet(path: &Path) -> Result<Vec<SheetEntry>, String> {
    let mut workbook =
        open_workbook_auto(path).map_err(|err| format!("Cannot open workbook '{}': {err}", path.display()))?;
    let range = workbook
        .worksheet_range("Employees")
        .map_err(|err| format!("Not found: sheet 'Employees' ({err})"))?;
    let (row_offset, _) = range.start().unwrap_or((0, 0));

    let rows: Vec<&[Data]> = range.rows().collect();
    let header_index = rows
        .iter()
        .position(|row| {
            let cells: Vec<String> = row.iter().map(|c| normalize(&cell_text(Some(c)))).collect();
            cells.iter().any(|c| c == "eeid") && cells.iter().any(|c| c == "computername")
        })
        .ok_or("Invalid sheet: header row with 'EE. ID' and 'Computer Name' not found")?;
    let header: Vec<String> = rows[header_index].iter().map(|c| normalize(&cell_text(Some(c)))).collect();
    let col = |name: &str| header.iter().position(|c| c == name);
    let staff_col = col("eeid").ok_or("Invalid sheet: no 'EE. ID' column")?;
    let computer_cols: Vec<usize> = ["computername", "computer2"].iter().filter_map(|name| col(name)).collect();
    let status_col = col("status");

    let mut entries = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(header_index + 1) {
        let staff_id = cell_text(row.get(staff_col)).to_uppercase();
        let status = status_col.map(|c| cell_text(row.get(c))).unwrap_or_default();
        for &c in &computer_cols {
            let computer = cell_text(row.get(c)).to_uppercase();
            if computer.is_empty() {
                continue;
            }
            let computer = SHEET_FIXES
                .iter()
                .find(|(staff, typo, _)| *staff == staff_id && *typo == computer)
                .map(|(_, _, real)| real.to_string())
                .unwrap_or(computer);
            entries.push(SheetEntry {
                row_number: row_offset as usize + index + 1,
                staff_id: staff_id.clone(),
                computer,
                status: status.clone(),
            });
        }
    }
    Ok(entries)
}

/// "ASWVNLAP254" -> "VNLAP254", the app's asset code (display name keeps the full name).
fn asset_code_for(computer: &str) -> String {
    computer.strip_prefix("ASW").unwrap_or(computer).to_string()
}

fn delete_junk_monitors_tx(tx: &Transaction<'_>) -> Result<usize, String> {
    let junk = "SELECT id FROM assets WHERE asset_type = 'Monitor' AND created_at = ?1 \
                AND asset_code LIKE 'VNMON%' AND SUBSTR(asset_code, 6) NOT GLOB '[0-9]*'";
    let requests = format!(
        "SELECT borrow_request_id FROM borrow_request_items WHERE asset_id IN ({junk}) \
         EXCEPT SELECT borrow_request_id FROM borrow_request_items WHERE asset_id NOT IN ({junk})"
    );
    let err = |e: rusqlite::Error| format!("Cannot delete junk monitors: {e}");
    let request_ids: Vec<i64> = {
        let mut stmt = tx.prepare(&requests).map_err(err)?;
        let ids = stmt
            .query_map(params![JUNK_MONITOR_CREATED_AT], |r| r.get(0))
            .map_err(err)?
            .collect::<Result<Vec<i64>, _>>()
            .map_err(err)?;
        ids
    };
    tx.execute(&format!("DELETE FROM asset_loans WHERE asset_id IN ({junk})"), params![JUNK_MONITOR_CREATED_AT])
        .map_err(err)?;
    tx.execute(
        &format!("DELETE FROM borrow_request_items WHERE asset_id IN ({junk})"),
        params![JUNK_MONITOR_CREATED_AT],
    )
    .map_err(err)?;
    for id in &request_ids {
        tx.execute("DELETE FROM borrow_request_confirmations WHERE borrow_request_id = ?", params![id])
            .map_err(err)?;
        tx.execute("DELETE FROM borrow_requests WHERE id = ?", params![id]).map_err(err)?;
    }
    tx.execute("UPDATE asset_import_rows SET imported_asset_id = NULL WHERE imported_asset_id IN (SELECT id FROM assets WHERE asset_type = 'Monitor' AND created_at = ?1 AND asset_code LIKE 'VNMON%' AND SUBSTR(asset_code, 6) NOT GLOB '[0-9]*')", params![JUNK_MONITOR_CREATED_AT])
        .map_err(err)?;
    tx.execute(&format!("DELETE FROM assets WHERE id IN ({junk})"), params![JUNK_MONITOR_CREATED_AT])
        .map_err(err)
}

struct Employee {
    id: i64,
    staff_id: String,
    full_name: String,
}

/// Applies the baseline inside `tx`. The caller commits (apply) or drops it (dry run).
pub fn apply_baseline_tx(tx: &Transaction<'_>, entries: &[SheetEntry]) -> Result<BaselineReport, String> {
    let mut report = BaselineReport::default();
    report.junk_monitors_deleted = delete_junk_monitors_tx(tx)?;

    let laptop_category = asset::ensure_category_exists_or_create_tx(tx, "Laptop", "serialized")?;
    let find_employee = |staff_id: &str| -> Result<Option<Employee>, String> {
        tx.query_row(
            "SELECT id, employee_id, full_name FROM employees WHERE UPPER(TRIM(employee_id)) = ?",
            params![staff_id],
            |r| Ok(Employee { id: r.get(0)?, staff_id: r.get(1)?, full_name: r.get(2)? }),
        )
        .optional()
        .map_err(|e| format!("Cannot read employee '{staff_id}': {e}"))
    };

    // computer -> wanted holder; computer -> sheet status (for store rows).
    let mut wanted: BTreeMap<String, Employee> = BTreeMap::new();
    let mut store_status: HashMap<String, String> = HashMap::new();
    for entry in entries {
        if entry.staff_id.is_empty() {
            store_status.entry(entry.computer.clone()).or_insert_with(|| entry.status.clone());
            continue;
        }
        let Some(employee) = find_employee(&entry.staff_id)? else {
            report.unknown_employees.push(format!("row {}: {} ({})", entry.row_number, entry.staff_id, entry.computer));
            store_status.entry(entry.computer.clone()).or_insert_with(String::new);
            continue;
        };
        if let Some(other) = wanted.get(&entry.computer) {
            if other.id != employee.id {
                report.conflicts.push(format!(
                    "{} listed for {} and {}",
                    entry.computer, other.staff_id, employee.staff_id
                ));
            }
            continue;
        }
        wanted.insert(entry.computer.clone(), employee);
    }
    for computer in report.conflicts.iter().filter_map(|c| c.split(' ').next()) {
        wanted.remove(computer);
    }

    let find_asset = |computer: &str| -> Result<Option<i64>, String> {
        tx.query_row(
            "SELECT id FROM assets WHERE asset_code = ?1 COLLATE NOCASE OR display_name = ?2 COLLATE NOCASE ORDER BY asset_code = ?1 COLLATE NOCASE DESC LIMIT 1",
            params![asset_code_for(computer), computer],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| format!("Cannot read asset '{computer}': {e}"))
    };
    let current_holder = |asset_id: i64| -> Result<Option<(i64, String)>, String> {
        tx.query_row(
            "SELECT l.employee_id_fk, e.employee_id FROM asset_loans l JOIN employees e ON e.id = l.employee_id_fk WHERE l.asset_id = ? AND l.returned_at IS NULL",
            params![asset_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("Cannot read loan for asset {asset_id}: {e}"))
    };
    let close_loan = |asset_id: i64| -> Result<(), String> {
        tx.execute(
            "UPDATE asset_loans SET returned_at = datetime('now') WHERE asset_id = ? AND returned_at IS NULL",
            params![asset_id],
        )
        .map(|_| ())
        .map_err(|e| format!("Cannot close loan for asset {asset_id}: {e}"))
    };
    let create = |computer: &str, report: &mut BaselineReport| -> Result<i64, String> {
        let record = asset::create_asset_tx(
            tx,
            &asset::AssetUpsertInput {
                asset_code: asset_code_for(computer),
                category_id: Some(laptop_category.id),
                asset_type: "Laptop".to_string(),
                display_name: computer.to_string(),
                display_name_short: None,
                brand: None,
                model: None,
                serial_number: None,
                usage_location: None,
                adapter_number: None,
                warehouse: None,
                notes: Some("Created by laptop baseline 2026-11-26".to_string()),
                condition: Some("spare".to_string()),
            },
        )?;
        report.created.push(computer.to_string());
        Ok(record.id)
    };

    let mut handled: HashSet<i64> = HashSet::new();
    for (computer, employee) in &wanted {
        let asset_id = match find_asset(computer)? {
            Some(id) => id,
            None => create(computer, &mut report)?,
        };
        handled.insert(asset_id);
        match current_holder(asset_id)? {
            Some((holder, _)) if holder == employee.id => {
                report.kept += 1;
                continue;
            }
            Some((_, holder_staff)) => {
                close_loan(asset_id)?;
                report.reassigned.push(format!("{computer}: {holder_staff} -> {}", employee.staff_id));
            }
            None => report.assigned.push(format!("{computer} -> {}", employee.staff_id)),
        }
        asset_import::create_owner_aware_import_loan_tx(
            tx,
            asset_id,
            &asset_code_for(computer),
            employee.id,
            &employee.staff_id,
            &employee.full_name,
            None,
        )?;
    }

    // Everything else goes to the store: sheet rows without a holder, then every other laptop in the app.
    // By asset id: some laptops carry the model as their name (VNLAP523 "Lenovo ThinkPad E14").
    let mut store: Vec<(i64, String, String)> = Vec::new();
    let mut store_rows: Vec<(String, String)> = store_status.into_iter().collect();
    store_rows.sort();
    for (computer, status) in store_rows {
        if wanted.contains_key(&computer) {
            continue;
        }
        let asset_id = match find_asset(&computer)? {
            Some(id) => id,
            None => create(&computer, &mut report)?,
        };
        if handled.insert(asset_id) {
            store.push((asset_id, computer, status));
        }
    }
    {
        let mut stmt = tx
            .prepare("SELECT id, asset_code FROM assets WHERE category_id = ? OR asset_type = 'Laptop' ORDER BY asset_code")
            .map_err(|e| format!("Cannot list laptops: {e}"))?;
        let laptops = stmt
            .query_map(params![laptop_category.id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| format!("Cannot list laptops: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Cannot list laptops: {e}"))?;
        for (asset_id, code) in laptops {
            if handled.insert(asset_id) {
                store.push((asset_id, code, String::new()));
            }
        }
    }
    for (asset_id, computer, status) in &store {
        let asset_id = *asset_id;
        if let Some((_, holder_staff)) = current_holder(asset_id)? {
            close_loan(asset_id)?;
            report.returned_to_store.push(format!("{computer} (was {holder_staff})"));
        }
        asset::set_asset_status_tx(tx, asset_id, "in_stock")?;
        let broken = normalize(status) == "broken";
        tx.execute(
            "UPDATE assets SET usage_location = ?, condition = CASE WHEN ? THEN 'broken' ELSE condition END, updated_at = datetime('now') WHERE id = ?",
            params![STORE_LOCATION, broken, asset_id],
        )
        .map_err(|e| format!("Cannot move {computer} to store: {e}"))?;
        if broken {
            report.set_broken.push(computer.clone());
        }
    }

    let payload = json!({
        "junkMonitorsDeleted": report.junk_monitors_deleted,
        "created": report.created,
        "assigned": report.assigned,
        "kept": report.kept,
        "reassigned": report.reassigned,
        "returnedToStore": report.returned_to_store,
        "unknownEmployees": report.unknown_employees,
        "conflicts": report.conflicts,
    })
    .to_string();
    audit::insert_audit_log_tx(tx, "laptop_baseline.apply", "system", None, "assets", "laptop_baseline", Some(&payload))?;
    Ok(report)
}

/// `laptop_baseline <db path> <xlsx path> [--apply]`; returns the process exit code.
pub fn run_cli(args: &[String]) -> i32 {
    let (Some(db), Some(xlsx)) = (args.get(1), args.get(2)) else {
        eprintln!("Usage: laptop_baseline <staff_kit.sqlite3> <laptop list.xlsx> [--apply]");
        return 2;
    };
    let apply = args.iter().any(|a| a == "--apply");
    match run(Path::new(db), Path::new(xlsx), apply) {
        Ok(report) => {
            report.print();
            println!("{}", if apply { "APPLIED." } else { "DRY RUN: nothing written. Re-run with --apply to write." });
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn run(db: &Path, xlsx: &Path, apply: bool) -> Result<BaselineReport, String> {
    let entries = parse_laptop_sheet(xlsx)?;
    if apply {
        let snapshot = super::backup::create_mandatory_safety_snapshot_for_db(db, "before_laptop_baseline")?;
        println!("Safety snapshot: {}", snapshot.full_path);
    }
    let mut conn: Connection = super::open_encrypted_connection(db)?;
    super::configure_connection(&conn)?;
    let tx = conn.transaction().map_err(|e| format!("Cannot start transaction: {e}"))?;
    let report = apply_baseline_tx(&tx, &entries)?;
    if apply {
        tx.commit().map_err(|e| format!("Cannot commit laptop baseline: {e}"))?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{apply_baseline_tx, SheetEntry};
    use crate::db::{apply_migrations, configure_connection};

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();
        apply_migrations(&conn).unwrap();
        conn
    }

    fn employee(conn: &Connection, staff_id: &str) -> i64 {
        conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES(?, ?, ?, 0) RETURNING id",
            params![staff_id, format!("Name {staff_id}"), format!("{staff_id}@x.com")],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn laptop(conn: &Connection, code: &str) -> i64 {
        conn.query_row(
            "INSERT INTO assets(asset_code, asset_type, display_name, status, category_id) VALUES(?, 'Laptop', ?, 'in_stock', (SELECT id FROM asset_categories WHERE category_code = 'laptop')) RETURNING id",
            params![code, format!("ASW{code}")],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn loan(conn: &Connection, asset_id: i64, employee_id: i64) {
        let request: i64 = conn
            .query_row(
                "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type) VALUES(hex(randomblob(8)), ?, 'x', 'x', 'approved', 'borrow') RETURNING id",
                params![employee_id],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO borrow_request_items(borrow_request_id, asset_id, asset_code_snapshot) VALUES(?, ?, 'x')",
            params![request, asset_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![asset_id, employee_id, request],
        )
        .unwrap();
        conn.execute("UPDATE assets SET status = 'assigned' WHERE id = ?", params![asset_id]).unwrap();
    }

    fn holder(conn: &Connection, code: &str) -> Option<String> {
        conn.query_row(
            "SELECT e.employee_id FROM assets a LEFT JOIN asset_loans l ON l.asset_id = a.id AND l.returned_at IS NULL LEFT JOIN employees e ON e.id = l.employee_id_fk WHERE a.asset_code = ?",
            params![code],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn asset_state(conn: &Connection, code: &str) -> (String, String, Option<String>) {
        conn.query_row(
            "SELECT status, condition, usage_location FROM assets WHERE asset_code = ?",
            params![code],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    }

    fn entry(staff_id: &str, computer: &str, status: &str) -> SheetEntry {
        SheetEntry { row_number: 1, staff_id: staff_id.to_string(), computer: computer.to_string(), status: status.to_string() }
    }

    #[test]
    fn baseline_assigns_keeps_moves_returns_creates_and_deletes_junk() {
        let mut conn = conn();
        let a = employee(&conn, "ASWVN1");
        let b = employee(&conn, "ASWVN2");
        let c = employee(&conn, "ASWVN3");
        let lap1 = laptop(&conn, "VNLAP001"); // free -> assign to a
        let lap2 = laptop(&conn, "VNLAP002"); // already b -> keep
        let lap3 = laptop(&conn, "VNLAP003"); // with a, sheet says c -> move
        let lap4 = laptop(&conn, "VNLAP004"); // with c, not assigned in sheet -> store
        let lap5 = laptop(&conn, "VNLAP523"); // named by model, sheet gives it to c -> keep
        conn.execute("UPDATE assets SET display_name = 'Lenovo ThinkPad E14' WHERE id = ?", params![lap5]).unwrap();
        loan(&conn, lap5, c);
        let _ = lap1;
        loan(&conn, lap2, b);
        loan(&conn, lap3, a);
        loan(&conn, lap4, c);
        // junk monitor from the bad import, and a real monitor that must stay
        let junk: i64 = conn
            .query_row(
                "INSERT INTO assets(asset_code, asset_type, display_name, status, created_at) VALUES('VNMONNAME ASWVN1', 'Monitor', 'Name ASWVN1', 'assigned', '2026-11-25 08:14:35') RETURNING id",
                [],
                |r| r.get(0),
            )
            .unwrap();
        loan(&conn, junk, a);
        conn.execute(
            "INSERT INTO assets(asset_code, asset_type, display_name, status, created_at) VALUES('VNMON530', 'Monitor', 'MON530', 'in_stock', '2026-11-25 08:14:35')",
            [],
        )
        .unwrap();

        let entries = vec![
            entry("ASWVN1", "ASWVNLAP001", "Assigned"),
            entry("ASWVN1", "ASWVNMACPRO009", "Assigned"), // second computer, unknown to the app
            entry("ASWVN2", "ASWVNLAP002", "Assigned"),
            entry("ASWVN3", "ASWVNLAP003", "Assigned"),
            entry("ASWVN3", "ASWVNLAP523", "Assigned"),
            entry("", "ASWVNLAP004", "Interview"),
            entry("", "ASWVNLAP068", "Broken"), // unknown to the app, broken
            entry("ASWVN999", "ASWVNLAP385", "Assigned"), // employee not in app
        ];
        let tx = conn.transaction().unwrap();
        let report = apply_baseline_tx(&tx, &entries).unwrap();
        tx.commit().unwrap();

        assert_eq!(report.junk_monitors_deleted, 1);
        assert_eq!(report.kept, 2);
        assert_eq!(report.reassigned, vec!["ASWVNLAP003: ASWVN1 -> ASWVN3".to_string()]);
        assert_eq!(report.returned_to_store, vec!["ASWVNLAP004 (was ASWVN3)".to_string()]);
        assert_eq!(report.unknown_employees.len(), 1);
        assert_eq!(holder(&conn, "VNLAP001").as_deref(), Some("ASWVN1"));
        assert_eq!(holder(&conn, "VNMACPRO009").as_deref(), Some("ASWVN1"));
        assert_eq!(holder(&conn, "VNLAP002").as_deref(), Some("ASWVN2"));
        assert_eq!(holder(&conn, "VNLAP003").as_deref(), Some("ASWVN3"));
        assert_eq!(holder(&conn, "VNLAP004"), None);
        assert_eq!(holder(&conn, "VNLAP523").as_deref(), Some("ASWVN3"));
        assert_eq!(asset_state(&conn, "VNLAP004"), ("in_stock".into(), "spare".into(), Some("office".into())));
        assert_eq!(asset_state(&conn, "VNLAP068"), ("in_stock".into(), "broken".into(), Some("office".into())));
        assert_eq!(asset_state(&conn, "VNLAP385").0, "in_stock");
        assert_eq!(asset_state(&conn, "VNMACPRO009").0, "assigned");
        let monitors: Vec<String> = conn
            .prepare("SELECT asset_code FROM assets WHERE asset_type = 'Monitor'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(monitors, vec!["VNMON530".to_string()]);
        let orphan_requests: i64 = conn
            .query_row("SELECT COUNT(*) FROM borrow_requests r WHERE NOT EXISTS (SELECT 1 FROM borrow_request_items i WHERE i.borrow_request_id = r.id)", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphan_requests, 0);
    }

    #[test]
    fn computer_listed_for_two_people_is_skipped() {
        let mut conn = conn();
        employee(&conn, "ASWVN1");
        employee(&conn, "ASWVN2");
        laptop(&conn, "VNLAP001");
        let entries = vec![entry("ASWVN1", "ASWVNLAP001", "Assigned"), entry("ASWVN2", "ASWVNLAP001", "Assigned")];
        let tx = conn.transaction().unwrap();
        let report = apply_baseline_tx(&tx, &entries).unwrap();
        tx.commit().unwrap();
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(holder(&conn, "VNLAP001"), None);
    }
}
