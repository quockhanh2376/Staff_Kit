import type { EmployeeColumnDefinition } from "../../types/staff"
import type { UiColumnDefinition } from "../../types/app"

export function toUiColumnDefinition(
    column: EmployeeColumnDefinition,
    labelOverride?: string,
): UiColumnDefinition {
    return {
        key: column.key,
        label: labelOverride?.trim() || column.label,
        source: column.source,
        dataType: column.dataType,
    }
}
