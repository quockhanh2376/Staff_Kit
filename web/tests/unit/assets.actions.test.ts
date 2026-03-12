import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  revalidatePath: vi.fn(),
  requireApiActor: vi.fn(),
  createAssetRecord: vi.fn(),
  updateAssetRecord: vi.fn(),
  preloadAssets: vi.fn(),
  parseAssetPreloadFile: vi.fn(),
}));

vi.mock("next/cache", () => ({
  revalidatePath: mocks.revalidatePath,
}));

vi.mock("@/lib/api/auth", () => ({
  requireApiActor: mocks.requireApiActor,
}));

vi.mock("@/lib/assets/assets.service", () => ({
  createAssetRecord: mocks.createAssetRecord,
  updateAssetRecord: mocks.updateAssetRecord,
  preloadAssets: mocks.preloadAssets,
}));

vi.mock("@/lib/assets/assets-preload-parser", () => ({
  parseAssetPreloadFile: mocks.parseAssetPreloadFile,
}));

import {
  createAssetAction,
  preloadAssetsAction,
  updateAssetAction,
} from "@/app/(protected)/assets/actions";

const actor = {
  accountId: 100,
  username: "itadmin",
  displayName: "IT Admin",
  role: "ADMIN" as const,
};

function buildCreateFormData() {
  const formData = new FormData();
  formData.set("assetCode", "AST-500");
  formData.set("name", "Dell Latitude 7450");
  formData.set("assetType", "Laptop");
  formData.set("status", "RETIRED");
  formData.set("recordedAt", "2026-03-10");
  formData.set("owningUnit", "IT");
  formData.set("managingUnit", "IT");
  formData.set("serialNumber", "SN-500");
  formData.set("brand", "Dell");
  formData.set("modelName", "Latitude 7450");
  formData.set("notes", "Seeded from action test");
  formData.set("retiredAt", "2026-03-12");
  formData.set("disposedAt", "");
  return formData;
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.requireApiActor.mockResolvedValue(actor);
});

describe("asset actions", () => {
  it("creates an asset from full-form input", async () => {
    mocks.createAssetRecord.mockResolvedValue({
      assetCode: "AST-500",
    });

    const result = await createAssetAction(buildCreateFormData());

    expect(mocks.createAssetRecord).toHaveBeenCalledWith(
      actor,
      expect.objectContaining({
        assetCode: "AST-500",
        name: "Dell Latitude 7450",
        assetType: "Laptop",
        status: "RETIRED",
        retiredAt: expect.any(Date),
        disposedAt: undefined,
      }),
    );
    expect(mocks.revalidatePath).toHaveBeenCalledWith("/assets");
    expect(result).toMatchObject({
      ok: true,
      assetCode: "AST-500",
    });
  });

  it("updates an asset from edit form input", async () => {
    mocks.updateAssetRecord.mockResolvedValue({
      assetCode: "AST-500",
    });

    const formData = new FormData();
    formData.set("assetCode", "AST-500");
    formData.set("name", "Updated Latitude 7450");
    formData.set("status", "DISPOSED");
    formData.set("disposedAt", "2026-03-13");

    const result = await updateAssetAction(formData);

    expect(mocks.updateAssetRecord).toHaveBeenCalledWith(
      actor,
      "AST-500",
      expect.objectContaining({
        name: "Updated Latitude 7450",
        status: "DISPOSED",
        disposedAt: expect.any(Date),
      }),
    );
    expect(result).toMatchObject({
      ok: true,
      assetCode: "AST-500",
    });
  });

  it("returns validation error for invalid retired and disposed dates", async () => {
    const formData = new FormData();
    formData.set("assetCode", "AST-500");
    formData.set("name", "Broken asset");
    formData.set("assetType", "Laptop");
    formData.set("status", "IN_STOCK");
    formData.set("retiredAt", "2026-03-12");

    const result = await createAssetAction(formData);

    expect(mocks.createAssetRecord).not.toHaveBeenCalled();
    expect(result).toMatchObject({
      ok: false,
      errorCode: "validation_error",
    });
  });

  it("preloads valid rows from an uploaded file", async () => {
    mocks.parseAssetPreloadFile.mockResolvedValue({
      validRows: [
        {
          assetCode: "AST-CSV-1",
          name: "Dock",
          assetType: "Dock",
          status: "IN_STOCK",
        },
      ],
      invalidRows: [],
      summary: {
        totalRows: 1,
        validRows: 1,
        invalidRows: 0,
      },
    });
    mocks.preloadAssets.mockResolvedValue({
      total: 1,
      created: 1,
      updated: 0,
    });

    const formData = new FormData();
    formData.set("file", new File(["csv"], "assets.csv", { type: "text/csv" }));

    const result = await preloadAssetsAction(formData);

    expect(mocks.preloadAssets).toHaveBeenCalledWith(actor, {
      assets: [
        {
          assetCode: "AST-CSV-1",
          name: "Dock",
          assetType: "Dock",
          status: "IN_STOCK",
        },
      ],
    });
    expect(result).toMatchObject({
      ok: true,
      summary: {
        total: 1,
        created: 1,
        updated: 0,
      },
    });
  });
});
