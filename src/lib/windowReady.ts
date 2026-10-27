import { staffApi } from '../services/staff-api'

export function scheduleShowMainWindow(
  api: Pick<typeof staffApi, 'showMainWindow'> = staffApi,
): void {
  requestAnimationFrame(() => {
    try {
      performance.mark('app-first-paint')
    } catch {
      // performance mark is best-effort
    }
    void api.showMainWindow().catch((err) => {
      console.error('[Staff Kit] Failed to show main window after first paint:', err)
    })
  })
}
