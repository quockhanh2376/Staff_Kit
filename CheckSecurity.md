# StaffKit — Security Audit Checklist

> Stack: **Tauri v2 + Rust + SQLite (rusqlite) + React/TypeScript**  
> Data: Employee PII (names, emails, DOB, contracts) — offline desktop app

---

## Audit Areas

### 1. SQL Injection
Scan all rusqlite queries in `src-tauri/src/db/*.rs`:
- Are all user inputs passed via `params![]` placeholders?
- Flag any string interpolation inside SQL strings.
- Files to check: `import.rs`, `employee.rs`, `team.rs`, and any other db files.

---

### 2. Tauri IPC Command Exposure
Review `src-tauri/src/lib.rs` or `commands/*.rs`:
- List all `#[tauri::command]` functions exposed to the frontend.
- Are there commands that could be abused (e.g., arbitrary file read/write, shell exec)?
- Is there an allowlist/denylist in `tauri.conf.json` for command exposure?

---

### 3. Input Validation
Check all Tauri command handlers:
- Are string lengths validated? Could a user submit 10 MB of text to a field?
- Are file paths sanitized to prevent path traversal (e.g., `../../etc/passwd`)?
- Are `employee_id`, `team_id` foreign key values validated before DB operations?

---

### 4. Tauri CSP (Content Security Policy)
Check `tauri.conf.json`:
- Is CSP configured? What is the current policy?
- Are inline scripts/styles allowed (XSS risk)?
- Is `unsafe-eval` or `unsafe-inline` present?

---

### 5. Data Protection at Rest
- Is the SQLite database encrypted (SQLCipher or similar)?
- Are there any plaintext passwords/secrets stored in the DB or `localStorage`?
- Check if sensitive fields (email, phone, DOB) are stored in cleartext.

---

### 6. Frontend XSS / Injection
Scan `src/features/**/*.tsx`:
- Any `dangerouslySetInnerHTML` usage?
- Are user-supplied strings ever inserted into HTML without sanitization?
- Check if any `invoke()` calls pass unsanitized user input as command parameters.

---

### 7. File System Access
- Does the app read/write files outside its designated app data directory?
- Review file import/export functionality (Excel import) for path traversal.
- Check Tauri capabilities/permissions in `tauri.conf.json`.

---

### 8. Rust Memory Safety
- Are there any `unsafe {}` blocks? If so, document why and assess risk.
- Check for `unwrap()`/`expect()` calls that could panic in production.
- Are errors properly propagated or silently swallowed?

---

### 9. Secrets & Sensitive Config
- Grep for hardcoded passwords, API keys, or tokens in the codebase.
- Check `.env` files and `tauri.conf.json` for exposed secrets.
- Is anything sensitive logged to console or Tauri logs?

---

### 10. Session & Authentication
- Is there a session timeout implemented? (Edit mode has 2-min timeout — is logout handled securely?)
- Can a user bypass authentication by directly accessing views?

---

## Severity Scale

| Level | Description |
|---|---|
| 🔴 CRITICAL | Immediate data breach / data loss risk |
| 🟠 HIGH | Exploitable under normal usage |
| 🟡 MEDIUM | Risk under specific conditions |
| 🟢 LOW | Minor hardening improvement |

## Report Format (per finding)

```
- File + line reference
- Severity: CRITICAL / HIGH / MEDIUM / LOW
- Description of the risk
- Recommended fix with code example
```
