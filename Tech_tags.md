# Tech Stack & Tools cho Employee Manager App (Tauri v2)

Date: 2026-02-26

## 1) Core Framework

- `tauri-v2` (desktop + mobile targets)
- `rust` (backend commands, DB, import/export)
- `typescript`
- `react`
- `vite`

## 2) Database Layer

- `sqlite`
- `fts5` for fast full-text search (`full_name`, `email`, `notes`, `team`)
- Driver options:
  - `rusqlite` (simple sync local-first)
  - `sqlx` (async + macros + typed query model)

## 3) Excel Import / Data Processing

- `calamine` (native `.xlsx` reader in Rust)
- `polars` (optional for large datasets and transforms)
- `csv` (export and optional intermediate normalization)
- `exsource-master-input` (dev convention: `ExSource/*.xlsx`)

## 4) UI / Frontend Libraries

- `shadcn-ui` style component system
- `tanstack-table` for table/filter/sort/pagination
- `lucide-react` for iconography
- `tailwindcss-v4` for tokenized theme/styling

## 5) Tauri Plugins

- `tauri-plugin-fs` (file read/write)
- `tauri-plugin-dialog` (file picker)

## 6) Build & Dev Tools

- `cargo-tauri`
- `trunk` (only if WASM path is needed)
- `xcode` (iOS builds)
- `android-studio` (Android builds)

## 7) Cargo.toml - Recommended (rusqlite)

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

# DB
rusqlite = { version = "0.32", features = ["bundled"] }

# Import / Export
calamine = "0.27"
polars = { version = "0.46", optional = true, default-features = false, features = ["lazy", "fmt", "strings", "dtype-date", "dtype-datetime"] }
csv = "1"

# Tauri plugins
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"

# Error + datetime
anyhow = "1"
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
```

## 8) Cargo.toml - Alternative (sqlx)

```toml
[dependencies]
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls", "macros", "chrono"] }
```

## 9) Frontend package tags (npm)

- `react`
- `react-dom`
- `typescript`
- `vite`
- `tailwindcss`
- `@tailwindcss/vite`
- `lucide-react`
- `@tanstack/react-table` (if table logic is split)
- `class-variance-authority`
- `clsx`
- `tailwind-merge`

## 10) Technical Tags (normalized)

- `tauri-v2`
- `rust-backend`
- `react-typescript`
- `sqlite-fts5`
- `excel-import-calamine`
- `optional-polars`
- `csv-export`
- `shadcn-ui`
- `tanstack-table`
- `tailwindcss-v4`
- `tauri-plugin-fs`
- `tauri-plugin-dialog`
- `cargo-tauri`
- `offline-first`
- `onedrive-sync-optional`
- `exsource-master-input`
- `db-first-ui-read`

## 11) Module Mapping

- `db/*` -> sqlite schema, migrations, FTS query
- `import/*` -> excel parse, row validation, upsert service
- `export/*` -> csv generation
- `commands/*` -> tauri command handlers
- `ui/*` -> table/filter/drawer/theme toggle
- `pipeline/*` -> ExSource input -> import pipeline -> DB -> UI/report
