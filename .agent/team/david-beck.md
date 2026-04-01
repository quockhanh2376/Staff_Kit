# David Beck

Role:
- Senior Backend Engineer

Ownership:
- SQLite schema evolution, data model alignment, and Tauri backend contracts
- Import staging, confirmation transactions, status transitions, and audit-safe mutations
- Compatibility boundary for borrow `2.0.1` while the asset model is rebaselined

Watchouts:
- No official data mutation before confirm or approval
- No schema drift between staging and committed records
- Guard duplicate asset code and serial handling
- Keep backend slices small and decoupled from temporary UI structure

Preferred handoff:
- Migration or schema note
- Command/API contract summary
- Transaction and validation risks
