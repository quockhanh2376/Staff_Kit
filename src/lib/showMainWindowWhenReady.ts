export function scheduleMainWindowShow(
  show: () => Promise<void>,
  scheduleFrame: (callback: FrameRequestCallback) => number,
  reportError: (message: string, error: unknown) => void,
): number {
  return scheduleFrame(() => {
    void show().catch((error: unknown) => {
      reportError("[Staff Kit] failed to show main window:", error)
    })
  })
}
