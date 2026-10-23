import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { staffApi } from './services/staff-api'

// Signal main window readiness as soon as bundle begins execution
void staffApi.showMainWindow().catch(() => {})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
