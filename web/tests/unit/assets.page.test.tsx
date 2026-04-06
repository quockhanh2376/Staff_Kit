import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

vi.mock("@/app/(protected)/assets/actions", () => ({
  createAssetAction: vi.fn(),
  updateAssetAction: vi.fn(),
  preloadAssetsAction: vi.fn(),
}));

vi.mock("@/lib/assets/assets-preload-parser", () => ({
  parseAssetPreloadFile: vi.fn(),
}));

import { AssetsPageShell } from "@/components/assets/AssetsPageShell";

const assets = [
  {
    id: 1,
    assetCode: "AST-1001",
    name: "Dell Latitude 7450",
    assetType: "Laptop",
    status: "ASSIGNED",
    owningUnit: "IT",
    managingUnit: "IT",
    serialNumber: "SN-1001",
    recordedAt: new Date("2026-03-10T08:00:00.000Z"),
    assignments: [
      {
        employee: {
          employeeId: "ADP001",
          fullName: "Nguyen Minh Anh",
        },
      },
    ],
  },
];

describe("AssetsPageShell", () => {
  it("renders the asset workstation with filter controls and primary actions", () => {
    const markup = renderToStaticMarkup(
      <AssetsPageShell
        assets={assets}
        filters={{
          q: "",
          status: "",
          assetType: "",
        }}
      />,
    );

    expect(markup).toContain("New Asset");
    expect(markup).toContain("Preload Assets");
    expect(markup).toContain("Current Holder");
    expect(markup).toContain("AST-1001");
    expect(markup).toContain("Nguyen Minh Anh");
  });
});
