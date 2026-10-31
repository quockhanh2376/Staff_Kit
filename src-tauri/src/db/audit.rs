use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};

use super::humanize_sqlite_error;
use crate::auth_session::Role;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogRecord {
    pub id: i64,
    pub event_type: String,
    pub actor_type: String,
    pub actor_ref: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub payload_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryAuditLogsInput {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub event_category: Option<String>, // "all", "security", "business"
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryAuditLogsResult {
    pub logs: Vec<AuditLogRecord>,
    pub total_count: i64,
}

pub(crate) fn insert_audit_log_tx(
    tx: &Transaction<'_>,
    event_type: &str,
    actor_type: &str,
    actor_ref: Option<&str>,
    entity_type: &str,
    entity_id: &str,
    payload_json: Option<&str>,
) -> Result<(), String> {
    tx.execute(
        r#"
        INSERT INTO audit_logs(
          event_type,
          actor_type,
          actor_ref,
          entity_type,
          entity_id,
          payload_json,
          created_at
        )
        VALUES(?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            event_type,
            actor_type,
            actor_ref,
            entity_type,
            entity_id,
            payload_json
        ],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(())
}

pub(crate) fn insert_audit_log_conn(
    conn: &Connection,
    event_type: &str,
    actor_type: &str,
    actor_ref: Option<&str>,
    entity_type: &str,
    entity_id: &str,
    payload_json: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO audit_logs(
          event_type,
          actor_type,
          actor_ref,
          entity_type,
          entity_id,
          payload_json,
          created_at
        )
        VALUES(?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            event_type,
            actor_type,
            actor_ref,
            entity_type,
            entity_id,
            payload_json
        ],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(())
}

pub(crate) fn query_audit_logs(
    conn: &Connection,
    role: Role,
    input: QueryAuditLogsInput,
) -> Result<QueryAuditLogsResult, String> {
    let limit = input.limit.unwrap_or(50).clamp(1, 200);
    let offset = input.offset.unwrap_or(0).max(0);
    let search_pattern = input
        .search
        .as_deref()
        .map(|s| format!("%{}%", s.trim().to_lowercase()));

    // Server-side role enforcement (D3.7–D3.9 / SA-2):
    // SuperAdmin can view all events (security + business + legacy).
    // Admin is strictly constrained to business.* events only.
    let is_super_admin = role == Role::SuperAdmin;

    let mut conditions = Vec::new();
    let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();

    if !is_super_admin {
        // Non-super admin can ONLY see business.* events
        conditions.push("event_type LIKE 'business.%'".to_string());
    } else if let Some(cat) = input.event_category.as_deref() {
        match cat.to_lowercase().as_str() {
            "security" => {
                conditions.push("(event_type LIKE 'security.%' OR (event_type NOT LIKE 'business.%'))".to_string());
            }
            "business" => {
                conditions.push("event_type LIKE 'business.%'".to_string());
            }
            _ => {
                // "all" or unrecognized: no category filter
            }
        }
    }

    if let Some(pattern) = search_pattern {
        if !pattern.replace('%', "").trim().is_empty() {
            conditions.push(
                "(LOWER(event_type) LIKE ? OR LOWER(COALESCE(actor_ref, '')) LIKE ? OR LOWER(entity_id) LIKE ? OR LOWER(COALESCE(payload_json, '')) LIKE ?)"
                    .to_string(),
            );
            params_vec.push(pattern.clone().into());
            params_vec.push(pattern.clone().into());
            params_vec.push(pattern.clone().into());
            params_vec.push(pattern.into());
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM audit_logs {}", where_clause);
    let total_count: i64 = {
        let mut count_stmt = conn.prepare(&count_sql).map_err(humanize_sqlite_error)?;
        count_stmt
            .query_row(rusqlite::params_from_iter(params_vec.iter()), |row| row.get(0))
            .map_err(humanize_sqlite_error)?
    };

    let query_sql = format!(
        "SELECT id, event_type, actor_type, actor_ref, entity_type, entity_id, payload_json, created_at \
         FROM audit_logs {} ORDER BY id DESC LIMIT ? OFFSET ?",
        where_clause
    );

    let mut query_params = params_vec;
    query_params.push(limit.into());
    query_params.push(offset.into());

    let mut stmt = conn.prepare(&query_sql).map_err(humanize_sqlite_error)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(query_params.iter()), |row| {
            Ok(AuditLogRecord {
                id: row.get(0)?,
                event_type: row.get(1)?,
                actor_type: row.get(2)?,
                actor_ref: row.get(3)?,
                entity_type: row.get(4)?,
                entity_id: row.get(5)?,
                payload_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(humanize_sqlite_error)?;

    let mut logs = Vec::new();
    for row in rows {
        logs.push(row.map_err(humanize_sqlite_error)?);
    }

    Ok(QueryAuditLogsResult { logs, total_count })
}

pub(crate) fn prune_audit_logs_older_than(
    conn: &Connection,
    retention_days: i64,
) -> Result<usize, String> {
    if retention_days < 1095 {
        return Err("audit retention policy enforces a minimum of 1095 days (3 years)".to_string());
    }

    let changed = conn
        .execute(
            "DELETE FROM audit_logs WHERE created_at < datetime('now', '-' || ? || ' days')",
            params![retention_days],
        )
        .map_err(humanize_sqlite_error)?;

    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_audit_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE audit_logs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              event_type TEXT NOT NULL,
              actor_type TEXT NOT NULL,
              actor_ref TEXT,
              entity_type TEXT NOT NULL,
              entity_id TEXT NOT NULL,
              payload_json TEXT,
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TRIGGER IF NOT EXISTS audit_logs_no_update
            BEFORE UPDATE ON audit_logs
            BEGIN
              SELECT RAISE(ABORT, 'audit_logs entries are immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS audit_logs_no_delete
            BEFORE DELETE ON audit_logs
            FOR EACH ROW
            WHEN OLD.created_at >= datetime('now', '-1095 days')
            BEGIN
              SELECT RAISE(ABORT, 'audit_logs entries cannot be deleted within the 3-year retention period');
            END;
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_audit_logs_are_immutable() {
        let conn = setup_test_audit_db();
        insert_audit_log_conn(
            &conn,
            "security.auth.login_success",
            "account",
            Some("super_admin"),
            "session",
            "1",
            Some(r#"{"status":"SUCCESS"}"#),
        )
        .unwrap();

        let update_res = conn.execute("UPDATE audit_logs SET event_type = 'tampered' WHERE id = 1", []);
        assert!(update_res.is_err(), "Direct update to audit_logs must be aborted by trigger");
        let err_msg = update_res.unwrap_err().to_string();
        assert!(err_msg.contains("immutable"), "Error must state audit_logs entries are immutable: {}", err_msg);
    }

    #[test]
    fn test_3_year_retention_blocks_deletion_of_recent_logs() {
        let conn = setup_test_audit_db();
        insert_audit_log_conn(
            &conn,
            "security.auth.login_success",
            "account",
            Some("super_admin"),
            "session",
            "1",
            None,
        )
        .unwrap();

        // Attempt to delete recent record (< 1095 days)
        let delete_res = conn.execute("DELETE FROM audit_logs WHERE id = 1", []);
        assert!(delete_res.is_err(), "Recent audit_logs deletion must be aborted by retention trigger");
        let err_msg = delete_res.unwrap_err().to_string();
        assert!(err_msg.contains("3-year retention period"), "Error must state retention period violation: {}", err_msg);
    }

    #[test]
    fn test_3_year_retention_allows_pruning_records_older_than_1095_days() {
        let conn = setup_test_audit_db();

        // Insert record backdated to 4 years ago (1460 days)
        conn.execute(
            r#"
            INSERT INTO audit_logs(event_type, actor_type, actor_ref, entity_type, entity_id, payload_json, created_at)
            VALUES('business.export.file', 'account', 'admin_1', 'export', 'data.csv', '{}', datetime('now', '-1460 days'))
            "#,
            [],
        )
        .unwrap();

        // Also insert a fresh record
        insert_audit_log_conn(
            &conn,
            "security.auth.login_success",
            "account",
            Some("super_admin"),
            "session",
            "1",
            None,
        )
        .unwrap();

        // Prune records older than 1095 days (minimum allowed)
        let pruned = prune_audit_logs_older_than(&conn, 1095).expect("prune older than 3 years succeeds");
        assert_eq!(pruned, 1, "Exactly the 4-year-old record should be pruned");

        // Fresh record remains
        let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, 1, "Recent record must be preserved");

        // Reject pruning with retention parameter < 1095 days
        assert!(prune_audit_logs_older_than(&conn, 30).is_err(), "Pruning with < 1095 days must be rejected");
    }

    #[test]
    fn test_query_audit_logs_role_enforcement() {
        let conn = setup_test_audit_db();

        // Insert a security event
        insert_audit_log_conn(
            &conn,
            "security.auth.login_success",
            "account",
            Some("super_admin"),
            "session",
            "1",
            Some(r#"{"status":"SUCCESS"}"#),
        )
        .unwrap();

        // Insert a business event
        insert_audit_log_conn(
            &conn,
            "business.import.excel",
            "account",
            Some("admin_user"),
            "import",
            "staff.xlsx",
            Some(r#"{"inserted":10}"#),
        )
        .unwrap();

        // Insert a legacy event (unprefixed)
        insert_audit_log_conn(
            &conn,
            "legacy_event_type",
            "system",
            None,
            "system",
            "0",
            None,
        )
        .unwrap();

        // 1. SuperAdmin queries all
        let sa_all = query_audit_logs(&conn, Role::SuperAdmin, QueryAuditLogsInput::default()).unwrap();
        assert_eq!(sa_all.total_count, 3, "Super Admin sees all 3 events");

        // 2. SuperAdmin filters security
        let sa_sec = query_audit_logs(
            &conn,
            Role::SuperAdmin,
            QueryAuditLogsInput {
                event_category: Some("security".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(sa_sec.total_count, 2, "Super Admin sees security + legacy events under security");

        // 3. SuperAdmin filters business
        let sa_biz = query_audit_logs(
            &conn,
            Role::SuperAdmin,
            QueryAuditLogsInput {
                event_category: Some("business".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(sa_biz.total_count, 1, "Super Admin sees business event under business");

        // 4. Admin query is ALWAYS forced to business.* regardless of requested category
        let admin_res = query_audit_logs(
            &conn,
            Role::Admin,
            QueryAuditLogsInput {
                event_category: Some("security".to_string()), // attempting to request security!
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(admin_res.total_count, 1, "Admin is forced to business.* only");
        assert_eq!(admin_res.logs[0].event_type, "business.import.excel");
    }
}
