# Staff_Kit Desktop Separation Report

Date: 2026-03-15

## Goal

Keep `Staff_Kit` as a native desktop application and remove mixed web-project context related to AssetDesk-Pro.

## Source Of Truth For Staff_Kit Desktop

- `README.md`
- `Note.md`
- `QUALITY.md`
- `package.json`
- `src/`
- `src-tauri/`

## Files And Folders Identified For Separation

These items belong to the separate web-track and should not stay in the desktop-native Staff_Kit workspace:

- `web/`
- `openspec/`
- `ConvertWEB.md`
- `docs/business-notes.md`

## Context Files Cleaned Instead Of Removed

These files are still useful for local agent/project guidance, but were rewritten so they match the desktop-native direction:

- `.agent/project-context.md`
- `.agent/workflows/implement-feature.md`

## Notes

- No direct runtime dependency from the desktop app to `E:\AssetDesk-Pro` was found.
- The desktop app continues to depend on Tauri IPC through `src/services/staff-api.ts` and remains native-desktop only.
- The removed items were planning/spec/bootstrap materials for the separate web application track.
