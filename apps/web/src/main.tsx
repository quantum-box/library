import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'
import './app/globals.css'
import './app/animations.css'

const router = createRouter({ routeTree })

// Router type registration is in src/router-overrides.d.ts
// Using AnyRouter during migration to allow string interpolation in Link `to` props

const rootElement = document.getElementById('root')!
if (!rootElement.innerHTML) {
  const root = createRoot(rootElement)
  root.render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>,
  )
}
