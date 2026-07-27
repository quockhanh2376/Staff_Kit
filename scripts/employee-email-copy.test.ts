import assert from "node:assert/strict"

import {
    copyEmailValue,
    isCopyableEmailValue,
    type ClipboardWriter,
} from "../src/features/employees/emailCopyUtils.ts"

assert.equal(isCopyableEmailValue("person@example.com"), true)
assert.equal(isCopyableEmailValue("  person@example.com  "), true)
assert.equal(isCopyableEmailValue(null), false)
assert.equal(isCopyableEmailValue(undefined), false)
assert.equal(isCopyableEmailValue(""), false)
assert.equal(isCopyableEmailValue("   "), false)
assert.equal(isCopyableEmailValue("-"), false)

const copied: string[] = []
const writer: ClipboardWriter = {
    writeText: async (value) => {
        copied.push(value)
    },
}

assert.deepEqual(
    await copyEmailValue("  Person@Example.COM  ", writer),
    { status: "copied", value: "Person@Example.COM" },
)
assert.deepEqual(copied, ["Person@Example.COM"])

const failed = await copyEmailValue(
    "person@example.com",
    {
        writeText: async () => {
            throw new Error("clipboard unavailable")
        },
    },
)
assert.deepEqual(failed, { status: "failed" })

let inertCalls = 0
const inert = await copyEmailValue(" - ", {
    writeText: async () => {
        inertCalls += 1
    },
})
assert.deepEqual(inert, { status: "inert" })
assert.equal(inertCalls, 0)

console.log("employee-email-copy tests passed")
