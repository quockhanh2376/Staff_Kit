import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { AssetDashboard } from "./AssetDashboard"
import type { AssetDashboardState } from "./useAssetDashboardState"
import type { AssetImportState } from "./useAssetDirectImportState"

type SerializedRow = AssetDashboardState["serializedRows"][number]

function createRow(assetId: number, displayName: string): SerializedRow {
  return {
    assetId,
    assetCode: `AST-${String(assetId).padStart(4, "0")}`,
    categoryCode: "H_PHONE",
    categoryName: "H Phone",
    displayName,
    displayNameShort: null,
    brand: "Samsung",
    computerName: null,
    model: "M3",
    serialNumber: null,
    usageLocation: null,
    adapterNumber: null,
    notes: null,
    status: "in_stock",
    holderEmployeeId: null,
    holderFullName: null,
    signatureStatus: null,
  }
}

describe("AssetDashboard Edit asset", () => {
  const store = new Map<string, string>()

  beforeEach(() => {
    store.clear()
    Object.defineProperty(window, "localStorage", {
      value: {
        getItem: (key: string) => store.get(key) ?? null,
        setItem: (key: string, value: string) => store.set(key, value),
        removeItem: (key: string) => store.delete(key),
        clear: () => store.clear(),
        key: (index: number) => Array.from(store.keys())[index] ?? null,
        get length() {
          return store.size
        },
      },
      writable: true,
      configurable: true,
    })
  })

  function renderDashboard(isSuperAdminAccount: boolean, updateAssetDetails = vi.fn()) {
    const assetDashboard = {
      summary: null,
      serializedRows: [
        createRow(1, "Samsung Galaxy A17"),
        createRow(3, "Samsung Galaxy A18"),
        createRow(4, "Samsung Galaxy A19"),
      ],
      quantityRows: [],
      categoryDetails: [],
      isLoadingDashboard: false,
      isSavingAssetId: null,
      statusMessage: "",
      refreshDashboard: vi.fn(),
      updateAssetDetails,
    } as unknown as AssetDashboardState
    const assetImport = {
      isImportModalOpen: false,
      openImportModal: vi.fn(),
      closeImportModal: vi.fn(),
      isSubmitting: false,
    } as unknown as AssetImportState

    render(
      <AssetDashboard
        activeUserScope="test-user"
        auth={{ isAdminAccount: true, isSuperAdminAccount, canImportData: true }}
        assetDashboard={assetDashboard}
        assetImport={assetImport}
        triggerReload={vi.fn()}
      />,
    )
    return updateAssetDetails
  }

  it("hides Edit buttons for admins who are not super admin", () => {
    renderDashboard(false)
    expect(screen.queryByRole("button", { name: /^Edit AST-/ })).toBeNull()
  })

  it("saves edited display name for the clicked row", async () => {
    const updateAssetDetails = renderDashboard(true, vi.fn().mockResolvedValue({ id: 3 }))

    fireEvent.click(screen.getByRole("button", { name: "Edit AST-0003" }))
    expect(screen.queryByRole("textbox", { name: /asset code/i })).toBeNull()
    expect(screen.getByLabelText("Display name")).toHaveValue("Samsung Galaxy A18")

    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "Samsung Galaxy A17" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }))

    expect(updateAssetDetails).toHaveBeenCalledWith(
      3,
      expect.objectContaining({ displayName: "Samsung Galaxy A17", model: "M3", notes: null }),
    )
    expect(await screen.findByRole("button", { name: "Edit AST-0003" })).toBeInTheDocument()
    expect(screen.queryByLabelText("Display name")).toBeNull()
  })

  it("keeps the drawer open with the draft when save fails", async () => {
    const updateAssetDetails = renderDashboard(true, vi.fn().mockResolvedValue(null))

    fireEvent.click(screen.getByRole("button", { name: "Edit AST-0004" }))
    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "Samsung Galaxy A17" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }))

    await vi.waitFor(() => expect(updateAssetDetails).toHaveBeenCalled())
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(screen.getByLabelText("Display name")).toHaveValue("Samsung Galaxy A17")
  })

  it("disables save when the display name is blank", () => {
    renderDashboard(true)
    fireEvent.click(screen.getByRole("button", { name: "Edit AST-0001" }))
    fireEvent.change(screen.getByLabelText("Display name"), { target: { value: "   " } })
    expect(screen.getByRole("button", { name: "Save changes" })).toBeDisabled()
  })
})
