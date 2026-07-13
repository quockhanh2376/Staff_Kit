use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use super::column::{upsert_dynamic_field_definitions_for_map, upsert_dynamic_fields_tx};
use super::employee::UpsertAction;
use super::schema::{AZURE_AD_ACCOUNT_FIELD_KEY, STAFF_GROUP_EMPLOYEE_LIST};
use super::{
    humanize_sqlite_error, normalize_optional_text, open_runtime_connection, require_text,
};

const MSSQL_HOST_ENV: &str = "STAFFKIT_MSSQL_HOST";
const MSSQL_PORT_ENV: &str = "STAFFKIT_MSSQL_PORT";
const MSSQL_USER_ENV: &str = "STAFFKIT_MSSQL_USER";
const MSSQL_PASSWORD_ENV: &str = "STAFFKIT_MSSQL_PASSWORD";

const DEFAULT_MSSQL_QUERY: &str = r#"
WITH StaffList AS
(
    SELECT
        S.[Code],
        S.[Name],
        S.[NickName],
        COALESCE(S.[WorkEmail], S.[WorkEmail2]) AS [WorkEmail],
        ASPU.UserName AS [AzureAD],
        ROW_NUMBER() OVER (
            PARTITION BY S.[Code]
            ORDER BY S.[Code]
        ) AS RowNum
    FROM [AssetManagement].[dbo].[Staffs] AS S
    INNER JOIN [AssetManagement].[dbo].[AspNetUsers] AS ASPU
        ON S.Id = ASPU.StaffId
    WHERE S.[Resigned] = 0
)
SELECT TOP (1000)
    [Code],
    [Name],
    [NickName],
    [WorkEmail],
    [AzureAD]
FROM StaffList
WHERE RowNum = 1
ORDER BY [Code]
"#;

pub fn build_connection_string(host: &str, port: u16, user: &str, password: &str) -> String {
    format!(
        "server=tcp:{host},{port};uid={user};pwd={password};encrypt=true;TrustServerCertificate=yes;"
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlConnectionDefaults {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

impl Default for MssqlConnectionDefaults {
    fn default() -> Self {
        let port = std::env::var(MSSQL_PORT_ENV)
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(1433);

        Self {
            host: std::env::var(MSSQL_HOST_ENV).unwrap_or_default(),
            port,
            user: std::env::var(MSSQL_USER_ENV).unwrap_or_default(),
            password: std::env::var(MSSQL_PASSWORD_ENV).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlStaffRecord {
    pub code: String,
    pub name: String,
    pub nick_name: Option<String>,
    pub work_email: Option<String>,
    pub azure_ad_account: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlImportPreview {
    pub total_rows: usize,
    pub records: Vec<MssqlStaffRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlImportReport {
    pub total_rows: usize,
    pub imported: u32,
    pub updated: u32,
    pub failed: u32,
    pub errors: Vec<MssqlImportError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlImportError {
    pub code: String,
    pub action: String,
    pub error: Option<String>,
}

type MssqlClient = Client<Compat<TcpStream>>;

fn format_mssql_connect_error(error: impl std::fmt::Display) -> String {
    let detail = error.to_string();
    let normalized = detail.to_ascii_lowercase();

    if normalized.contains("10054")
        || normalized.contains("forcibly closed")
        || normalized.contains("connection reset")
    {
        return format!(
            "MSSQL connection failed: the server closed the connection during TLS/authentication negotiation (Windows error 10054). Verify the SQL Server port, TLS/encryption settings, firewall, and SQL credentials. Details: {detail}"
        );
    }

    if normalized.contains("tls")
        || normalized.contains("handshake")
        || normalized.contains("certificate")
    {
        return format!(
            "MSSQL connection failed during TLS negotiation. Verify the SQL Server encryption settings and certificate configuration. Details: {detail}"
        );
    }

    format!("MSSQL authentication failed. Verify the SQL username and password. Details: {detail}")
}

fn append_import_log(app: &AppHandle, message: impl AsRef<str>) {
    let Ok(mut log_dir) = app.path().app_local_data_dir() else {
        eprintln!("[mssql_import] {}", message.as_ref());
        return;
    };

    log_dir.push("logs");
    if let Err(err) = fs::create_dir_all(&log_dir) {
        eprintln!(
            "[mssql_import] failed to create log directory {}: {err}",
            log_dir.to_string_lossy()
        );
        eprintln!("[mssql_import] {}", message.as_ref());
        return;
    }

    let log_path = log_dir.join("mssql-import.log");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(mut file) => {
            let _ = writeln!(file, "[{timestamp}] {}", message.as_ref());
        }
        Err(err) => {
            eprintln!(
                "[mssql_import] failed to open log file {}: {err}",
                log_path.to_string_lossy()
            );
            eprintln!("[mssql_import] {}", message.as_ref());
        }
    }
}

fn get_required_mssql_text(
    row: &tiberius::Row,
    index: usize,
    label: &str,
) -> Result<String, String> {
    if let Some(value) = row.get::<&str, _>(index) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(value) = row.get::<i32, _>(index) {
        return Ok(value.to_string());
    }

    if let Some(value) = row.get::<i64, _>(index) {
        return Ok(value.to_string());
    }

    Err(format!("MSSQL row missing {label} column"))
}

fn get_optional_mssql_text(row: &tiberius::Row, index: Option<usize>) -> Option<String> {
    let index = index?;
    if let Some(value) = row.get::<&str, _>(index) {
        return Some(value.trim().to_string()).filter(|value| !value.is_empty());
    }

    None
}

fn find_mssql_column_index(row: &tiberius::Row, aliases: &[&str]) -> Option<usize> {
    row.columns().iter().position(|column| {
        aliases
            .iter()
            .any(|alias| column.name().eq_ignore_ascii_case(alias))
    })
}

fn upsert_mssql_staff_record(
    tx: &Transaction<'_>,
    record: &MssqlStaffRecord,
    staff_group: &str,
) -> Result<UpsertAction, String> {
    let employee_id = require_text(record.code.clone(), "employeeId")?
        .trim()
        .to_uppercase();
    let full_name = require_text(record.name.clone(), "fullName")?;
    let email =
        normalize_optional_text(record.work_email.clone()).map(|value| value.to_lowercase());
    let nick_name = normalize_optional_text(record.nick_name.clone());

    let existing_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM employees WHERE employee_id = ?",
            params![employee_id.as_str()],
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
              nick_name = COALESCE(?, nick_name),
              email = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                full_name.as_str(),
                nick_name.as_deref(),
                email.as_deref(),
                id
            ],
        )
        .map_err(humanize_sqlite_error)?;

        upsert_mssql_azure_ad_account(tx, id, record.azure_ad_account.as_deref())?;

        return Ok(UpsertAction::Updated);
    }

    tx.execute(
        r#"
        INSERT INTO employees (
          employee_id,
          full_name,
          nick_name,
          email,
          staff_group,
          updated_at
        ) VALUES (
          ?, ?, ?, ?, ?, datetime('now')
        )
        "#,
        params![
            employee_id.as_str(),
            full_name.as_str(),
            nick_name.as_deref(),
            email.as_deref(),
            staff_group,
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let inserted_id = tx.last_insert_rowid();
    upsert_mssql_azure_ad_account(tx, inserted_id, record.azure_ad_account.as_deref())?;

    Ok(UpsertAction::Inserted)
}

fn upsert_mssql_azure_ad_account(
    tx: &Transaction<'_>,
    employee_id: i64,
    value: Option<&str>,
) -> Result<(), String> {
    let Some(value) = normalize_optional_text(value.map(str::to_string)) else {
        return Ok(());
    };

    let mut fields = HashMap::new();
    fields.insert(AZURE_AD_ACCOUNT_FIELD_KEY.to_string(), value);
    upsert_dynamic_field_definitions_for_map(tx, &fields)?;
    upsert_dynamic_fields_tx(tx, employee_id, &fields)
}

async fn connect(connection_string: &str) -> Result<MssqlClient, String> {
    let config = Config::from_ado_string(connection_string)
        .map_err(|err| format!("invalid connection string: {err}"))?;

    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|err| format!("failed to connect to MSSQL server: {err}"))?;

    tcp.set_nodelay(true).ok();

    let client = Client::connect(config, tcp.compat_write())
        .await
        .map_err(format_mssql_connect_error)?;

    Ok(client)
}

pub async fn get_mssql_connection_defaults() -> MssqlConnectionDefaults {
    MssqlConnectionDefaults::default()
}

pub async fn test_mssql_connection(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<bool, String> {
    let conn_str = build_connection_string(host, port, user, password);
    let mut client = connect(&conn_str).await?;
    let stream = client
        .query("SELECT 1 AS test", &[])
        .await
        .map_err(|err| format!("MSSQL query failed: {err}"))?;

    let _ = stream.into_results().await;
    Ok(true)
}

pub async fn preview_mssql_staff(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    query: Option<&str>,
) -> Result<MssqlImportPreview, String> {
    let conn_str = build_connection_string(host, port, user, password);
    let sql = query.unwrap_or(DEFAULT_MSSQL_QUERY);
    let mut client = connect(&conn_str).await?;

    let stream = client
        .query(sql, &[])
        .await
        .map_err(|err| format!("MSSQL query failed: {err}"))?;

    let rows = stream
        .into_first_result()
        .await
        .map_err(|err| format!("failed to read MSSQL results: {err}"))?;

    let mut records = Vec::with_capacity(rows.len());

    for row in &rows {
        let code_index = find_mssql_column_index(row, &["Code"])
            .ok_or_else(|| "MSSQL row missing Code column".to_string())?;
        let name_index = find_mssql_column_index(row, &["Name"])
            .ok_or_else(|| "MSSQL row missing Name column".to_string())?;
        let code = get_required_mssql_text(row, code_index, "Code")?;
        let name = get_required_mssql_text(row, name_index, "Name")?;
        let nick_name =
            get_optional_mssql_text(row, find_mssql_column_index(row, &["NickName", "Nickname"]));
        let email =
            get_optional_mssql_text(row, find_mssql_column_index(row, &["WorkEmail", "Email"]));
        let azure_ad_account = get_optional_mssql_text(
            row,
            find_mssql_column_index(row, &["AzureAD", "AzureAdAccount", "AzureADAccount"]),
        );

        records.push(MssqlStaffRecord {
            code,
            name,
            nick_name,
            work_email: email,
            azure_ad_account,
        });
    }

    Ok(MssqlImportPreview {
        total_rows: records.len(),
        records,
    })
}

pub async fn import_mssql_staff(
    app: &AppHandle,
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    query: Option<&str>,
    staff_group: Option<&str>,
) -> Result<MssqlImportReport, String> {
    let target_group = staff_group.unwrap_or(STAFF_GROUP_EMPLOYEE_LIST);
    append_import_log(
        app,
        format!(
            "start host={host} port={port} user={user} staff_group={target_group} query={}",
            if query.is_some() { "custom" } else { "default" }
        ),
    );

    let preview = match preview_mssql_staff(host, port, user, password, query).await {
        Ok(preview) => {
            append_import_log(
                app,
                format!("preview_ok total_rows={}", preview.records.len()),
            );
            preview
        }
        Err(err) => {
            append_import_log(app, format!("preview_failed error={err}"));
            return Err(format!("MSSQL import failed during preview: {err}"));
        }
    };

    let mut conn = match open_runtime_connection(app) {
        Ok(conn) => conn,
        Err(err) => {
            append_import_log(app, format!("sqlite_open_failed error={err}"));
            return Err(format!("MSSQL import failed before database write: {err}"));
        }
    };
    let tx = conn.transaction().map_err(|err| {
        append_import_log(app, format!("transaction_start_failed error={err}"));
        format!("MSSQL import failed to start database transaction: {err}")
    })?;

    let mut report = MssqlImportReport {
        total_rows: preview.records.len(),
        imported: 0,
        updated: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for record in &preview.records {
        match upsert_mssql_staff_record(&tx, record, target_group) {
            Ok(UpsertAction::Inserted) => report.imported += 1,
            Ok(UpsertAction::Updated) => report.updated += 1,
            Err(err) => {
                report.failed += 1;
                append_import_log(app, format!("row_failed code={} error={err}", record.code));
                report.errors.push(MssqlImportError {
                    code: record.code.clone(),
                    action: "failed".to_string(),
                    error: Some(err),
                });
            }
        }
    }

    tx.commit().map_err(|err| {
        append_import_log(app, format!("commit_failed error={err}"));
        format!("MSSQL import failed to commit database transaction: {err}")
    })?;

    append_import_log(
        app,
        format!(
            "finish total_rows={} imported={} updated={} failed={}",
            report.total_rows, report.imported, report.updated, report.failed
        ),
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn build_connection_string_requires_encryption_and_trusted_certificate() {
        let connection_string = build_connection_string("sql.example", 1433, "staff", "secret");

        assert!(connection_string.contains("server=tcp:sql.example,1433;"));
        assert!(connection_string.contains("uid=staff;"));
        assert!(connection_string.contains("pwd=secret;"));
        assert!(connection_string.contains("encrypt=true;"));
        assert!(connection_string.contains("TrustServerCertificate=yes;"));
    }

    #[test]
    fn upsert_mssql_staff_record_persists_nickname_and_azure_ad_account() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        super::super::configure_connection(&conn).expect("configure sqlite pragmas");
        super::super::apply_migrations(&conn).expect("apply database migrations");

        let mut conn = conn;
        let tx = conn.transaction().expect("start transaction");
        let record = MssqlStaffRecord {
            code: "ASWVN001".to_string(),
            name: "Test User".to_string(),
            nick_name: Some("Tester".to_string()),
            work_email: Some("test@example.com".to_string()),
            azure_ad_account: Some("tester@example.com".to_string()),
        };

        assert!(matches!(
            upsert_mssql_staff_record(&tx, &record, STAFF_GROUP_EMPLOYEE_LIST),
            Ok(UpsertAction::Inserted)
        ));
        tx.commit().expect("commit transaction");

        let (nick_name, azure_ad_account): (Option<String>, Option<String>) = conn
            .query_row(
                r#"
                SELECT e.nick_name, v.value
                FROM employees e
                LEFT JOIN employee_dynamic_values v
                  ON v.employee_id = e.id
                 AND v.field_key = ?
                WHERE e.employee_id = ?
                "#,
                params![AZURE_AD_ACCOUNT_FIELD_KEY, "ASWVN001"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read imported employee");

        assert_eq!(nick_name.as_deref(), Some("Tester"));
        assert_eq!(azure_ad_account.as_deref(), Some("tester@example.com"));
    }
}
