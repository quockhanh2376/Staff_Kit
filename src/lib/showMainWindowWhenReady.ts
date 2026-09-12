export function scheduleMainWindowShow(
  show: () => Promise<void>,
  scheduleTimer: (callback: () => void, delay: number) => number,
  reportError: (message: string, error: unknown) => void,
): number {
  return scheduleTimer(() => {
    void show().catch((error: unknown) => {
      reportError("[Staff Kit] failed to show main window:", error)
    })
  }, 50)
}
