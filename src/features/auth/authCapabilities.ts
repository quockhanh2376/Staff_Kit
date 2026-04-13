import type { LocalAccountRecord } from "../../types/staff"

export type AuthCapabilities = {
    isAdminAccount: boolean
    isSuperAdminAccount: boolean
    canAccessSettings: boolean
    canImportData: boolean
    canResetData: boolean
    canEditEmployeeTable: boolean
    canEditEmployeeComputerName: boolean
}

export function deriveAuthCapabilities({
    activeAccount,
    isAuthenticated,
}: {
    activeAccount: LocalAccountRecord | null
    isAuthenticated: boolean
}): AuthCapabilities {
    const isAdminAccount = activeAccount?.role === "admin" || activeAccount?.role === "super_admin"
    const isSuperAdminAccount =
        activeAccount?.isSuperAdmin === true || activeAccount?.role === "super_admin"

    return {
        isAdminAccount,
        isSuperAdminAccount,
        canAccessSettings: isAdminAccount,
        canImportData: isAdminAccount,
        canResetData: isSuperAdminAccount,
        // Employee table edit should stay available for a valid signed-in session even if
        // local account metadata is still hydrating or temporarily unavailable.
        canEditEmployeeTable: isAuthenticated,
        canEditEmployeeComputerName: isAuthenticated && isSuperAdminAccount,
    }
}
