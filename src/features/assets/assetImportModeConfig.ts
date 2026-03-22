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
    "model",
    "serialNumber",
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

const LEGACY_ASSET_CODE_ALIASES = [
    "assetcode",
    "asset code",
    "code",
    "asset id",
    "assetid",
    "asset number",
    "assetnumber",
]

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
            model: mapping.model ?? null,
            note: mapping.notes ?? null,
        }
    }

    return {
        category: mapping.assetType ?? null,
        assetName: mapping.displayName ?? null,
        model: mapping.model ?? null,
        serialNumber: mapping.serialNumber ?? null,
        note: mapping.notes ?? null,
    }
}

export function toBackendAssetImportMapping(
    mode: AssetImportMode,
    mapping: AssetImportWizardMapping,
    headers: string[],
): AssetImportFieldMapping | null {
    if (mode !== "serialized") {
        return null
    }

    const legacyAssetCodeHeader = detectLegacyAssetCodeHeader(headers)
    if (!legacyAssetCodeHeader) {
        return null
    }

    return {
        assetCode: legacyAssetCodeHeader,
        assetType: mapping.category ?? null,
        displayName: mapping.assetName ?? null,
        model: mapping.model ?? null,
        serialNumber: mapping.serialNumber ?? null,
        notes: mapping.note ?? null,
    }
}

export function canStageAssetImportMode(
    mode: AssetImportMode,
    headers: string[],
    mapping: AssetImportWizardMapping,
): boolean {
    if (!hasRequiredAssetImportMapping(mapping, mode)) {
        return false
    }

    if (mode === "quantity") {
        return false
    }

    return detectLegacyAssetCodeHeader(headers) !== null
}

export function getAssetImportStageBlockReason(
    mode: AssetImportMode,
    headers: string[],
    mapping: AssetImportWizardMapping,
): string | null {
    if (!hasRequiredAssetImportMapping(mapping, mode)) {
        return "Map all required columns before staging a batch."
    }

    if (mode === "quantity") {
        return "Quantity batch staging lands in the next slice. This pass wires mode-aware wizard state only."
    }

    if (!detectLegacyAssetCodeHeader(headers)) {
        return "Serialized staging still needs the next backend slice for auto-generated asset codes. Legacy files with an Asset Code column can still stage now."
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
        case "model":
            return row.model ?? ""
        case "serialNumber":
            return row.serialNumber ?? ""
        case "note":
            return row.notes ?? findRawValue(row.rawValues, mapping.note)
        case "brand":
        case "quantity":
        case "warehouse":
            return findRawValue(row.rawValues, mapping[fieldKey])
        default:
            return ""
    }
}

export function toBackendRowFieldKey(
    fieldKey: AssetImportWizardFieldKey,
): "assetType" | "displayName" | "model" | "serialNumber" | "notes" | null {
    switch (fieldKey) {
        case "category":
            return "assetType"
        case "assetName":
        case "itemName":
            return "displayName"
        case "model":
            return "model"
        case "serialNumber":
            return "serialNumber"
        case "note":
            return "notes"
        default:
            return null
    }
}

function detectLegacyAssetCodeHeader(headers: string[]): string | null {
    const matched = headers.find((header) => {
        const normalizedHeader = normalizeHeader(header)
        return LEGACY_ASSET_CODE_ALIASES.some(
            (alias) => normalizeHeader(alias) === normalizedHeader,
        )
    })
    return matched ?? null
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

function normalizeHeader(value: string): string {
    return value
        .trim()
        .toLowerCase()
        .replace(/[_-]/g, " ")
        .replace(/\s+/g, " ")
}
