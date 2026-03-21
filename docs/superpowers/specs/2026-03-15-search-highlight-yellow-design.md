# Search Highlight Yellow Design

## Goal
Make search matches in the employee table visually obvious on the dark theme by using a stronger yellow text highlight and a soft yellow background on the matched cell.

## Scope
- Apply only to the employee table search result presentation.
- Keep existing search behavior unchanged.
- Preserve existing edit, move-selection, duplicate-check, and hover states.

## Design

### 1. Match Presentation
- Keep the existing substring-based, diacritic-insensitive search matching.
- Continue rendering matched text with `mark`.
- Upgrade the `mark` styling to a warm, high-contrast yellow with dark text so the exact match is immediately visible.

### 2. Cell-Level Emphasis
- When a table cell contains a search match, apply a soft yellow background to the whole cell.
- Add a subtle inner border or glow so the matched cell remains visible against the current dark table background.
- Do not tint the whole row; only the matched cells should receive the search-emphasis treatment.

### 3. State Priority
- Search emphasis must not override more important states:
  - duplicate warning rows
  - edit mode / active cell input
  - move selection state
  - hover feedback
- Matched-cell styling should layer cleanly with normal table rendering and remain readable.

### 4. Implementation Notes
- Extract the “does this cell match the current search term?” decision into a small helper so the cell can choose both text rendering and cell container styling from the same logic.
- Move colors to shared CSS tokens in `src/index.css` instead of scattering inline color values.

### 5. Verification
- Search a value that matches one cell.
- Search a value that matches multiple cells in the same row.
- Search a value that matches multiple rows.
- Confirm the matched text and matched cell both remain clear on dark theme.
- Run the existing quality gate.
