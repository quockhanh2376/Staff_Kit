import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { scheduleShowMainWindow } from './lib/windowReady'

try {
  performance.mark('app-bundle-start')
} catch {
  void 0
}

// Pre-render theme initialization from localStorage to prevent flash
try {
  const saved = localStorage.getItem('staffkit-theme') ?? 'dark'
  document.documentElement.classList.toggle('dark', saved === 'dark')
} catch {
  void 0
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)

scheduleShowMainWindow()
