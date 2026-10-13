# EE List MSSQL Preview Sync Design

## Goal

Change the EE List **Update** action into an explicitly approved sync from the configured MSSQL staff source into the local `employee_list` records. The source is queried only; the operation never sends a write statement to MSSQL.

## Current behavior and root cause

`useEmployeeState.updateEmployeeListFromMssql` tests the configured MSSQL connection and then calls `import_mssql_staff`. That backend command runs the default query and immediately upserts the returned rows to SQLite. The button has no comparison preview or approval boundary.

The current default query joins `Staffs` to `AspNetUsers` with `INNER JOIN` and limits output to `TOP (1000)`. It therefore omits staff without an ASP.NET user by design and any eligible joined rows beyond the first 1000. Its `ROW_NUMBER` ordering by `Code` does not resolve duplicates deterministically. The existing upsert searches employee IDs globally, so an EE List update can alter a matching employee in another staff group.

No MSSQL connection settings were present in the Process, User, or Machine environment during this audit. Consequently, live Query A/Query B counts and source connectivity have not been verified.

## Approved query direction

Use the user-approved fixed Query B owned by the EE List sync path:

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
    FROM [AssetManagement].[dbo].[Staffs] AS S
    INNER JOIN [AssetManagement].[dbo].[AspNetUsers] AS ASPU
        ON S.[Id] = ASPU.[StaffId]
    WHERE S.[Resigned] = 0
)
SELECT [Code], [Name], [NickName], [WorkEmail], [AzureAD]
FROM StaffList
WHERE RowNum = 1
ORDER BY [Code]
```

Query B intentionally uses `INNER JOIN`: staff without `AspNetUsers` are not part of the sync result. It has no row cap. It deduplicates by normalized Staff Code with deterministic ordering. It returns `Code`, `Name`, `NickName`, `COALESCE(WorkEmail, WorkEmail2) AS WorkEmail`, and `AspNetUsers.UserName AS AzureAD`. The sync command accepts no arbitrary SQL text. The legacy custom MSSQL import panel remains a separate workflow and is outside this change. The user explicitly confirmed this Query B, including the inner-join exclusion rule, on 2026-10-13.

The query applies the mapping `Code -> employee_id`, `Name -> full_name`, `NickName -> nick_name`, `WorkEmail -> email`, and `AzureAD -> azure_account` only when that column is included by the approved query. Source eligibility is `Resigned = 0`. Staff Code is trimmed and case-normalized for matching. Duplicate source Codes are collapsed deterministically and reported in the preview; conflicting data for the same normalized Code blocks approval instead of being silently discarded.

## Flow

1. Clicking Update requests a preview. The backend connects with configured credentials and executes only the fixed Query B. It performs no MSSQL mutations.
2. The backend reads current local employee-list records and AzureAD values through a SQLite read-only connection (`SQLITE_OPEN_READ_ONLY` plus `PRAGMA query_only=ON`); preview must not call migration or WAL configuration routines.
3. Compare by normalized Staff Code and classify `New`, `Changed`, `Unchanged`, and `Missing from source`. Missing is report-only. Rows in other staff groups are not changed; if a source Code collides with a non-EE-List row, show a conflict and block applying that conflicting row rather than moving or rewriting it.
4. Display category counts and reviewable row details before presenting an explicit approval action. Preview connection/query errors are visible and leave local DB contents unchanged.
5. Approval references the exact preview snapshot. The backend confirms that the current source result still matches that snapshot; if it changed, reject the approval and require a fresh preview. After confirmation, one SQLite transaction inserts New rows and updates only mapped fields of Changed `employee_list` rows. It leaves Unchanged, Missing, and other-group rows untouched.
6. On success, refresh the EE List and show completion time plus New/Changed/Unchanged/Missing counts. On failure, show a safe actionable error and preserve the prior local data through transaction rollback. Result/error feedback is session-visible; no extra status write occurs before approval.

## Component ownership

- `src-tauri/src/db/mssql_import.rs`: fixed sync query, source record normalization/deduplication, read-only local snapshot, category comparison, preview snapshot lifecycle, approved transactional apply. Keep the generic MSSQL import behavior separate.
- `src-tauri/src/lib.rs`: admin-authorized preview/apply command wrappers and preview-state registration.
- `src/services/staff-api.ts` and `src/types/staff.ts`: typed preview/apply IPC DTOs.
- `src/features/employees/useEmployeeState.ts` and `EmployeeView.tsx`: Update preview/approval state, category/result/error display, successful list refresh.
- Focused Rust and frontend tests cover mapping, classification, local read-only preview, approval gating, local-only apply, duplicate/conflict behavior, connection failure, status feedback, and refresh after source changes.

## Safety boundaries

- No remote insert/update/delete or arbitrary query is exposed by the EE List Update sync commands. The configured source login must be read-only; the sync itself sends only Query B, a SELECT statement.
- No local transaction is opened by the preview path; local DB writes happen only in the explicit approval command.
- Local apply is restricted to `employee_list` and mapped source fields. No Employee master fields outside the approved mapping, loan/asset, auth, or other staff-group behavior is changed.
- No live source DB writes, no live local DB writes during development, and no merge to main before Qeen approves the exact feature commit.

## Acceptance evidence

Tests prove new/change/unchanged/missing classification by Staff Code; approved mapping and Code dedupe; preview does not change local rows; approval applies only New/Changed EE List rows; failed connection or changed source snapshot causes zero local writes; and a repeated preview after changed source data returns the new values. A direct SQL A/B count comparison requires configured read-only source credentials and must be recorded separately from fixture tests.
