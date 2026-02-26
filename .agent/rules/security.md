# Security Rules (StaffKit)

## Secrets

- Never commit secrets from `.env` or local credential files.
- Use `.env.example` (without real keys) when sharing required variables.

## Data Handling

- Treat `ExSource/` as local input-only data.
- Do not commit personal/employee raw data exports.
- Validate imported Excel data before persisting to SQLite.

## Coding Practices

- Validate all user/file inputs at boundaries.
- Use parameterized queries for DB writes/reads.
- Avoid logging sensitive data (emails, IDs, tokens) in plain text.

## Dependency Safety

- Add new dependencies only when required.
- Prefer maintained packages and official libraries.
