import type { EmployeePayload, EmployeeRecord } from "../../types/staff"

export function readEmployeeCellText(employee: EmployeeRecord, key: string): string {
    if (key === "rowNumber") return ""
    const record = employee as Record<string, unknown>
    const value = key in record ? record[key] : (employee.dynamicFields?.[key] ?? undefined)
    if (value === null || value === undefined) return ""
    return String(value)
}

export function extractComputerNameTokens(value: string): string[] {
    return value
        .split(/[\n,]+/g)
        .map((item) => item.trim().toUpperCase())
        .filter(Boolean)
}

export function collectDuplicateComputerEmployeeIds(
    employees: EmployeeRecord[],
    computerKeys: string[],
): Set<number> {
    const tokenToIds = new Map<string, Set<number>>()

    for (const employee of employees) {
        for (const key of computerKeys) {
            const raw = readEmployeeCellText(employee, key)
            const tokens = extractComputerNameTokens(raw)
            for (const token of tokens) {
                const existing = tokenToIds.get(token) ?? new Set<number>()
                existing.add(employee.id)
                tokenToIds.set(token, existing)
            }
        }
    }

    const duplicateIds = new Set<number>()
    for (const ids of tokenToIds.values()) {
        if (ids.size > 1) {
            ids.forEach((id) => duplicateIds.add(id))
        }
    }

    return duplicateIds
}

export function isEmployeeTableEditableColumn(key: string): boolean {
    if (key === "rowNumber") return false
    if (key === "employeeId") return false
    if (key === "computerName") return false
    return true
}

export function buildEmployeePayloadForSave(
    employee: EmployeeRecord,
    drafts: Record<string, string>,
): EmployeePayload {
    const TOP_LEVEL_KEYS = new Set([
        "employeeId",
        "fullName",
        "nickName",
        "teamName",
        "project",
        "jobTitle",
        "email",
        "cellphone",
        "dateOfBirth",
        "gender",
        "aswStartDate",
        "clientStartDate",
        "contractEndDate",
        "clientYearOfServices",
        "notes",
    ])

    const dynamicDrafts: Record<string, string> = {}
    for (const [key, value] of Object.entries(drafts)) {
        if (key === "computerName") {
            continue
        }
        if (!TOP_LEVEL_KEYS.has(key)) {
            dynamicDrafts[key] = value
        }
    }

    return {
        employeeId: (drafts.employeeId ?? employee.employeeId) || "",
        fullName: (drafts.fullName ?? employee.fullName) || "",
        nickName: drafts.nickName ?? employee.nickName ?? null,
        teamName: drafts.teamName ?? employee.teamName ?? null,
        project: drafts.project ?? employee.project ?? null,
        jobTitle: drafts.jobTitle ?? employee.jobTitle ?? null,
        email: drafts.email ?? employee.email ?? null,
        cellphone: drafts.cellphone ?? employee.cellphone ?? null,
        dateOfBirth: drafts.dateOfBirth ?? employee.dateOfBirth ?? null,
        gender: drafts.gender ?? employee.gender ?? null,
        aswStartDate: drafts.aswStartDate ?? employee.aswStartDate ?? null,
        clientStartDate: drafts.clientStartDate ?? employee.clientStartDate ?? null,
        contractEndDate: drafts.contractEndDate ?? employee.contractEndDate ?? null,
        clientYearOfServices:
            drafts.clientYearOfServices ?? employee.clientYearOfServices ?? null,
        // Keep the persisted raw value instead of echoing the derived display value.
        computerName: employee.storedComputerName ?? null,
        notes: drafts.notes ?? employee.notes ?? null,
        staffGroup: employee.staffGroup,
        dynamicFields:
            Object.keys(dynamicDrafts).length > 0
                ? { ...(employee.dynamicFields ?? {}), ...dynamicDrafts }
                : (employee.dynamicFields ?? null),
    }
}
