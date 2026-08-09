import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import { BorrowPolicyCard } from "./BorrowPolicyCard"
import { BORROW_REVIEW_DESCRIPTION_TYPOGRAPHY } from "./borrowReviewCopy"

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
    expect(screen.queryByText("English", { exact: true })).toBeNull()
    expect(screen.queryByText("Vietnamese", { exact: true })).toBeNull()
    expect(screen.getByLabelText("English Handle with Care policy").getAttribute("spellcheck")).toBe("false")
    expect(screen.getByLabelText("Vietnamese Handle with Care policy").getAttribute("spellcheck")).toBe("false")
    const english = screen.getByLabelText("English Handle with Care policy")
    const vietnamese = screen.getByLabelText("Vietnamese Handle with Care policy")
    expect(BORROW_REVIEW_DESCRIPTION_TYPOGRAPHY).toBe("borrow-review-description-typography")
    expect(english.className).toContain(BORROW_REVIEW_DESCRIPTION_TYPOGRAPHY)
    expect(vietnamese.className).toContain(BORROW_REVIEW_DESCRIPTION_TYPOGRAPHY)
    for (const textarea of [english, vietnamese]) {
      expect(textarea.className.split(/\s+/).filter((token) => /^(text|font|leading|tracking)-/.test(token))).toEqual([])
    }
    expect(screen.getByLabelText("Vietnamese Handle with Care policy").className).toContain("w-full")
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
