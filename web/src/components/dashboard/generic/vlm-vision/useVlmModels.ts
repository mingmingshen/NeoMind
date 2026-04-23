import { useState, useEffect, useCallback } from 'react'
import { api } from '@/lib/api'
import type { VisionModel } from './types'

export function useVlmModels() {
  const [models, setModels] = useState<VisionModel[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadModels = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const resp = await api.listLlmBackends()
      const backends = resp.backends || []
      const visionModels: VisionModel[] = []

      for (const backend of backends) {
        if (backend.capabilities?.supports_multimodal) {
          visionModels.push({
            id: backend.id,
            name: backend.name || backend.model,
            backendId: backend.id,
            backendName: backend.name || backend.id,
          })
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
