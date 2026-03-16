use rusqlite::{params, Connection, Transaction};

use super::humanize_sqlite_error;

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
