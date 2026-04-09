import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const loginPage = readFileSync("src/features/auth/LoginPage.tsx", "utf8")

assert.match(loginPage, /max-w-\[420px\]/)
assert.match(loginPage, /bg-\[#161b22\]/)
assert.match(loginPage, /bg-\[#0d1117\]/)
assert.match(loginPage, /border-slate-800/)
assert.match(loginPage, /Forgot Password/)
assert.match(loginPage, /Back to login/)
assert.match(loginPage, /hover:bg-emerald-500/)
assert.match(loginPage, /hover:bg-slate-800\/50/)

console.log("login-auth-ui tests passed")
