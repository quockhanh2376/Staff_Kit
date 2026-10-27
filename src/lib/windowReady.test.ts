import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { scheduleShowMainWindow } from './windowReady'

describe('scheduleShowMainWindow', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('schedules showMainWindow exactly once on the next animation frame', async () => {
    let frameCallback: FrameRequestCallback | null = null
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
      frameCallback = cb
      return 1
    })

    const showMainWindowMock = vi.fn().mockResolvedValue(undefined)
    scheduleShowMainWindow({ showMainWindow: showMainWindowMock })

    expect(window.requestAnimationFrame).toHaveBeenCalledTimes(1)
    expect(showMainWindowMock).not.toHaveBeenCalled()

    expect(frameCallback).not.toBeNull()
    frameCallback!(performance.now())

    expect(showMainWindowMock).toHaveBeenCalledTimes(1)
  })

  it('handles showMainWindow rejection without uncaught exception', async () => {
    let frameCallback: FrameRequestCallback | null = null
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
      frameCallback = cb
      return 1
    })
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    const showMainWindowMock = vi.fn().mockRejectedValue(new Error('window not ready'))
    scheduleShowMainWindow({ showMainWindow: showMainWindowMock })

    frameCallback!(performance.now())

    // Allow promise rejection microtask to settle
    await Promise.resolve()

    expect(showMainWindowMock).toHaveBeenCalledTimes(1)
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      '[Staff Kit] Failed to show main window after first paint:',
      expect.any(Error),
    )
  })
})
