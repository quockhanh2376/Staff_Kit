import { describe, expect, it } from "vitest";

import {
  assetCreateSchema,
  assetPreloadSchema,
  assetUpdateSchema,
} from "@/lib/assets/assets.schemas";

describe("assetCreateSchema", () => {
  it("rejects retiredAt when status is not RETIRED", () => {
    expect(() =>
      assetCreateSchema.parse({
        name: "Dell Latitude",
        assetType: "Laptop",
        status: "IN_STOCK",
        retiredAt: "2026-03-12",
      }),
    ).toThrow(/retired/i);
  });

  it("rejects disposedAt when status is not DISPOSED", () => {
    expect(() =>
      assetCreateSchema.parse({
        name: "Dell Latitude",
        assetType: "Laptop",
        status: "RETIRED",
        disposedAt: "2026-03-12",
      }),
    ).toThrow(/disposed/i);
  });

  it("accepts retiredAt when status is RETIRED", () => {
    const result = assetCreateSchema.parse({
      name: "Dell Latitude",
      assetType: "Laptop",
      status: "RETIRED",
      retiredAt: "2026-03-12",
    });

    expect(result.retiredAt).toBeInstanceOf(Date);
  });

  it("accepts disposedAt when status is DISPOSED", () => {
    const result = assetCreateSchema.parse({
      name: "Dell Latitude",
      assetType: "Laptop",
      status: "DISPOSED",
      disposedAt: "2026-03-12",
    });

    expect(result.disposedAt).toBeInstanceOf(Date);
  });
});

describe("assetUpdateSchema", () => {
  it("rejects disposedAt when status is RETIRED", () => {
    expect(() =>
      assetUpdateSchema.parse({
        status: "RETIRED",
        disposedAt: "2026-03-12",
      }),
    ).toThrow(/disposed/i);
  });

  it("accepts retiredAt when status is RETIRED", () => {
    const result = assetUpdateSchema.parse({
      status: "RETIRED",
      retiredAt: "2026-03-12",
    });

    expect(result.retiredAt).toBeInstanceOf(Date);
  });
});

describe("assetPreloadSchema", () => {
  it("requires assetCode for preload rows", () => {
    expect(() =>
      assetPreloadSchema.parse({
        assets: [
          {
            name: "Dock",
            assetType: "Dock",
          },
        ],
      }),
    ).toThrow(/assetCode/i);
  });
});
