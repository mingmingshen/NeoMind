export interface VlmVisionConfig {
  agentId?: string
  sessionId?: string
  modelId?: string
  modelName?: string
  systemPrompt: string
  contextWindowSize: number
}

export type VlmMessageType = 'image' | 'ai' | 'user' | 'error'

export interface VlmMessage {
  id: string
  type: VlmMessageType
  content: string
  timestamp: number
  dataSource?: string
  modelName?: string
  duration?: number
  isStreaming?: boolean
}

export interface VisionModel {
  id: string
  name: string
  backendId: string
  backendName: string
}

export const DEFAULT_SYSTEM_PROMPT =
  'You are a professional image analysis assistant. Carefully observe the image content, describe the scene, and point out any notable changes or anomalies.'
