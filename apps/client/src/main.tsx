import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { TooltipProvider } from '@tachyon-sdk/native-ui'
import './index.css'
import { router } from './router'
import { ThemeProvider } from './contexts/ThemeContext'
import { AppUpdateNotice } from './components/AppUpdateNotice'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <TooltipProvider delayDuration={350}>
        <RouterProvider router={router} />
        <AppUpdateNotice />
      </TooltipProvider>
    </ThemeProvider>
  </StrictMode>,
)
