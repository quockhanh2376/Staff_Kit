import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

vi.mock("@/app/(protected)/assets/actions", () => ({
  preloadAssetsAction: vi.fn(),
}));

vi.mock("@/lib/assets/assets-preload-parser", () => ({
  parseAssetPreloadFile: vi.fn(),
}));

import { AssetPreloadModal } from "@/components/assets/AssetPreloadModal";

describe("AssetPreloadModal", () => {
  it("renders preload upload controls and validation summary", () => {
    const markup = renderToStaticMarkup(
      <AssetPreloadModal
        isOpen
        onClose={() => undefined}
        onSubmitted={() => undefined}
      />,
    );

    expect(markup).toContain("Upload CSV or XLSX");
    expect(markup).toContain("Valid rows");
    expect(markup).toContain("Invalid rows");
  });
});
