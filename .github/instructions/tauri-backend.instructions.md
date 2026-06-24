---
description: "Use when editing Rust, Tauri commands, database access, import flows, LAN services, or persistence under src-tauri/. Covers command boundaries, SQLite rules, and validation for the Staff Kit desktop backend."
applyTo: "src-tauri/**"
---

# Staff Kit Tauri Backend Guidelines

- Keep `src-tauri/src/lib.rs` focused on Tauri command registration and thin command wrappers; put business logic in `db/`, `lan_assets.rs`, or `lan_server.rs` as appropriate.
- Prefer narrow command and database changes over broad rewrites; maintain the existing typed input and output boundary used by Tauri commands.
- Respect the locked data pipeline from `QUALITY.md`: Excel input must be validated before writes, and UI/report flows should read from the database rather than raw files.
- Keep database work parameterized and avoid logging sensitive values.
- When touching Rust or Tauri code, run a Rust-scoped validation step such as `npm run check:tauri` or `cargo check --manifest-path src-tauri/Cargo.toml` when the environment allows it.