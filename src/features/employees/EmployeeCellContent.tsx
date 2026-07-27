import type { ReactNode } from "react"
import type { UiColumnDefinition } from "../../types/app"
import { EmailCopyCell } from "./EmailCopyCell"
import { isCopyableEmailValue } from "./emailCopyUtils"

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

    return <span className={isEditable ? "cursor-cell select-none" : ""}>{displayValue}</span>
}
