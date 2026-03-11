import { prisma } from "@/lib/prisma";
import type { AssetStatus } from "@/generated/prisma/client";

type AssetRepositoryClient = Pick<typeof prisma, "asset">;

export type AssetListFilters = {
  q?: string;
  status?: AssetStatus;
  assetType?: string;
  take?: number;
};

export type AssetWriteInput = {
  assetCode: string;
  name: string;
  assetType: string;
  status?: AssetStatus;
  recordedAt?: Date;
  owningUnit?: string;
  managingUnit?: string;
  serialNumber?: string;
  brand?: string;
  modelName?: string;
  notes?: string;
};

export type AssetUpdateData = Partial<Omit<AssetWriteInput, "assetCode">>;

export async function listAssets(db: AssetRepositoryClient, filters: AssetListFilters) {
  return db.asset.findMany({
    where: {
      OR: filters.q
        ? [
            {
              assetCode: {
                contains: filters.q,
                mode: "insensitive",
              },
            },
            {
              name: {
                contains: filters.q,
                mode: "insensitive",
              },
            },
          ]
        : undefined,
      status: filters.status,
      assetType: filters.assetType
        ? {
            equals: filters.assetType,
            mode: "insensitive",
          }
        : undefined,
    },
    orderBy: {
      createdAt: "desc",
    },
    take: filters.take ?? 50,
    include: {
      assignments: {
        where: {
          status: "ACTIVE",
        },
        orderBy: {
          assignedAt: "desc",
        },
        take: 1,
        include: {
          employee: {
            select: {
              id: true,
              employeeId: true,
              fullName: true,
            },
          },
        },
      },
    },
  });
}

export async function findAssetByCode(
  db: AssetRepositoryClient,
  assetCode: string,
) {
  return db.asset.findUnique({
    where: {
      assetCode,
    },
    include: {
      assignments: {
        orderBy: {
          assignedAt: "desc",
        },
        include: {
          employee: {
            select: {
              id: true,
              employeeId: true,
              fullName: true,
            },
          },
        },
      },
    },
  });
}

export async function findAssetsByCodes(
  db: AssetRepositoryClient,
  assetCodes: string[],
) {
  return db.asset.findMany({
    where: {
      assetCode: {
        in: assetCodes,
      },
    },
  });
}

export async function createAsset(
  db: AssetRepositoryClient,
  data: AssetWriteInput,
) {
  return db.asset.create({
    data,
  });
}

export async function updateAssetByCode(
  db: AssetRepositoryClient,
  assetCode: string,
  data: AssetUpdateData,
) {
  return db.asset.update({
    where: {
      assetCode,
    },
    data,
  });
}

export async function upsertAssetByCode(
  db: AssetRepositoryClient,
  assetCode: string,
  data: AssetWriteInput,
) {
  return db.asset.upsert({
    where: {
      assetCode,
    },
    create: data,
    update: {
      name: data.name,
      assetType: data.assetType,
      status: data.status,
      recordedAt: data.recordedAt,
      owningUnit: data.owningUnit,
      managingUnit: data.managingUnit,
      serialNumber: data.serialNumber,
      brand: data.brand,
      modelName: data.modelName,
      notes: data.notes,
    },
  });
}
