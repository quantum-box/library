import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { TooltipProvider } from '@tachyon-sdk/native-ui'
import './index.css'
import { router } from './router'
import { ThemeProvider } from './contexts/ThemeContext'
import { AppUpdateNotice } from './components/AppUpdateNotice'
import { NativeMenuLabels } from './components/desktop/NativeMenuLabels'
import { I18nProvider, preloadInitialLocale } from './i18n'

// Resolve the starting language before the first paint so the shell never
// flashes English on its way to the user's locale.
void preloadInitialLocale().then((locale) => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <I18nProvider initial={locale}>
        <ThemeProvider>
          <TooltipProvider delayDuration={350}>
            <RouterProvider router={router} />
            <AppUpdateNotice />
            <NativeMenuLabels />
          </TooltipProvider>
        </ThemeProvider>
      </I18nProvider>
    </StrictMode>,
  )
})
