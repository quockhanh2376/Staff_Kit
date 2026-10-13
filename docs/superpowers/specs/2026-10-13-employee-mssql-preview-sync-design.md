# EE List Remote MSSQL Sync Design

## Goal

Update EE List synchronization to use the approved fixed Query B, a credential-safe Remote EE Sync settings screen, a verified read-only SQL Server login, a local comparison preview, and explicit local approval before writes.

## Approved source query

Query B is the canonical sync query. It selects Staffs joined to AspNetUsers with `INNER JOIN`, filters `Resigned = 0`, deduplicates by normalized `Code`, has no TOP limit, and maps `Code`, `Name`, `NickName`, `COALESCE(WorkEmail, WorkEmail2) AS WorkEmail`, and `AspNetUsers.UserName AS AzureAD`. Staff rows lacking AspNetUsers are intentionally excluded. The application owns this SQL; the user cannot enter SQL.

The configured database is the SQL connection's current database; the fixed query uses schema-qualified objects so the selected database is honored:

```sql
WITH StaffList AS
(
    SELECT
        S.[Code],
        S.[Name],
        S.[NickName],
        COALESCE(S.[WorkEmail], S.[WorkEmail2]) AS [WorkEmail],
        ASPU.[UserName] AS [AzureAD],
        ROW_NUMBER() OVER (
            PARTITION BY UPPER(LTRIM(RTRIM(S.[Code])))
            ORDER BY S.[Id], ASPU.[UserName]
        ) AS RowNum
    FROM [dbo].[Staffs] AS S
    INNER JOIN [dbo].[AspNetUsers] AS ASPU ON S.[Id] = ASPU.[StaffId]
    WHERE S.[Resigned] = 0
)
SELECT [Code], [Name], [NickName], [WorkEmail], [AzureAD]
FROM StaffList
WHERE RowNum = 1
ORDER BY [Code]
```

## Settings and credentials

Add Settings → Remote EE Sync with Server, Database, Authentication type, the currently approved SQL login, Password, and Test Connection. The initial approved login is `user_reader`; only a super_admin may change it, through an explicit exact-match confirmation. The approved login is stored in SQLCipher `app_settings`, and the backend rejects any saved or environment username that does not exactly match it before loading a password or opening a network connection. Windows Integrated authentication is rejected because it cannot prove the approved SQL login. The UI receives only non-secret profile fields and a `hasSavedPassword` boolean. Password entry is sent directly to a backend command and never returned to the UI.

Persist server, database, port, authentication type, approved username, authorization actor/time, and a fixed secret reference in SQLCipher `app_settings`. Store SQL passwords in the current Windows user's Credential Manager (protected by Windows credential encryption/DPAPI); do not store password material in SQLite, source, exports, logs, or profile files. On non-Windows platforms, SQL password persistence is unavailable and must fail closed with a clear reason. Empty password on save preserves an existing password; an explicit remove action deletes it. Changing the approved login deletes only Staff Kit's fixed Credential Manager target, clears pending previews, and requires a new password. If the legacy Staff Kit profile names the prohibited HRMS login, migration clears that profile value and deletes only the Staff Kit target without reading the secret. Environment variables `STAFFKIT_MSSQL_SERVER`, `STAFFKIT_MSSQL_DATABASE`, `STAFFKIT_MSSQL_PORT`, `STAFFKIT_MSSQL_AUTH_TYPE`, `STAFFKIT_MSSQL_USERNAME`, and `STAFFKIT_MSSQL_PASSWORD` provide a temporary full-profile override for automation. If any is present, all fields required by the selected auth type must be present; never mix partial environment configuration with saved settings. Environment values are never persisted or returned. Do not log their values.

## Read-only connection gate

Before any SQL network/authentication attempt, the Rust backend requires SQL Server authentication and an exact username match for the encrypted, currently approved login. Only super_admin can authorize a change; regular admins cannot. This gate applies to saved profiles, environment overrides, Test Connection, Preview, and Apply. The prohibited `aswhite-plus-webserver`, unknown users, case/whitespace variants, invalid approval values, and Windows Integrated authentication are rejected before TCP/TLS setup or password retrieval. Then connect with required encryption, system-trusted server certificate validation, and the configured database; certificate validation failures block connection. Run a non-destructive permission introspection query that checks effective server, database, schema, object, role, and principal permissions including inherited permissions. Reject when any effective write/escalation permission exists, required permission evidence is NULL/missing, or inspection fails. The connection's ApplicationIntent/read-only option is not treated as a substitute for permission verification. The only data query allowed is the fixed Query B SELECT; Test Connection performs the same permission gate and Query B fetch, returning PASS/FAIL, a safe reason, and row count without employee rows. Permission introspection is limited to read-only metadata SELECTs. No remote INSERT/UPDATE/DELETE/DDL is issued.

### SQL TLS certificate acceptance

Confirmed server-side cause (2026-10-13): SQL Server 2017 ERRORLOG reports, “A self-generated certificate was successfully loaded for encryption.” The server must be configured with a DBA-provided certificate trusted by the client; the application must continue to require encryption and normal certificate/hostname validation. Do not enable `TrustServerCertificate`, accept invalid certificates, or fall back to plaintext.

After the DBA installs and binds the certificate to SQL Server, accept the change only when all of these checks pass:

1. The saved Server value remains exactly as entered and resolves to the intended SQL Server. DNS names must match a DNS SAN; an IP endpoint must match an IP SAN. Do not silently convert between hostname and IP.
2. The presented certificate is within its validity period, chains to a CA trusted by the Windows client, and the server presents the required intermediate chain. Confirm the SQL Server service is using the intended certificate and TLS remains enabled.
3. In Staff Kit, Test Connection succeeds with certificate validation enabled and reports no certificate/hostname warning. Then independently confirm the effective read-only permission gate passes.
4. Only after both gates pass, fetch fixed Query B and compare total rows, unique Staff Code values, and Code/Name/NickName/WorkEmail/AzureAD mappings with a direct read-only DBA result. Open the preview and verify New/Changed/Unchanged/Missing counts; do not click Apply during acceptance.
5. Confirm no remote writes and no local business DB writes occurred. If any certificate, connection, permission, query, or mapping check fails, stop and report the sanitized reason; do not bypass TLS validation.

## Sync lifecycle

Connect → permission gate → Query B → read local EE List through encrypted SQLite read-only connection → compare by normalized Staff Code → show New/Changed/Unchanged/Missing preview → wait for explicit approval → re-fetch Query B and recheck source permissions → apply only New/Changed EE List rows in one local transaction. Missing is report-only. Other employee groups, Employee master behavior, loans, assets, and auth remain untouched. Test Connection, preview, and all pre-approval operations perform no local business writes. Apply is the only local write boundary.

## Legacy SQL paths

Retire the generic MSSQL importer UI and its Tauri commands because arbitrary SQL and arbitrary staff-group imports violate the fixed Query B boundary. Preserve the unrelated Excel import flow. All remaining MSSQL connections use the single Remote EE Sync profile and permission gate.

## Error and status behavior

Test Connection always resolves to a result with PASS/FAIL, safe reason, and optional row count; secrets, connection strings, employee rows, and raw driver errors are not logged or returned. EE List displays preview, completion time/counts, or a safe actionable failure. No password is included in exported profiles; no MSSQL credentials are part of import/export files.

## Verification and safety

Tests cover configuration precedence and partial env rejection, password-free DTOs, secret storage abstraction, fixed Query B-only APIs, role-inherited permission denial, Test Connection PASS/FAIL results, preview-before-approval, local-write boundary, and existing New/Changed/Unchanged/Missing behavior. Qeen independently validates the exact commit and permission enforcement. Direct Query B count and field mapping verification is only performed using a separately configured read-only login; no live source or live Staff Kit DB writes are allowed.
