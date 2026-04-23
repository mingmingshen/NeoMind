import type { StateCreator } from 'zustand'
import type { VlmVisionConfig } from '@/components/dashboard/generic/vlm-vision/types'

export interface VlmVisionState {
  vlmConfigs: Record<string, VlmVisionConfig>
}

export interface VlmVisionSlice extends VlmVisionState {
  getVlmConfig: (componentId: string) => VlmVisionConfig | undefined
  setVlmConfig: (componentId: string, config: Partial<VlmVisionConfig>) => void
  clearVlmConfig: (componentId: string) => void
}

const DEFAULT_CONFIG: VlmVisionConfig = {
  systemPrompt: 'You are a professional image analysis assistant. Carefully observe the image content, describe the scene, and point out any notable changes or anomalies.',
  contextWindowSize: 10,
}

export const createVlmVisionSlice: StateCreator<
  VlmVisionSlice,
  [],
  [],
  VlmVisionSlice
> = (set, get) => ({
  vlmConfigs: {},

  getVlmConfig: (componentId: string) => {
    return get().vlmConfigs[componentId]
  },

  setVlmConfig: (componentId: string, config: Partial<VlmVisionConfig>) => {
    set((state) => ({
      vlmConfigs: {
        ...state.vlmConfigs,
        [componentId]: {
          ...(state.vlmConfigs[componentId] || DEFAULT_CONFIG),
          ...config,
        },
      },
    }))
  },

  clearVlmConfig: (componentId: string) => {
    set((state) => {
      const { [componentId]: _, ...rest } = state.vlmConfigs
      return { vlmConfigs: rest }
    })
  },
})
