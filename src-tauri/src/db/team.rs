use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::{humanize_sqlite_error, normalize_optional_text, open_runtime_connection, require_text};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRecord {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub parent_name: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamUpsertInput {
    pub id: Option<i64>,
    pub name: String,
    pub parent_name: Option<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn list_teams(app: &AppHandle) -> Result<Vec<TeamRecord>, String> {
    let conn = open_runtime_connection(app)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              t.id,
              t.name,
              t.parent_id,
              p.name AS parent_name,
              COUNT(e.id) AS member_count
            FROM teams t
            LEFT JOIN teams p ON p.id = t.parent_id
            LEFT JOIN employees e ON e.team_id = t.id
            GROUP BY t.id, t.name, t.parent_id, p.name
            ORDER BY COALESCE(p.name, t.name) COLLATE NOCASE ASC, t.name COLLATE NOCASE ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare teams query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TeamRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                parent_name: row.get(3)?,
                member_count: row.get(4)?,
            })
        })
        .map_err(|err| format!("failed to query teams: {err}"))?;

    let mut teams = Vec::new();
    for row in rows {
        teams.push(row.map_err(|err| format!("failed to read team row: {err}"))?);
    }

    Ok(teams)
}

pub fn upsert_team(app: &AppHandle, payload: TeamUpsertInput) -> Result<TeamRecord, String> {
    let conn = open_runtime_connection(app)?;
    let normalized_name = require_text(payload.name, "name")?;
    let parent_name = normalize_optional_text(payload.parent_name);

    // Resolve parent_id from parent_name
    let parent_id: Option<i64> = if let Some(ref pname) = parent_name {
        conn.query_row(
            "SELECT id FROM teams WHERE name = ?",
            params![pname.as_str()],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|_| {
            // Parent doesn't exist yet — create it first
            conn.execute("INSERT OR IGNORE INTO teams(name) VALUES (?)", params![pname.as_str()])
                .map_err(humanize_sqlite_error)?;
            conn.query_row(
                "SELECT id FROM teams WHERE name = ?",
                params![pname.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map(Some)
            .map_err(|err| format!("failed to resolve parent team id: {err}"))
        })?
    } else {
        None
    };

    let team_id = if let Some(id) = payload.id {
        conn.execute(
            "UPDATE teams SET name = ?, parent_id = ? WHERE id = ?",
            params![normalized_name.as_str(), parent_id, id],
        )
        .map_err(humanize_sqlite_error)?;
        id
    } else {
        conn.execute(
            "INSERT INTO teams(name, parent_id) VALUES (?, ?) ON CONFLICT(name) DO UPDATE SET name = excluded.name, parent_id = excluded.parent_id",
            params![normalized_name.as_str(), parent_id],
        )
        .map_err(humanize_sqlite_error)?;

        conn.query_row(
            "SELECT id FROM teams WHERE name = ?",
            params![normalized_name.as_str()],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to resolve team id after upsert: {err}"))?
    };

    load_team_by_id(&conn, team_id)
}

pub fn delete_team(app: &AppHandle, id: i64) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;

    // Check how many members are in this team
    let in_use: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM employees WHERE team_id = ?",
            params![id],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to check team references: {err}"))?;

    if in_use > 0 {
        // Ensure ZZ_Floating exists (create if missing)
        conn.execute(
            "INSERT OR IGNORE INTO teams(name) VALUES ('ZZ_Floating')",
            [],
        )
        .map_err(humanize_sqlite_error)?;

        let floating_id: i64 = conn
            .query_row(
                "SELECT id FROM teams WHERE name = 'ZZ_Floating'",
                [],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to find ZZ_Floating team: {err}"))?;

        // Prevent deleting ZZ_Floating itself while it has members
        if floating_id == id {
            return Err(format!(
                "ZZ_Floating is currently assigned to {in_use} employee(s). Please re-assign them before removing this holding team."
            ));
        }

        // Move all members to ZZ_Floating
        conn.execute(
            "UPDATE employees SET team_id = ? WHERE team_id = ?",
            params![floating_id, id],
        )
        .map_err(|err| format!("failed to move employees to ZZ_Floating: {err}"))?;
    }

    let changed = conn
        .execute("DELETE FROM teams WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;

    Ok(changed > 0)
}

// ── Private helpers ───────────────────────────────────────────────────────────

pub(crate) fn load_team_by_id(conn: &Connection, id: i64) -> Result<TeamRecord, String> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        r#"
        SELECT
          t.id,
          t.name,
          t.parent_id,
          p.name AS parent_name,
          COUNT(e.id) AS member_count
        FROM teams t
        LEFT JOIN teams p ON p.id = t.parent_id
        LEFT JOIN employees e ON e.team_id = t.id
        WHERE t.id = ?
        GROUP BY t.id, t.name, t.parent_id, p.name
        "#,
        params![id],
        |row| {
            Ok(TeamRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                parent_name: row.get(3)?,
                member_count: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load team: {err}"))?
    .ok_or_else(|| format!("team with id {id} was not found"))
}

/// Resolve or create a team by name inside a transaction. Used by import.rs.
pub(crate) fn resolve_team_id_tx(
    tx: &rusqlite::Transaction<'_>,
    team_name: Option<&str>,
) -> Result<Option<i64>, String> {
    let Some(name) = team_name else {
        return Ok(None);
    };
    let name = name.trim();
    if name.is_empty() {
        return Ok(None);
    }

    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM teams WHERE name = ?",
            params![name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to query team: {err}"))?;

    if let Some(id) = existing {
        return Ok(Some(id));
    }

    tx.execute("INSERT INTO teams(name) VALUES (?)", params![name])
        .map_err(humanize_sqlite_error)?;

    Ok(Some(tx.last_insert_rowid()))
}
