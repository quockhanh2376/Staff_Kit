import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

import type { BorrowLanSettings } from "../src/types/staff.ts"
import {
  buildBorrowLanUrlPreview,
  chooseBorrowLanHostInput,
} from "../src/features/settings/borrowLanAutoFill.ts"
import * as borrowLanAutoFillModule from "../src/features/settings/borrowLanAutoFill.ts"

const settingsState = readFileSync("src/features/settings/useSettingsState.ts", "utf8")
const borrowLanQrCard = readFileSync("src/features/borrow/BorrowLanQrCard.tsx", "utf8")

const applyDetectedBorrowLanSettings = (
  borrowLanAutoFillModule as {
    applyDetectedBorrowLanSettings?: (settings: BorrowLanSettings | null, detectedHost: string | null) => BorrowLanSettings | null
  }
).applyDetectedBorrowLanSettings

assert.equal(chooseBorrowLanHostInput("saved.example.com", "203.0.113.45"), "203.0.113.45")
assert.equal(chooseBorrowLanHostInput("saved.example.com", null), "saved.example.com")
assert.equal(chooseBorrowLanHostInput(" saved.example.com ", "   "), "saved.example.com")
assert.equal(buildBorrowLanUrlPreview("203.0.113.45", "8787"), "http://203.0.113.45:8787/borrow")
assert.equal(buildBorrowLanUrlPreview("2001:db8::10", "8787"), "http://[2001:db8::10]:8787/borrow")
assert.equal(buildBorrowLanUrlPreview("", "8787"), "Borrow URL will appear here after host is detected or entered.")
assert.equal(buildBorrowLanUrlPreview("203.0.113.45", "0"), "Enter a valid port to preview the Borrow URL.")

assert.equal(typeof applyDetectedBorrowLanSettings, "function")

const savedSettings: BorrowLanSettings = {
  host: "OFFICE-PC",
  port: 8787,
  borrowUrl: "http://OFFICE-PC:8787/borrow",
}

assert.deepEqual(applyDetectedBorrowLanSettings?.(savedSettings, "192.168.2.25"), {
  host: "192.168.2.25",
  port: 8787,
  borrowUrl: "http://192.168.2.25:8787/borrow",
})
assert.deepEqual(applyDetectedBorrowLanSettings?.(savedSettings, null), savedSettings)

assert.match(settingsState, /handleRefreshBorrowLanHost/)
assert.match(settingsState, /detectBorrowLanHost/)
assert.match(settingsState, /setBorrowLanSettings\(\(current\) => applyDetectedBorrowLanSettings\(current, nextHost\)\)/)
assert.match(borrowLanQrCard, /RefreshCw/)
assert.match(borrowLanQrCard, /handleRefreshBorrowLanHost/)

console.log("borrow-lan-autofill tests passed")
