import { describe, expect, it, vi } from "vitest"

import { scheduleMainWindowShow } from "./showMainWindowWhenReady"

describe("scheduleMainWindowShow", () => {
  it("waits for the next animation frame before asking Tauri to show the window", async () => {
    const show = vi.fn().mockResolvedValue(undefined)
    let frameCallback: FrameRequestCallback | undefined
    const scheduleFrame = vi.fn((callback: FrameRequestCallback) => {
      frameCallback = callback
      return 7
    })

    scheduleMainWindowShow(show, scheduleFrame, vi.fn())

    expect(scheduleFrame).toHaveBeenCalledTimes(1)
    expect(show).not.toHaveBeenCalled()

    frameCallback?.(0)
    await Promise.resolve()

    expect(show).toHaveBeenCalledTimes(1)
  })
})
