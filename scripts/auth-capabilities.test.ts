import assert from "node:assert/strict"

import { deriveAuthCapabilities } from "../src/features/auth/authCapabilities.ts"
import type { LocalAccountRecord } from "../src/types/staff.ts"

const userAccount: LocalAccountRecord = {
  id: 1,
  accountKey: "user-account",
  displayName: "User",
  username: "user",
  role: "user",
  isSuperAdmin: false,
  isActive: true,
  forcePasswordReset: false,
  createdAt: "2026-04-13 00:00:00",
  updatedAt: "2026-04-13 00:00:00",
}

const adminAccount: LocalAccountRecord = {
  ...userAccount,
  id: 2,
  accountKey: "admin-account",
  displayName: "Admin",
  username: "admin",
  role: "admin",
}

const superAdminAccount: LocalAccountRecord = {
  ...adminAccount,
  id: 3,
  accountKey: "super-admin-account",
  displayName: "Super Admin",
  username: "superadmin",
  role: "super_admin",
  isSuperAdmin: true,
}

assert.deepEqual(
  deriveAuthCapabilities({
    activeAccount: null,
    isAuthenticated: false,
  }),
  {
    isAdminAccount: false,
    isSuperAdminAccount: false,
    canAccessSettings: false,
    canImportData: false,
    canResetData: false,
    canEditEmployeeTable: false,
  },
)

assert.equal(
  deriveAuthCapabilities({
    activeAccount: null,
    isAuthenticated: true,
  }).canEditEmployeeTable,
  true,
)

assert.deepEqual(
  deriveAuthCapabilities({
    activeAccount: adminAccount,
    isAuthenticated: true,
  }),
  {
    isAdminAccount: true,
    isSuperAdminAccount: false,
    canAccessSettings: true,
    canImportData: true,
    canResetData: false,
    canEditEmployeeTable: true,
  },
)

assert.deepEqual(
  deriveAuthCapabilities({
    activeAccount: superAdminAccount,
    isAuthenticated: true,
  }),
  {
    isAdminAccount: true,
    isSuperAdminAccount: true,
    canAccessSettings: true,
    canImportData: true,
    canResetData: true,
    canEditEmployeeTable: true,
  },
)

assert.deepEqual(
  deriveAuthCapabilities({
    activeAccount: userAccount,
    isAuthenticated: true,
  }),
  {
    isAdminAccount: false,
    isSuperAdminAccount: false,
    canAccessSettings: false,
    canImportData: false,
    canResetData: false,
    canEditEmployeeTable: true,
  },
)

console.log("auth-capabilities tests passed")