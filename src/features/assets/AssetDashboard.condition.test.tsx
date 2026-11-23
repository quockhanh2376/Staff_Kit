import { fireEvent, render, screen, within } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { AssetDashboard } from "./AssetDashboard"
import type { AssetDashboardState } from "./useAssetDashboardState"
import type { AssetImportState } from "./useAssetDirectImportState"

type SerializedRow = AssetDashboardState["serializedRows"][number]

function createRow(
  assetId: number,
  condition: SerializedRow["condition"],
  status = "in_stock",
): SerializedRow {
  return {
    assetId,
    assetCode: `AST-${String(assetId).padStart(4, "0")}`,
    categoryCode: "H_PHONE",
    categoryName: "H Phone",
    displayName: `Phone ${assetId}`,
    displayNameShort: null,
    brand: "Samsung",
    computerName: null,
    model: "M3",
    serialNumber: null,
    usageLocation: null,
    adapterNumber: null,
    notes: null,
    status,
    holderEmployeeId: null,
    holderFullName: null,
    signatureStatus: null,
    condition,
    holderTeamName: null,
  }
}

const ROWS = [
  createRow(1, "new"),
  createRow(2, "spare"),
  createRow(3, "spare"),
  createRow(4, "broken"),
  createRow(5, "spare", "assigned"),
]

describe("AssetDashboard condition", () => {
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

  function renderDashboard(
    isAdminAccount: boolean,
    updateAssetCondition = vi.fn(),
    conditionError: string | null = null,
  ) {
    const assetDashboard = {
      summary: null,
      serializedRows: ROWS,
      quantityRows: [],
      categoryDetails: [],
      isLoadingDashboard: false,
      isSavingAssetId: null,
      isSavingConditionAssetId: null,
      assetEditError: null,
      conditionError,
      clearAssetEditError: vi.fn(),
      statusMessage: "",
      refreshDashboard: vi.fn(),
      updateAssetDetails: vi.fn(),
      updateAssetCondition,
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
        auth={{ isAdminAccount, isSuperAdminAccount: false, canImportData: true }}
        assetDashboard={assetDashboard}
        assetImport={assetImport}
        triggerReload={vi.fn()}
      />,
    )
    return updateAssetCondition
  }

  it("admin sees condition selector and saving calls updateAssetCondition(3, \"broken\")", () => {
    const updateAssetCondition = renderDashboard(true, vi.fn().mockResolvedValue({ id: 3 }))
    const select = screen.getByRole("combobox", { name: "Condition AST-0003" })
    expect(select).toHaveValue("spare")
    fireEvent.change(select, { target: { value: "broken" } })
    expect(updateAssetCondition).toHaveBeenCalledWith(3, "broken")
  })

  it("plain user sees condition text, no selector", () => {
    renderDashboard(false)
    expect(screen.queryByRole("combobox", { name: /^Condition AST-/ })).toBeNull()
    const row = screen.getByText("Phone 4").closest("tr") as HTMLElement
    expect(within(row).getByText("Broken")).toBeInTheDocument()
  })

  it("New option is disabled for an assigned asset", () => {
    renderDashboard(true)
    const assigned = screen.getByRole("combobox", { name: "Condition AST-0005" })
    expect(within(assigned).getByRole("option", { name: "New" })).toBeDisabled()
    const inStock = screen.getByRole("combobox", { name: "Condition AST-0002" })
    expect(within(inStock).getByRole("option", { name: "New" })).not.toBeDisabled()
  })

  it("refused New shows the backend error", () => {
    renderDashboard(true, vi.fn(), "Cannot set condition to new after first hand-out")
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Cannot set condition to new after first hand-out",
    )
  })

  it("condition filter Broken shows only broken rows", () => {
    renderDashboard(true)
    fireEvent.change(screen.getByRole("combobox", { name: "Filter serialized assets by condition" }), {
      target: { value: "broken" },
    })
    expect(screen.getByText("Phone 4")).toBeInTheDocument()
    expect(screen.queryByText("Phone 1")).toBeNull()
    expect(screen.queryByText("Phone 2")).toBeNull()
    expect(screen.queryByText("Phone 5")).toBeNull()
  })

  it("cards count in-stock New/Spare/Broken and assigned", () => {
    renderDashboard(true)
    const value = (key: string) => screen.getByTestId(`condition-card-${key}`).textContent
    expect(value("total")).toBe("5")
    expect(value("assigned")).toBe("1")
    expect(value("new")).toBe("1")
    expect(value("spare")).toBe("2")
    expect(value("broken")).toBe("1")
  })
})
