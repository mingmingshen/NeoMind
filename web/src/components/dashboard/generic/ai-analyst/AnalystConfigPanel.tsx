import { useState, useEffect } from 'react'
import { Settings2, Cpu, MessageSquare, Gauge } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Badge } from '@/components/ui/badge'
import { useAnalystModels } from './useAnalystModels'
import type { AiAnalystConfig } from './types'

interface AnalystConfigPanelProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  config: AiAnalystConfig
  onSave: (config: Partial<AiAnalystConfig>) => void
  dataSource?: string
}

export function AnalystConfigPanel({ open, onOpenChange, config, onSave, dataSource }: AnalystConfigPanelProps) {
  const [systemPrompt, setSystemPrompt] = useState(config.systemPrompt)
  const [modelId, setModelId] = useState(config.modelId || '')
  const [contextWindowSize, setContextWindowSize] = useState(config.contextWindowSize)
  const { models, loading } = useAnalystModels()

  useEffect(() => {
    setSystemPrompt(config.systemPrompt)
    setModelId(config.modelId || '')
    setContextWindowSize(config.contextWindowSize)
  }, [config])

  const handleSave = () => {
    const selectedModel = models.find((m) => m.id === modelId)
    onSave({
      systemPrompt,
      modelId: modelId || undefined,
      modelName: selectedModel?.name,
      contextWindowSize,
    })
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-base">
            <Settings2 className="h-4 w-4" />
            AI Analyst Configuration
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Data Source */}
          <div>
            <label className="text-xs text-muted-foreground">Data Source</label>
            <div className="mt-1 px-3 py-2 rounded-lg bg-muted/30 border border-border/50 text-sm">
              {dataSource || 'Not configured (use component settings)'}
            </div>
          </div>

          {/* Model Selection */}
          <div>
            <label className="text-xs text-muted-foreground flex items-center gap-1">
              <Cpu className="h-4 w-4" />
              Analysis Model
            </label>
            {loading ? (
              <div className="mt-1 text-xs text-muted-foreground">Loading models...</div>
            ) : models.length === 0 ? (
              <div className="mt-1 text-xs text-yellow-600">No vision models found. Configure an LLM backend with a vision-capable model first.</div>
            ) : (
              <select
                value={modelId}
                onChange={(e) => setModelId(e.target.value)}
                className="mt-1 w-full rounded-lg border border-border/50 bg-background px-3 py-2 text-sm"
              >
                <option value="">Select a model</option>
                {models.map((m) => (
                  <option key={`${m.backendId}-${m.id}`} value={m.id}>
                    {m.name} ({m.backendName})
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* System Prompt */}
          <div>
            <label className="text-xs text-muted-foreground flex items-center gap-1">
              <MessageSquare className="h-4 w-4" />
              System Prompt
            </label>
            <Textarea
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              className="mt-1 min-h-[80px] text-xs"
              placeholder="Describe how the AI should analyze the data..."
            />
          </div>

          {/* Context Window */}
          <div>
            <label className="text-xs text-muted-foreground flex items-center gap-1">
              <Gauge className="h-4 w-4" />
              Context Window
            </label>
            <div className="mt-1 flex items-center gap-3">
              <input
                type="range"
                min={1}
                max={20}
                step={1}
                value={contextWindowSize}
                onChange={(e) => setContextWindowSize(Number(e.target.value))}
                className="flex-1"
              />
              <Badge variant="outline" className="text-xs h-6 px-2">
                {contextWindowSize}
              </Badge>
            </div>
            <p className="text-[10px] text-muted-foreground mt-1">
              Number of recent data+response pairs to display in timeline
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave}>Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
