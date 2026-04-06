import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

vi.mock("@/app/(protected)/assets/actions", () => ({
  createAssetAction: vi.fn(),
  updateAssetAction: vi.fn(),
}));

import { AssetFormModal } from "@/components/assets/AssetFormModal";

describe("AssetFormModal", () => {
  it("renders full asset fields including retiredAt and disposedAt", () => {
    const markup = renderToStaticMarkup(
      <AssetFormModal
        isOpen
        mode="create"
        onClose={() => undefined}
        onSubmitted={() => undefined}
      />,
    );

    expect(markup).toContain("Asset Code");
    expect(markup).toContain("Serial Number");
    expect(markup).toContain("Retired At");
    expect(markup).toContain("Disposed At");
  });
});
