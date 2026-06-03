import { StrictMode } from "react"
import * as React from "react"
import { createRoot } from "react-dom/client"
import { BrowserRouter } from "react-router-dom"
import * as jsxRuntime from "react/jsx-runtime"
import "./index.css"
import "./i18n/config"
import App from "./App"
import { ThemeProvider } from "@/components/ui/theme"
import { initVisualViewport } from "@/hooks/useVisualViewport"

// Expose React and JSX runtime as globals for community component bundles (IIFE)
;(window as any).React = React
;(window as any).jsxRuntime = jsxRuntime

// Expose NeoMind API for community/extension components
// Components can call: window.neomind.callExtension(id, command, args)
;(window as any).neomind = {
  /**
   * Call an extension command from a frontend component.
   * @param extensionId - Extension ID (e.g. "yolo-device-inference")
   * @param command - Command name (e.g. "analyze")
   * @param args - Command arguments (optional)
   * @returns Promise with the command result
   */
  callExtension: async (extensionId: string, command: string, args?: Record<string, unknown>) => {
    const { getApiBase } = await import('@/lib/api')
    const apiBase = getApiBase()

    // Get auth token if available
    let token: string | null = null
    try {
      const tokenStr = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token')
      if (tokenStr) {
        const parsed = JSON.parse(tokenStr)
        token = parsed?.token || parsed
      }
    } catch {}

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    }
    if (token) {
      headers['Authorization'] = `Bearer ${token}`
    }

    const response = await fetch(`${apiBase}/extensions/${extensionId}/command`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ command, args: args || {} }),
    })

    if (!response.ok) {
      const text = await response.text().catch(() => 'Unknown error')
      return { success: false, error: `HTTP ${response.status}: ${text}` }
    }

    return response.json()
  },
}

// Initialize global VisualViewport tracking for mobile keyboard handling
initVisualViewport()

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter future={{ v7_relativeSplatPath: true, v7_startTransition: true }}>
      <ThemeProvider>
        <App />
      </ThemeProvider>
    </BrowserRouter>
  </StrictMode>,
)
