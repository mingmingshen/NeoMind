import type { StateCreator } from 'zustand'
import type { AiAnalystConfig } from '@/components/dashboard/generic/ai-analyst/types'

export interface AiAnalystState {
  analystConfigs: Record<string, AiAnalystConfig>
}

export interface AiAnalystSlice extends AiAnalystState {
  getAnalystConfig: (componentId: string) => AiAnalystConfig | undefined
  setAnalystConfig: (componentId: string, config: Partial<AiAnalystConfig>) => void
  clearAnalystConfig: (componentId: string) => void
}

const DEFAULT_CONFIG: AiAnalystConfig = {
  systemPrompt: 'You are a professional data analysis assistant. Analyze the provided data — images, metrics, or structured data — describe what you observe, and point out any notable patterns, changes, or anomalies.',
  contextWindowSize: 10,
}

export const createAiAnalystSlice: StateCreator<
  AiAnalystSlice,
  [],
  [],
  AiAnalystSlice
> = (set, get) => ({
  analystConfigs: {},

  getAnalystConfig: (componentId: string) => {
    return get().analystConfigs[componentId]
  },

  setAnalystConfig: (componentId: string, config: Partial<AiAnalystConfig>) => {
    set((state) => ({
      analystConfigs: {
        ...state.analystConfigs,
        [componentId]: {
          ...(state.analystConfigs[componentId] || DEFAULT_CONFIG),
          ...config,
        },
      },
    }))
  },

  clearAnalystConfig: (componentId: string) => {
    set((state) => {
      const { [componentId]: _, ...rest } = state.analystConfigs
      return { analystConfigs: rest }
    })
  },
})
