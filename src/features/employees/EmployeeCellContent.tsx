import type { ReactNode } from "react"
import type { UiColumnDefinition } from "../../types/app"
import { EmailCopyCell } from "./EmailCopyCell"
import { isCopyableEmailValue } from "./emailCopyUtils"
import { getTeamPillClasses } from "../teams/teamColors"

type EmployeeCellContentProps = {
    column: UiColumnDefinition
    rawValue: unknown
    displayValue: ReactNode
    isEditable: boolean
    isActiveCell: boolean
    editor: ReactNode
}

export function EmployeeCellContent({
    column,
    rawValue,
    displayValue,
    isEditable,
    isActiveCell,
    editor,
}: EmployeeCellContentProps) {
    if (isEditable && isActiveCell) return editor

    if (!isEditable && column.dataType === "email" && isCopyableEmailValue(rawValue)) {
        return <EmailCopyCell email={String(rawValue)}>{displayValue}</EmailCopyCell>
    }

    if (!isEditable && column.key === "teamName") {
        const teamText = typeof rawValue === "string" ? rawValue.trim() : ""
        if (!teamText) {
            return <span className="text-[var(--text-secondary)]">—</span>
        }
        return (
            <span
                className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold ${getTeamPillClasses(teamText)}`}
            >
                {displayValue}
            </span>
        )
    }

    return <span className={isEditable ? "cursor-cell select-none" : ""}>{displayValue}</span>
}

