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
const sharedImportTypes = readFileSync("src/features/import/sharedImportTypes.ts", "utf8")

assert.match(staffTypes, /export type SharedImportPreviewSummary = \{/)
assert.match(staffTypes, /totalRows: number/)
assert.match(staffTypes, /validRows: number/)
assert.match(staffTypes, /errorRows: number/)
assert.match(staffTypes, /export type SharedImportPreviewRow = \{/)
assert.match(staffTypes, /export type SharedImportReport = \{/)
assert.match(staffTypes, /imported: number/)
assert.match(staffTypes, /skipped: number/)
assert.match(staffTypes, /failed: number/)
assert.match(sharedImportTypes, /from "\.\.\/\.\.\/types\/staff"/)
assert.doesNotMatch(sharedImportTypes, /export type SharedImportPreviewSummary = \{/)
assert.doesNotMatch(sharedImportTypes, /export type SharedImportPreviewRow = \{/)
assert.doesNotMatch(sharedImportTypes, /export type SharedImportReport = \{/)

console.log("shared-import-shell contract tests passed")

const importDrawer = readFileSync("src/features/import/ImportDrawer.tsx", "utf8")
const sharedImportShell = readFileSync("src/features/import/sharedImportShell.tsx", "utf8")

assert.match(sharedImportShell, /export function SharedImportShell/)
assert.match(sharedImportShell, /"Approve Import"/)
assert.match(sharedImportShell, /role="dialog"/)
assert.match(sharedImportShell, /aria-modal="true"/)
assert.match(sharedImportShell, /aria-labelledby=/)
assert.match(sharedImportShell, /tabIndex=\{-1\}/)
assert.match(sharedImportShell, /event\.key === "Escape"/)
assert.match(importDrawer, /<SharedImportShell/)
assert.doesNotMatch(importDrawer, /Review Import Changes/)
assert.doesNotMatch(importDrawer, /selectedImportRowIndices/)
assert.doesNotMatch(importDrawer, /togglePreviewRowSelection/)
