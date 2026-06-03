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
;(window as any).neomind = {
  /**
   * Call an extension command from a frontend component.
   * POST /api/extensions/:id/command
   */
  callExtension: async (extensionId: string, command: string, args?: Record<string, unknown>) => {
    const { api } = await import('@/lib/api')
    return api.executeCommand(extensionId, { command, args: args || {} })
  },

  /**
   * List all installed extensions.
   * GET /api/extensions
   */
  listExtensions: async () => {
    const { api } = await import('@/lib/api')
    return api.listExtensions()
  },

  /**
   * Create a transform automation.
   * Maps component TransformConfig → backend CreateAutomationRequest.
   * POST /api/automations
   */
  createTransform: async (config: {
    name: string
    scope: string
    extension_id: string
    command?: string
    input?: Record<string, unknown>
    output?: Record<string, unknown>
    args?: Record<string, unknown>
    rule: { device_id: string; device_type?: string }
  }) => {
    const { api } = await import('@/lib/api')

    // Map component template config → backend TransformAutomation format
    // Backend expects: { scope: {device: "..."}, operations: [{op_type: "extension", ...}] }
    const parameters: Record<string, unknown> = { ...(config.input || {}), ...(config.args || {}) }
    const outputMetrics = config.output ? Object.keys(config.output) : []

    const definition: Record<string, unknown> = {
      scope: { device: config.scope },
      operations: [{
        op_type: 'extension',
        extension_id: config.extension_id,
        command: config.command || 'detect',
        parameters,
        output_metrics: outputMetrics,
        output_mapping: config.output || null,
      }],
    }

    const result = await api.createAutomation({
      name: config.name,
      type: 'transform',
      enabled: true,
      definition,
    })

    // Return shape expected by components: { id, name, scope, extension_id, rule, status, created_at }
    const automation = (result as any).automation
    return {
      id: automation.id,
      name: automation.name,
      scope: config.scope,
      extension_id: config.extension_id,
      rule: config.rule,
      status: automation.enabled ? 'active' : 'paused',
      created_at: automation.created_at,
    }
  },

  /**
   * Delete a transform automation.
   * DELETE /api/automations/:id
   */
  deleteTransform: async (id: string) => {
    const { api } = await import('@/lib/api')
    await api.deleteAutomation(id)
  },

  /**
   * List transform automations, optionally filtered.
   * GET /api/automations?type=transform
   */
  listTransforms: async (filter?: { scope?: string; extension_id?: string }) => {
    const { api } = await import('@/lib/api')
    const result = await api.listAutomations({ type: 'transform' })
    return result.automations.map((a: any) => ({
      id: a.id,
      name: a.name,
      scope: a.scope || '',
      extension_id: a.extension_id || '',
      rule: a.rule || {},
      status: a.enabled ? 'active' : 'paused',
      created_at: a.created_at,
    }))
  },

  /**
   * Write a virtual metric value for a device.
   * POST /api/automations/transforms/process
   */
  writeMetric: async (deviceId: string, metric: string, value: unknown) => {
    // Virtual metrics are produced by transforms.
    // For direct writes from components, we use the transform process endpoint
    // with a minimal inline payload that sets the metric directly.
    const { api } = await import('@/lib/api')
    try {
      await api.processTransformData({
        device_id: deviceId,
        data: { [metric]: value },
      })
    } catch {
      // Silent degradation — components should never block on metric writes
    }
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
