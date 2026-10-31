import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { AssetDashboard } from "./AssetDashboard"
import type { AssetDashboardState } from "./useAssetDashboardState"
import type { AssetImportState } from "./useAssetDirectImportState"

type SerializedRow = AssetDashboardState["serializedRows"][number]

function createMockSerializedRow(index: number): SerializedRow {
  return {
    assetId: index + 1,
    assetCode: `AST-${String(index + 1).padStart(4, "0")}`,
    categoryCode: "LAPTOP",
    categoryName: "Laptop",
    displayName: `MacBook Pro ${index + 1}`,
    displayNameShort: `MBP-${index + 1}`,
    computerName: `MAC-${index + 1}`,
    model: "M3",
    serialNumber: `SN${index + 1000}`,
    usageLocation: "HCM Office",
    adapterNumber: null,
    notes: null,
    status: "in_use",
    holderEmployeeId: "ASWVN001",
    holderFullName: "Nguyen Van A",
    signatureStatus: null,
  }
}

describe("AssetDashboard SerializedAssetTable client-side pagination", () => {
  const store = new Map<string, string>()
  const mockLocalStorage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => store.set(key, value),
    removeItem: (key: string) => store.delete(key),
    clear: () => store.clear(),
    key: (index: number) => Array.from(store.keys())[index] ?? null,
    get length() {
      return store.size
    },
  }

  beforeEach(() => {
    store.clear()
    Object.defineProperty(window, "localStorage", {
      value: mockLocalStorage,
      writable: true,
      configurable: true,
    })
  })

  const mockRows: SerializedRow[] = Array.from({ length: 60 }, (_, i) =>
    createMockSerializedRow(i),
  )

  const mockAssetDashboard = {
    summary: null,
    serializedRows: mockRows,
    quantityRows: [],
    categoryDetails: [],
    isLoadingDashboard: false,
    isLoadingSummary: false,
    isUpdatingStockItemId: null,
    isSavingCategory: false,
    isDeactivatingCategoryId: null,
    errorMessage: null,
    reloadSummary: vi.fn(),
    reloadDashboard: vi.fn(),
    updateStockItemQuantity: vi.fn(),
    saveCategory: vi.fn(),
    deactivateCategory: vi.fn(),
  } as unknown as AssetDashboardState

  const mockAssetImport = {
    isImportModalOpen: false,
    openImportModal: vi.fn(),
    closeImportModal: vi.fn(),
    isSubmitting: false,
  } as unknown as AssetImportState

  const mockAuth = {
    isAdminAccount: true,
    canImportData: true,
  }

  it("renders 25 rows by default with 60 total assets and pagination status", () => {
    render(
      <AssetDashboard
        activeUserScope="test-user"
        auth={mockAuth}
        assetDashboard={mockAssetDashboard}
        assetImport={mockAssetImport}
        triggerReload={vi.fn()}
      />,
    )

    // Table rows rendered (excluding table header)
    const rows = screen.getAllByRole("row")
    // Header row + 25 data rows = 26 rows
    expect(rows.length).toBe(26)

    // Shows 1–25 of 60 assets
    expect(screen.getByText("Showing 1–25 of 60 assets")).toBeInTheDocument()
    expect(screen.getByText("1/3")).toBeInTheDocument()

    // First asset visible, 26th asset not visible on page 1
    expect(screen.getByText("MacBook Pro 1")).toBeInTheDocument()
    expect(screen.queryByText("MacBook Pro 26")).not.toBeInTheDocument()
  })

  it("navigates to page 2 when clicking Next", () => {
    render(
      <AssetDashboard
        activeUserScope="test-user"
        auth={mockAuth}
        assetDashboard={mockAssetDashboard}
        assetImport={mockAssetImport}
        triggerReload={vi.fn()}
      />,
    )

    const nextButton = screen.getByRole("button", { name: "Next" })
    expect(nextButton).not.toBeDisabled()

    fireEvent.click(nextButton)

    // Shows 26–50 of 60 assets
    expect(screen.getByText("Showing 26–50 of 60 assets")).toBeInTheDocument()
    expect(screen.getByText("2/3")).toBeInTheDocument()

    expect(screen.queryByText("MacBook Pro 1")).not.toBeInTheDocument()
    expect(screen.getByText("MacBook Pro 26")).toBeInTheDocument()
  })

  it("switches rows per page to 50 and updates total pages to 2", () => {
    render(
      <AssetDashboard
        activeUserScope="test-user"
        auth={mockAuth}
        assetDashboard={mockAssetDashboard}
        assetImport={mockAssetImport}
        triggerReload={vi.fn()}
      />,
    )

    const select = screen.getByLabelText("Rows per page")
    fireEvent.change(select, { target: { value: "50" } })

    expect(screen.getByText("Showing 1–50 of 60 assets")).toBeInTheDocument()
    expect(screen.getByText("1/2")).toBeInTheDocument()

    const rows = screen.getAllByRole("row")
    // Header row + 50 data rows = 51 rows
    expect(rows.length).toBe(51)
  })

  it("resets to page 1 when search filter changes", () => {
    render(
      <AssetDashboard
        activeUserScope="test-user"
        auth={mockAuth}
        assetDashboard={mockAssetDashboard}
        assetImport={mockAssetImport}
        triggerReload={vi.fn()}
      />,
    )

    // Navigate to page 2
    fireEvent.click(screen.getByRole("button", { name: "Next" }))
    expect(screen.getByText("2/3")).toBeInTheDocument()

    // Type in search input: exact code AST-0005
    const searchInput = screen.getByPlaceholderText(/search/i)
    fireEvent.change(searchInput, { target: { value: "AST-0005" } })

    // Page must reset to 1
    expect(screen.getByText("1/1")).toBeInTheDocument()
    expect(screen.getByText("Showing 1–1 of 1 assets")).toBeInTheDocument()
    expect(screen.getByText("MacBook Pro 5")).toBeInTheDocument()
  })
})
