import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

import {
    SHARED_IMPORT_PREVIEW_SUMMARY_KEYS,
    SHARED_IMPORT_REPORT_COUNT_KEYS,
    SHARED_IMPORT_PREVIEW_ROW_KEYS,
} from "../src/features/import/sharedImportTypes.ts"

assert.deepEqual(SHARED_IMPORT_PREVIEW_SUMMARY_KEYS, ["totalRows", "validRows", "errorRows"])
assert.deepEqual(SHARED_IMPORT_REPORT_COUNT_KEYS, ["imported", "skipped", "failed"])
assert.deepEqual(SHARED_IMPORT_PREVIEW_ROW_KEYS, ["id", "title", "subtitle", "badge", "cells"])

const staffTypes = readFileSync("src/types/staff.ts", "utf8")

assert.match(staffTypes, /export type SharedImportPreviewSummary = \{/)
assert.match(staffTypes, /totalRows: number/)
assert.match(staffTypes, /validRows: number/)
assert.match(staffTypes, /errorRows: number/)
assert.match(staffTypes, /export type SharedImportPreviewRow = \{/)
assert.match(staffTypes, /export type SharedImportReport = \{/)
assert.match(staffTypes, /imported: number/)
assert.match(staffTypes, /skipped: number/)
assert.match(staffTypes, /failed: number/)

console.log("shared-import-shell contract tests passed")
