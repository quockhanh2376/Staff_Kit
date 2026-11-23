import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { EmployeeSyncDrawer } from "./EmployeeSyncDrawer"
import type { EmployeeState } from "./useEmployeeState"
import type { EmployeeSyncStagedRow } from "../../types/staff"

const DUPLICATE_NOTE = "Duplicate Staff Code in HR source"
const STALE_NOTE = "Changed in the app after this list was loaded — review again"

const fields = (fullName: string) => ({ fullName, nickName: null, email: null, azureAdAccount: null })

const row = (over: Partial<EmployeeSyncStagedRow>): EmployeeSyncStagedRow => ({
    id: 1,
    staffCode: "A1",
    kind: "new",
    localEmployeeId: null,
    localGroup: null,
    source: [fields("Source One")],
    local: null,
    edits: null,
    action: "apply",
    notes: [],
    state: "pending",
    assetsHeld: 0,
    isFloating: false,
    ...over,
})

const mixedRows = [
    row({ id: 1, staffCode: "A1", kind: "new", action: "apply" }),
    row({ id: 2, staffCode: "A2", kind: "changed", action: "apply", local: fields("Local Two") }),
    row({ id: 3, staffCode: "A3", kind: "conflict", action: "move_to_employee_list", localGroup: "offboarding" }),
    row({ id: 4, staffCode: "A4", kind: "missing", action: "move_to_offboarding", source: [], local: fields("Local Four") }),
    row({
        id: 5,
        staffCode: "A5",
        kind: "conflict",
        action: "skip",
        source: [fields("Dup A"), fields("Dup B")],
        notes: [DUPLICATE_NOTE],
    }),
]

function setup(rows: EmployeeSyncStagedRow[], over: Partial<EmployeeState> = {}) {
    const emp = {
        syncBatch: { id: 7, createdAt: "2026-11-23T00:00:00Z", counts: { new: 0, changed: 0, unchanged: 12, missing: 0, conflicts: 0 }, rows },
        isSyncBusy: false,
        syncError: null,
        updateSyncRow: vi.fn().mockResolvedValue(true),
        refreshSyncRow: vi.fn().mockResolvedValue(undefined),
        discardEmployeeSync: vi.fn().mockResolvedValue(true),
        applyEmployeeSync: vi.fn().mockResolvedValue({
            added: 1, updated: 2, movedToEmployeeList: 3, offboarded: 4, skipped: 0, completedAt: "2026-11-23T00:00:00Z",
        }),
        ...over,
    } as unknown as EmployeeState
    const onClose = vi.fn()
    const setGlobalNotice = vi.fn()
    render(<EmployeeSyncDrawer open employeeState={emp} onClose={onClose} setGlobalNotice={setGlobalNotice} />)
    return { emp, onClose, setGlobalNotice }
}

afterEach(() => vi.restoreAllMocks())

describe("EmployeeSyncDrawer", () => {
    it("renders rows with default actions per kind", () => {
        setup(mixedRows)
        expect(screen.getByLabelText("EE List sync")).toBeInTheDocument()
        expect(screen.getByLabelText("Action for A1")).toHaveDisplayValue("Apply")
        expect(screen.getByLabelText("Action for A2")).toHaveDisplayValue("Apply")
        expect(screen.getByLabelText("Action for A3")).toHaveDisplayValue("Move to EE List")
        expect(screen.getByLabelText("Action for A4")).toHaveDisplayValue("Move to Offboarding")
        const duplicate = screen.getByLabelText("Action for A5")
        expect(duplicate).toHaveDisplayValue("Skip")
        expect(duplicate).toBeDisabled()
        expect(duplicate.querySelectorAll("option")).toHaveLength(1)
        expect(screen.getByLabelText("Full name for A5")).toBeDisabled()
        // Kind chips come from rows; Unchanged from counts.
        expect(screen.getByText("New 1")).toBeInTheDocument()
        expect(screen.getByText("Changed 1")).toBeInTheDocument()
        expect(screen.getByText("Missing 1")).toBeInTheDocument()
        expect(screen.getByText("Conflicts 2")).toBeInTheDocument()
        expect(screen.getByText("Unchanged 12")).toBeInTheDocument()
    })

    it("changing action saves the row", () => {
        const { emp } = setup(mixedRows)
        fireEvent.change(screen.getByLabelText("Action for A4"), { target: { value: "none" } })
        expect(emp.updateSyncRow).toHaveBeenCalledWith(4, { action: "none" })
    })

    it("blur on edited name saves edits", () => {
        const { emp } = setup(mixedRows)
        const input = screen.getByLabelText("Full name for A1")
        expect(input).toHaveValue("Source One")
        fireEvent.change(input, { target: { value: "Edited One" } })
        fireEvent.blur(input)
        expect(emp.updateSyncRow).toHaveBeenCalledWith(1, { edits: fields("Edited One") })
    })

    it("fields stay editable while a row saves and keep earlier edits", () => {
        // isSyncBusy is true while the Full name save is in flight.
        const { emp } = setup(mixedRows, { isSyncBusy: true } as Partial<EmployeeState>)
        const fullName = screen.getByLabelText("Full name for A1")
        const nick = screen.getByLabelText("Nick for A1")
        fireEvent.change(fullName, { target: { value: "Edited One" } })
        fireEvent.blur(fullName)
        expect(nick).toBeEnabled()
        expect(screen.getByLabelText("Action for A1")).toBeEnabled()
        fireEvent.change(nick, { target: { value: "Nicky" } })
        expect(nick).toHaveValue("Nicky")
        fireEvent.blur(nick)
        expect(emp.updateSyncRow).toHaveBeenLastCalledWith(1, { edits: { ...fields("Edited One"), nickName: "Nicky" } })
    })

    it("a refused save shows the stored value again", async () => {
        const { emp } = setup(mixedRows, { updateSyncRow: vi.fn().mockResolvedValue(false) } as Partial<EmployeeState>)
        const fullName = screen.getByLabelText("Full name for A1")
        fireEvent.change(fullName, { target: { value: "" } })
        fireEvent.blur(fullName)
        expect(emp.updateSyncRow).toHaveBeenCalledWith(1, { edits: fields("") })
        await waitFor(() => expect(fullName).toHaveValue("Source One"))
    })

    it("approve label shows totals and is disabled when nothing writes", () => {
        setup(mixedRows)
        expect(screen.getByRole("button", { name: "Approve — Add 1 · Update 1 · Move 1 to EE List · Offboard 1" })).toBeEnabled()
    })

    it("approve is disabled when nothing writes", () => {
        setup([row({ action: "skip" }), row({ id: 2, staffCode: "A2", kind: "missing", action: "none", source: [] })])
        expect(screen.getByRole("button", { name: "Approve" })).toBeDisabled()
    })

    it("approve asks to confirm when offboarded people hold assets", async () => {
        const rows = [
            row({ id: 4, staffCode: "A4", kind: "missing", action: "move_to_offboarding", source: [], assetsHeld: 2 }),
            row({ id: 6, staffCode: "A6", kind: "missing", action: "move_to_offboarding", source: [], assetsHeld: 1 }),
        ]
        const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true)
        const { emp, onClose, setGlobalNotice } = setup(rows)
        const approve = screen.getByRole("button", { name: "Approve — Offboard 2" })
        fireEvent.click(approve)
        expect(confirm).toHaveBeenCalledWith("2 people being offboarded still hold 3 assets. Continue?")
        expect(emp.applyEmployeeSync).not.toHaveBeenCalled()
        fireEvent.click(approve)
        await waitFor(() => expect(onClose).toHaveBeenCalled())
        expect(emp.applyEmployeeSync).toHaveBeenCalledTimes(1)
        expect(setGlobalNotice).toHaveBeenCalledWith("EE List updated: added 1, updated 2, moved 3, offboarded 4.")
        expect(screen.getByText("Holds 2 assets")).toBeInTheDocument()
    })

    it("approve failure keeps the drawer open with the error", async () => {
        const { emp, onClose } = setup(mixedRows, {
            applyEmployeeSync: vi.fn().mockResolvedValue(null),
            syncError: "Cannot apply: 2 rows changed in the app — review them",
        } as Partial<EmployeeState>)
        expect(screen.getByRole("alert")).toHaveTextContent("Cannot apply: 2 rows changed in the app — review them")
        fireEvent.click(screen.getByRole("button", { name: /^Approve/ }))
        await waitFor(() => expect(emp.applyEmployeeSync).toHaveBeenCalled())
        expect(onClose).not.toHaveBeenCalled()
    })

    it("stale row shows Reload from app and calls refresh", () => {
        const { emp } = setup([row({ state: "stale", notes: [STALE_NOTE] })])
        expect(screen.getByText(STALE_NOTE)).toBeInTheDocument()
        expect(screen.getByLabelText("Action for A1").closest("tr")).toHaveClass("search-highlight-cell")
        fireEvent.click(screen.getByRole("button", { name: "Reload from app" }))
        expect(emp.refreshSyncRow).toHaveBeenCalledWith(1)
    })

    it("skip asks to confirm then discards", async () => {
        const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true)
        const { emp, onClose } = setup(mixedRows)
        fireEvent.click(screen.getByRole("button", { name: "Skip" }))
        expect(confirm).toHaveBeenCalledWith("Discard this EE List sync? Nothing will be written.")
        expect(emp.discardEmployeeSync).not.toHaveBeenCalled()
        fireEvent.click(screen.getByRole("button", { name: "Skip" }))
        expect(emp.discardEmployeeSync).toHaveBeenCalledTimes(1)
        await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1))
    })

    it("kind filter narrows the rows", () => {
        setup(mixedRows)
        fireEvent.change(screen.getByLabelText("Kind filter"), { target: { value: "missing" } })
        expect(screen.getByLabelText("Action for A4")).toBeInTheDocument()
        expect(screen.queryByLabelText("Action for A1")).not.toBeInTheDocument()
    })
})
