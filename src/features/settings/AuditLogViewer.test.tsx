import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import { describe, it, expect, vi, beforeEach } from "vitest"
import { AuditLogViewer } from "./AuditLogViewer"
import { staffApi } from "../../services/staff-api"

vi.mock("../../services/staff-api", () => ({
  staffApi: {
    queryAuditLogs: vi.fn(),
  },
}))

describe("AuditLogViewer", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it("super_admin can view category filters and query logs", async () => {
    vi.mocked(staffApi.queryAuditLogs).mockResolvedValueOnce({
      logs: [
        {
          id: 1,
          eventType: "security.auth.login_success",
          actorType: "account",
          actorRef: "super_admin",
          entityType: "session",
          entityId: "1",
          payloadJson: JSON.stringify({ role: "super_admin" }),
          createdAt: "2026-10-31 10:00:00",
        },
        {
          id: 2,
          eventType: "business.export.file",
          actorType: "account",
          actorRef: "admin_user",
          entityType: "export",
          entityId: "staff.csv",
          payloadJson: JSON.stringify({ bytes_written: 1024 }),
          createdAt: "2026-10-31 10:05:00",
        },
      ],
      totalCount: 2,
    })

    render(<AuditLogViewer userRole="super_admin" />)

    expect(screen.getByText("All Events")).toBeInTheDocument()
    expect(screen.getByText("Security")).toBeInTheDocument()
    expect(screen.getByText("Business")).toBeInTheDocument()

    await waitFor(() => {
      expect(screen.getByText("auth.login_success")).toBeInTheDocument()
      expect(screen.getByText("export.file")).toBeInTheDocument()
    })
  })

  it("admin sees constrained business-only notice and queries business logs", async () => {
    vi.mocked(staffApi.queryAuditLogs).mockResolvedValueOnce({
      logs: [
        {
          id: 2,
          eventType: "business.export.file",
          actorType: "account",
          actorRef: "admin_user",
          entityType: "export",
          entityId: "staff.csv",
          payloadJson: JSON.stringify({ bytes_written: 1024 }),
          createdAt: "2026-10-31 10:05:00",
        },
      ],
      totalCount: 1,
    })

    render(<AuditLogViewer userRole="admin" />)

    expect(screen.getByText(/Business Audit Log \(Read-only\)/i)).toBeInTheDocument()
    expect(screen.queryByText("All Events")).not.toBeInTheDocument()

    await waitFor(() => {
      expect(staffApi.queryAuditLogs).toHaveBeenCalledWith(
        expect.objectContaining({
          eventCategory: "business",
        })
      )
      expect(screen.getByText("export.file")).toBeInTheDocument()
    })
  })

  it("opens details modal when clicking an entry", async () => {
    vi.mocked(staffApi.queryAuditLogs).mockResolvedValueOnce({
      logs: [
        {
          id: 99,
          eventType: "security.account.created",
          actorType: "account",
          actorRef: "super_admin",
          entityType: "account",
          entityId: "2",
          payloadJson: JSON.stringify({ created_username: "new_staff" }),
          createdAt: "2026-10-31 12:00:00",
        },
      ],
      totalCount: 1,
    })

    render(<AuditLogViewer userRole="super_admin" />)

    await waitFor(() => {
      expect(screen.getByText("account.created")).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText("Details"))

    expect(screen.getByText("Audit Entry #99")).toBeInTheDocument()
    expect(screen.getByText("new_staff", { exact: false })).toBeInTheDocument()

    fireEvent.click(screen.getByText("Close"))
    expect(screen.queryByText("Audit Entry #99")).not.toBeInTheDocument()
  })
})
