import type {
    AssetImportFieldMapping,
    AssetImportMode,
    AssetImportRawValue,
    AssetImportRowRecord,
} from "../../types/staff"

export type AssetImportWizardFieldKey =
    | "category"
    | "assetName"
    | "itemName"
    | "brand"
    | "model"
    | "serialNumber"
    | "quantity"
    | "warehouse"
    | "note"

export type AssetImportOwnerFieldKey =
    | "submittedStaffId"
    | "submittedFullName"
    | "submittedTeam"
    | "submittedPhoneNumber"

export type AssetImportWizardMapping = Partial<
    Record<AssetImportWizardFieldKey, string | null>
>

const SERIALIZED_MAPPING_KEYS = [
    "category",
    "assetName",
    "brand",
    "model",
    "serialNumber",
    "warehouse",
    "note",
] as const satisfies readonly AssetImportWizardFieldKey[]

const QUANTITY_MAPPING_KEYS = [
    "itemName",
    "category",
    "brand",
    "model",
    "quantity",
    "warehouse",
    "note",
] as const satisfies readonly AssetImportWizardFieldKey[]

const SERIALIZED_REQUIRED_KEYS = [
    "category",
    "assetName",
] as const satisfies readonly AssetImportWizardFieldKey[]

const QUANTITY_REQUIRED_KEYS = [
    "itemName",
    "category",
    "quantity",
] as const satisfies readonly AssetImportWizardFieldKey[]

const EDITABLE_FIELD_KEYS = new Set<AssetImportWizardFieldKey>([
    "category",
    "assetName",
    "itemName",
    "brand",
    "model",
    "serialNumber",
    "quantity",
    "warehouse",
    "note",
])

const FIELD_LABELS: Record<AssetImportWizardFieldKey, string> = {
    category: "Category",
    assetName: "Asset Name",
    itemName: "Item Name",
    brand: "Brand",
    model: "Model",
    serialNumber: "Serial Number",
    quantity: "Quantity",
    warehouse: "Warehouse",
    note: "Note",
}

const OWNER_FIELD_LABELS: Record<AssetImportOwnerFieldKey, string> = {
    submittedStaffId: "Staff ID",
    submittedFullName: "Submitted Name",
    submittedTeam: "Submitted Team",
    submittedPhoneNumber: "Phone Number",
}

const OWNER_FIELD_ALIASES: Record<AssetImportOwnerFieldKey, string[]> = {
    submittedStaffId: ["staffid", "eeid", "employeeid"],
    submittedFullName: ["tennhanvien", "vietnamesename", "fullname", "hoten"],
    submittedTeam: ["team", "client", "clientpmd"],
    submittedPhoneNumber: ["phonenumber", "phone", "cellphone", "mobilenumber"],
}

const MODE_LABELS: Record<AssetImportMode, string> = {
    serialized: "Serialized Assets",
    quantity: "Quantity Stock",
}

const FIELD_ALIASES: Record<AssetImportWizardFieldKey, string[]> = {
    category: ["category", "asset category", "asset_type", "asset type", "type", "loai tai san", "loaitaisan"],
    assetName: ["asset_name", "asset name", "display name", "display_name", "name", "assetname", "ten tai san", "tentaisan"],
    itemName: ["item_name", "item name", "name", "display name", "display_name", "asset name", "asset_name"],
    brand: ["brand", "maker", "vendor", "nhan hieu", "nhanhieu"],
    model: ["model", "model number", "model_number", "model no", "model_no"],
    serialNumber: ["serial_number", "serial number", "serialnumber", "serial", "serial no", "serial_no", "sn"],
    quantity: ["quantity", "qty", "so luong", "soluong"],
    warehouse: ["warehouse", "location", "stock location", "kho"],
    note: ["note", "notes", "remark", "remarks", "ghi chu", "ghichu"],
}

export function getAssetImportModeLabel(mode: AssetImportMode): string {
    return MODE_LABELS[mode]
}

export function getAssetImportMappingKeys(
    mode: AssetImportMode,
): readonly AssetImportWizardFieldKey[] {
    return mode === "quantity" ? QUANTITY_MAPPING_KEYS : SERIALIZED_MAPPING_KEYS
}

export function getRequiredAssetImportMappingKeys(
    mode: AssetImportMode,
): readonly AssetImportWizardFieldKey[] {
    return mode === "quantity" ? QUANTITY_REQUIRED_KEYS : SERIALIZED_REQUIRED_KEYS
}

export function getAssetImportReviewFieldKeys(
    mode: AssetImportMode,
): readonly AssetImportWizardFieldKey[] {
    return getAssetImportMappingKeys(mode)
}

export function getAssetImportFieldLabel(
    fieldKey: AssetImportWizardFieldKey,
): string {
    return FIELD_LABELS[fieldKey]
}

export function getAssetImportOwnerFieldLabel(
    fieldKey: AssetImportOwnerFieldKey,
): string {
    return OWNER_FIELD_LABELS[fieldKey]
}

export function isAssetImportFieldEditable(
    fieldKey: AssetImportWizardFieldKey,
): boolean {
    return EDITABLE_FIELD_KEYS.has(fieldKey)
}

export function detectAssetImportWizardMapping(
    headers: string[],
    mode: AssetImportMode,
): AssetImportWizardMapping {
    const mapping: AssetImportWizardMapping = {}

    for (const fieldKey of getAssetImportMappingKeys(mode)) {
        const matched = headers.find((header) => {
            const normalizedHeader = normalizeHeader(header)
            return FIELD_ALIASES[fieldKey].some(
                (alias) => normalizeHeader(alias) === normalizedHeader,
            )
        })
        if (matched) {
            mapping[fieldKey] = matched
        }
    }

    return mapping
}

export function mergeAssetImportWizardMappings(
    base: AssetImportWizardMapping,
    override: AssetImportWizardMapping,
): AssetImportWizardMapping {
    return {
        ...base,
        ...Object.fromEntries(
            Object.entries(override).filter(([, value]) => Boolean(value)),
        ),
    }
}

export function hasRequiredAssetImportMapping(
    mapping: AssetImportWizardMapping,
    mode: AssetImportMode,
): boolean {
    return getRequiredAssetImportMappingKeys(mode).every((fieldKey) =>
        Boolean(mapping[fieldKey]),
    )
}

export function convertBackendMappingToWizardMapping(
    mode: AssetImportMode,
    mapping: AssetImportFieldMapping,
): AssetImportWizardMapping {
    if (mode === "quantity") {
        return {
            itemName: mapping.displayName ?? null,
            category: mapping.assetType ?? null,
            brand: mapping.brand ?? null,
            model: mapping.model ?? null,
            quantity: mapping.quantity ?? null,
            warehouse: mapping.warehouse ?? null,
            note: mapping.notes ?? null,
        }
    }

    return {
        category: mapping.assetType ?? null,
        assetName: mapping.displayName ?? null,
        brand: mapping.brand ?? null,
        model: mapping.model ?? null,
        serialNumber: mapping.serialNumber ?? null,
        warehouse: mapping.warehouse ?? null,
        note: mapping.notes ?? null,
    }
}

export function toBackendAssetImportMapping(
    mode: AssetImportMode,
    mapping: AssetImportWizardMapping,
    headers: string[],
): AssetImportFieldMapping | null {
    void headers

    if (mode === "quantity") {
        return {
            assetType: mapping.category ?? null,
            displayName: mapping.itemName ?? null,
            brand: mapping.brand ?? null,
            model: mapping.model ?? null,
            quantity: mapping.quantity ?? null,
            warehouse: mapping.warehouse ?? null,
            notes: mapping.note ?? null,
        }
    }

    return {
        assetType: mapping.category ?? null,
        displayName: mapping.assetName ?? null,
        brand: mapping.brand ?? null,
        model: mapping.model ?? null,
        serialNumber: mapping.serialNumber ?? null,
        warehouse: mapping.warehouse ?? null,
        notes: mapping.note ?? null,
    }
}

export function canStageAssetImportMode(
    mode: AssetImportMode,
    headers: string[],
    mapping: AssetImportWizardMapping,
): boolean {
    void headers
    return hasRequiredAssetImportMapping(mapping, mode)
}

export function getAssetImportStageBlockReason(
    mode: AssetImportMode,
    headers: string[],
    mapping: AssetImportWizardMapping,
): string | null {
    void headers
    if (!hasRequiredAssetImportMapping(mapping, mode)) {
        return "Map all required columns before staging a batch."
    }

    return null
}

export function getAssetImportRowFieldValue(
    row: AssetImportRowRecord,
    fieldKey: AssetImportWizardFieldKey,
    mapping: AssetImportWizardMapping,
): string {
    switch (fieldKey) {
        case "category":
            return row.assetType ?? findRawValue(row.rawValues, mapping.category)
        case "assetName":
        case "itemName":
            return row.displayName ?? findRawValue(row.rawValues, mapping[fieldKey])
        case "brand":
            return row.brand ?? findRawValue(row.rawValues, mapping.brand)
        case "model":
            return row.model ?? ""
        case "serialNumber":
            return row.serialNumber ?? ""
        case "quantity":
            return row.quantity ?? findRawValue(row.rawValues, mapping.quantity)
        case "warehouse":
            return row.warehouse ?? findRawValue(row.rawValues, mapping.warehouse)
        case "note":
            return row.notes ?? findRawValue(row.rawValues, mapping.note)
        default:
            return ""
    }
}

export function getAssetImportOwnerFieldValue(
    row: AssetImportRowRecord,
    fieldKey: AssetImportOwnerFieldKey,
): string {
    switch (fieldKey) {
        case "submittedStaffId":
            return row.submittedStaffId ?? findRawValueByAliases(row.rawValues, OWNER_FIELD_ALIASES[fieldKey])
        case "submittedFullName":
            return row.submittedFullName ?? findRawValueByAliases(row.rawValues, OWNER_FIELD_ALIASES[fieldKey])
        case "submittedTeam":
            return row.submittedTeam ?? findRawValueByAliases(row.rawValues, OWNER_FIELD_ALIASES[fieldKey])
        case "submittedPhoneNumber":
            return row.submittedPhoneNumber ?? findRawValueByAliases(row.rawValues, OWNER_FIELD_ALIASES[fieldKey])
        default:
            return ""
    }
}

export function rowHasOwnerSnapshot(row: AssetImportRowRecord): boolean {
    return Boolean(
        getAssetImportOwnerFieldValue(row, "submittedStaffId") ||
            getAssetImportOwnerFieldValue(row, "submittedFullName") ||
            getAssetImportOwnerFieldValue(row, "submittedTeam") ||
            getAssetImportOwnerFieldValue(row, "submittedPhoneNumber"),
    )
}

export function buildDerivedComputerName(assetCode: string): string {
    const normalized = assetCode.trim().toUpperCase()
    if (!normalized) {
        return ""
    }
    return `ASW${normalized}`
}

export function formatDerivedComputerNames(assetCodes: readonly string[]): string {
    return assetCodes
        .map((assetCode) => buildDerivedComputerName(assetCode))
        .filter(Boolean)
        .join(",\n")
}

export function toBackendRowFieldKey(
    fieldKey: AssetImportWizardFieldKey | AssetImportOwnerFieldKey,
):
    | "assetType"
    | "displayName"
    | "brand"
    | "model"
    | "serialNumber"
    | "quantity"
    | "warehouse"
    | "notes"
    | "submittedStaffId"
    | "submittedFullName"
    | "submittedTeam"
    | "submittedPhoneNumber"
    | null {
    switch (fieldKey) {
        case "category":
            return "assetType"
        case "assetName":
        case "itemName":
            return "displayName"
        case "brand":
            return "brand"
        case "model":
            return "model"
        case "serialNumber":
            return "serialNumber"
        case "quantity":
            return "quantity"
        case "warehouse":
            return "warehouse"
        case "note":
            return "notes"
        case "submittedStaffId":
            return "submittedStaffId"
        case "submittedFullName":
            return "submittedFullName"
        case "submittedTeam":
            return "submittedTeam"
        case "submittedPhoneNumber":
            return "submittedPhoneNumber"
        default:
            return null
    }
}

function findRawValue(
    rawValues: AssetImportRawValue[],
    header: string | null | undefined,
): string {
    if (!header) {
        return ""
    }

    const normalizedTarget = normalizeHeader(header)
    const matched = rawValues.find(
        (rawValue) => normalizeHeader(rawValue.header) === normalizedTarget,
    )
    return matched?.value ?? ""
}

function findRawValueByAliases(
    rawValues: AssetImportRawValue[],
    aliases: readonly string[],
): string {
    const matched = rawValues.find((rawValue) =>
        aliases.some((alias) => normalizeLookupKey(alias) === normalizeLookupKey(rawValue.header)),
    )
    return matched?.value ?? ""
}

function normalizeHeader(value: string): string {
    return value
        .normalize("NFD")
        .replace(/\p{M}/gu, "")
        .trim()
        .toLowerCase()
        .replace(/[_-]/g, " ")
        .replace(/\s+/g, " ")
}

function normalizeLookupKey(value: string): string {
    return value
        .normalize("NFD")
        .replace(/\p{M}/gu, "")
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9]/g, "")
}
