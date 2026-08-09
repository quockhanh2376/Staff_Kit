import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import { BorrowPolicyCard } from "./BorrowPolicyCard"

function renderCard(overrides: Partial<Parameters<typeof BorrowPolicyCard>[0]> = {}) {
  return render(
    <BorrowPolicyCard
      english="Handle equipment carefully."
      vietnamese="Vui lòng giữ gìn thiết bị."
      savedEnglish="Handle equipment carefully."
      savedVietnamese="Vui lòng giữ gìn thiết bị."
      isLoading={false}
      isSaving={false}
      message=""
      onEnglishChange={vi.fn()}
      onVietnameseChange={vi.fn()}
      onSave={vi.fn()}
      {...overrides}
    />,
  )
}

describe("BorrowPolicyCard", () => {
  it("keeps English and Vietnamese in separate fields and disables unchanged save", () => {
    renderCard()

    expect(screen.getByLabelText("English Handle with Care policy")).toBeTruthy()
    expect(screen.getByLabelText("Vietnamese Handle with Care policy")).toBeTruthy()
    expect((screen.getByRole("button", { name: "Save Handle with Care policy" }) as HTMLButtonElement).disabled).toBe(true)
  })

  it("enables save when dirty and sends explicit save action", () => {
    const onSave = vi.fn()
    renderCard({ onSave, english: "Updated policy" })
    const save = screen.getByRole("button", { name: "Save Handle with Care policy" })
    expect((save as HTMLButtonElement).disabled).toBe(false)
    fireEvent.click(save)
    expect(onSave).toHaveBeenCalledTimes(1)
  })
})
