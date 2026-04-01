"use server";

import { revalidatePath } from "next/cache";
import { ZodError } from "zod";

import { requireApiActor } from "@/lib/api/auth";
import { ApiError } from "@/lib/api/errors";
import {
  parseAssetPreloadFile,
  type AssetPreloadInvalidRow,
} from "@/lib/assets/assets-preload-parser";
import {
  createAssetRecord,
  preloadAssets,
  updateAssetRecord,
} from "@/lib/assets/assets.service";
import {
  assetCreateSchema,
  assetUpdateSchema,
} from "@/lib/assets/assets.schemas";

type AssetMutationSummary = {
  total: number;
  created: number;
  updated: number;
};

export type AssetActionResult =
  | {
      ok: true;
      message: string;
      assetCode?: string;
      summary?: AssetMutationSummary;
    }
  | {
      ok: false;
      errorCode: string;
      message: string;
      fieldErrors?: Record<string, string[] | undefined>;
      invalidRows?: AssetPreloadInvalidRow[];
    };

function normalizeFormValue(value: FormDataEntryValue | null) {
  if (typeof value !== "string") {
    return undefined;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

/**
 * For date fields that must be explicitly clearable.
 * Returns `null` (not `undefined`) when blank, so Prisma writes NULL to the
 * column instead of silently skipping the field.
 */
function normalizeFormDate(value: FormDataEntryValue | null): string | null {
  if (typeof value !== "string") {
    return null;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function readAssetFormInput(formData: FormData) {
  return {
    assetCode: normalizeFormValue(formData.get("assetCode")),
    name: normalizeFormValue(formData.get("name")),
    assetType: normalizeFormValue(formData.get("assetType")),
    status: normalizeFormValue(formData.get("status")),
    recordedAt: normalizeFormValue(formData.get("recordedAt")),
    owningUnit: normalizeFormValue(formData.get("owningUnit")),
    managingUnit: normalizeFormValue(formData.get("managingUnit")),
    serialNumber: normalizeFormValue(formData.get("serialNumber")),
    brand: normalizeFormValue(formData.get("brand")),
    modelName: normalizeFormValue(formData.get("modelName")),
    notes: normalizeFormValue(formData.get("notes")),
    retiredAt: normalizeFormDate(formData.get("retiredAt")),
    disposedAt: normalizeFormDate(formData.get("disposedAt")),
  };
}

function revalidateAssetPaths() {
  revalidatePath("/assets");
  revalidatePath("/dashboard");
  revalidatePath("/audit");
}

function getActionErrorResult(error: unknown): AssetActionResult {
  if (error instanceof ApiError) {
    return {
      ok: false,
      errorCode: error.code,
      message: error.message,
    };
  }

  if (error instanceof ZodError) {
    return {
      ok: false,
      errorCode: "validation_error",
      message: "Asset input validation failed.",
      fieldErrors: error.flatten().fieldErrors,
    };
  }

  return {
    ok: false,
    errorCode: "internal_error",
    message: "An unexpected error occurred.",
  };
}

export async function createAssetAction(formData: FormData): Promise<AssetActionResult> {
  try {
    const actor = await requireApiActor("ADMIN");
    const input = assetCreateSchema.parse(readAssetFormInput(formData));
    const asset = await createAssetRecord(actor, input);

    revalidateAssetPaths();

    return {
      ok: true,
      message: `Created asset ${asset.assetCode}.`,
      assetCode: asset.assetCode,
    };
  } catch (error) {
    return getActionErrorResult(error);
  }
}

export async function updateAssetAction(formData: FormData): Promise<AssetActionResult> {
  try {
    const actor = await requireApiActor("ADMIN");
    const rawInput = readAssetFormInput(formData);
    const assetCode = rawInput.assetCode;

    if (!assetCode) {
      return {
        ok: false,
        errorCode: "validation_error",
        message: "Asset code is required for updates.",
        fieldErrors: {
          assetCode: ["Asset code is required for updates."],
        },
      };
    }

    const input = assetUpdateSchema.parse({
      name: rawInput.name,
      assetType: rawInput.assetType,
      status: rawInput.status,
      recordedAt: rawInput.recordedAt,
      owningUnit: rawInput.owningUnit,
      managingUnit: rawInput.managingUnit,
      serialNumber: rawInput.serialNumber,
      brand: rawInput.brand,
      modelName: rawInput.modelName,
      notes: rawInput.notes,
      retiredAt: rawInput.retiredAt,
      disposedAt: rawInput.disposedAt,
    });

    const asset = await updateAssetRecord(actor, assetCode, input);

    revalidateAssetPaths();

    return {
      ok: true,
      message: `Updated asset ${asset.assetCode}.`,
      assetCode: asset.assetCode,
    };
  } catch (error) {
    return getActionErrorResult(error);
  }
}

export async function preloadAssetsAction(formData: FormData): Promise<AssetActionResult> {
  try {
    const actor = await requireApiActor("ADMIN");
    const file = formData.get("file");

    if (!(file instanceof File) || file.size === 0) {
      return {
        ok: false,
        errorCode: "validation_error",
        message: "A CSV or XLSX file is required.",
        fieldErrors: {
          file: ["A CSV or XLSX file is required."],
        },
      };
    }

    const parsed = await parseAssetPreloadFile(file);

    if (parsed.invalidRows.length > 0) {
      return {
        ok: false,
        errorCode: "validation_error",
        message: "Resolve invalid preload rows before submitting.",
        invalidRows: parsed.invalidRows,
      };
    }

    const summary = await preloadAssets(actor, {
      assets: parsed.validRows,
    });

    revalidateAssetPaths();

    return {
      ok: true,
      message: `Preloaded ${summary.total} assets.`,
      summary,
    };
  } catch (error) {
    return getActionErrorResult(error);
  }
}
