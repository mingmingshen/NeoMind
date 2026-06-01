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
