import { describe, expect, it, vi } from "vitest"

import { scheduleMainWindowShow } from "./showMainWindowWhenReady"

describe("scheduleMainWindowShow", () => {
  it("uses a timer so the request runs while the WebView remains hidden", async () => {
    const show = vi.fn().mockResolvedValue(undefined)
    let timerCallback: (() => void) | undefined
    const scheduleTimer = vi.fn((callback: () => void) => {
      timerCallback = callback
      return 7
    })

    scheduleMainWindowShow(show, scheduleTimer, vi.fn())

    expect(scheduleTimer).toHaveBeenCalledWith(expect.any(Function), 50)
    expect(show).not.toHaveBeenCalled()

    timerCallback?.()
    await Promise.resolve()

    expect(show).toHaveBeenCalledTimes(1)
  })
})
