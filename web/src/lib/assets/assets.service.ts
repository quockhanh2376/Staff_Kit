import { Prisma } from "@/generated/prisma/client";

import type { ApiActor } from "@/lib/api/auth";
import { ApiError } from "@/lib/api/errors";
import { prisma } from "@/lib/prisma";
import { writeAuditLog } from "@/lib/audit/audit.service";

import {
  createAsset,
  findAssetByCode,
  findAssetsByCodes,
  listAssets,
  updateAssetByCode,
  upsertAssetByCode,
} from "./assets.repository";
import type {
  AssetCreateInput,
  AssetListFiltersInput,
  AssetPreloadInput,
  AssetUpdateInput,
} from "./assets.schemas";

function isUniqueConstraintError(error: unknown) {
  return (
    error instanceof Prisma.PrismaClientKnownRequestError && error.code === "P2002"
  );
}

async function generateAssetCode() {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    const candidate = `AST-${crypto.randomUUID().slice(0, 8).toUpperCase()}`;
    const existing = await findAssetByCode(prisma, candidate);

    if (!existing) {
      return candidate;
    }
  }

  throw new ApiError(
    500,
    "asset_code_generation_failed",
    "Unable to generate a unique asset code.",
  );
}

export async function getAssets(filters: AssetListFiltersInput) {
  return listAssets(prisma, filters);
}

export async function getAssetByCode(assetCode: string) {
  const asset = await findAssetByCode(prisma, assetCode);

  if (!asset) {
    throw new ApiError(404, "asset_not_found", `Asset ${assetCode} was not found.`);
  }

  return asset;
}

export async function createAssetRecord(
  actor: ApiActor,
  input: AssetCreateInput,
) {
  const assetCode = input.assetCode ?? (await generateAssetCode());

  try {
    return await prisma.$transaction(async (tx) => {
      const asset = await createAsset(tx, {
        assetCode,
        name: input.name,
        assetType: input.assetType,
        status: input.status ?? "IN_STOCK",
        recordedAt: input.recordedAt,
        owningUnit: input.owningUnit,
        managingUnit: input.managingUnit,
        serialNumber: input.serialNumber,
        brand: input.brand,
        modelName: input.modelName,
        notes: input.notes,
      });

      await writeAuditLog(tx, {
        actor,
        actionType: "asset.create",
        entityType: "asset",
        entityId: asset.assetKey,
        entityLabel: asset.assetCode,
        assetId: asset.id,
        metadata: {
          assetType: asset.assetType,
          status: asset.status,
        },
      });

      return asset;
    });
  } catch (error) {
    if (isUniqueConstraintError(error)) {
      throw new ApiError(
        409,
        "duplicate_asset_code",
        `Asset code ${assetCode} already exists.`,
      );
    }

    throw error;
  }
}

export async function preloadAssets(
  actor: ApiActor,
  input: AssetPreloadInput,
) {
  const assetCodes = input.assets.map((asset) => asset.assetCode);
  const existingAssets = await findAssetsByCodes(prisma, assetCodes);
  const existingCodeSet = new Set(existingAssets.map((asset) => asset.assetCode));

  const result = await prisma.$transaction(async (tx) => {
    for (const asset of input.assets) {
      await upsertAssetByCode(tx, asset.assetCode, {
        assetCode: asset.assetCode,
        name: asset.name,
        assetType: asset.assetType,
        status: asset.status ?? "IN_STOCK",
        recordedAt: asset.recordedAt,
        owningUnit: asset.owningUnit,
        managingUnit: asset.managingUnit,
        serialNumber: asset.serialNumber,
        brand: asset.brand,
        modelName: asset.modelName,
        notes: asset.notes,
      });
    }

    await writeAuditLog(tx, {
      actor,
      actionType: "asset.preload",
      entityType: "asset_batch",
      entityLabel: "asset-preload",
      metadata: {
        total: input.assets.length,
        created: input.assets.filter((asset) => !existingCodeSet.has(asset.assetCode)).length,
        updated: input.assets.filter((asset) => existingCodeSet.has(asset.assetCode)).length,
        assetCodes,
      },
    });

    return {
      total: input.assets.length,
      created: input.assets.filter((asset) => !existingCodeSet.has(asset.assetCode)).length,
      updated: input.assets.filter((asset) => existingCodeSet.has(asset.assetCode)).length,
    };
  });

  return result;
}

export async function updateAssetRecord(
  actor: ApiActor,
  assetCode: string,
  input: AssetUpdateInput,
) {
  const existingAsset = await findAssetByCode(prisma, assetCode);

  if (!existingAsset) {
    throw new ApiError(404, "asset_not_found", `Asset ${assetCode} was not found.`);
  }

  const updatedAsset = await prisma.$transaction(async (tx) => {
    const asset = await updateAssetByCode(tx, assetCode, input);

    await writeAuditLog(tx, {
      actor,
      actionType: "asset.update",
      entityType: "asset",
      entityId: asset.assetKey,
      entityLabel: asset.assetCode,
      assetId: asset.id,
      metadata: {
        before: {
          name: existingAsset.name,
          assetType: existingAsset.assetType,
          status: existingAsset.status,
        },
        after: {
          name: asset.name,
          assetType: asset.assetType,
          status: asset.status,
        },
      },
    });

    return asset;
  });

  return updatedAsset;
}
