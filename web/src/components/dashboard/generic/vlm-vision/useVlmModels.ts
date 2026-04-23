import { useState, useEffect, useCallback } from 'react'
import { api } from '@/lib/api'
import type { VisionModel } from './types'

const VISION_MODEL_PATTERNS = ['vl', 'vision', 'llava', 'bakllava', 'qwen-vl']

function isVisionModel(modelName: string): boolean {
  const lower = modelName.toLowerCase()
  return VISION_MODEL_PATTERNS.some((p) => lower.includes(p))
}

export function useVlmModels() {
  const [models, setModels] = useState<VisionModel[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadModels = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const resp = await api.listLlmBackends({ active_only: true })
      const backends = resp.backends || resp || []
      const visionModels: VisionModel[] = []

      for (const backend of Array.isArray(backends) ? backends : []) {
        const backendId = backend.id
        const backendName = backend.name || backendId

        // Check backend-level multimodal support
        if (backend.capabilities?.multimodal || backend.supports_multimodal) {
          if (backend.model || backend.default_model) {
            visionModels.push({
              id: backend.model || backend.default_model,
              name: backend.model || backend.default_model,
              backendId,
              backendName,
            })
          }
        }

        // For Ollama backends, try to list models and filter by vision capability
        if (backend.type === 'ollama' || backend.backend_type === 'ollama') {
          try {
            const modelsResp = await api.listOllamaModels()
            const modelList = modelsResp.models || []
            for (const m of modelList) {
              if (isVisionModel(m.name || m.model || '')) {
                visionModels.push({
                  id: m.name || m.model,
                  name: m.name || m.model,
                  backendId,
                  backendName,
                })
              }
            }
          } catch {
            // Skip if model listing fails for this backend
          }
        }
      }

      setModels(visionModels)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load models')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadModels()
  }, [loadModels])

  return { models, loading, error, reload: loadModels }
}