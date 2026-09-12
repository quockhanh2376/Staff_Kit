import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { scheduleMainWindowShow } from './lib/showMainWindowWhenReady.ts'
import { staffApi } from './services/staff-api.ts'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)

scheduleMainWindowShow(staffApi.showMainWindow, window.setTimeout, console.error)
