import { useState, useEffect, useCallback, useMemo } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import {
  Clock,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Brain,
  Database,
  Play,
  ChevronRight,
  Sparkles,
  Image as ImageIcon,
  ChevronDown,
  ChevronUp,
  Monitor,
  X,
} from "lucide-react"
import { api } from "@/lib/api"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { formatTimestamp } from "@/lib/utils/format"
import { MarkdownMessage } from "@/components/chat/MarkdownMessage"
import type { AgentExecutionDetail, DataCollected } from "@/types"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"

interface ExecutionDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  agentId: string
  executionId: string
}

export function ExecutionDetailDialog({
  open,
  onOpenChange,
  agentId,
  executionId,
}: ExecutionDetailDialogProps) {
  const { t } = useTranslation(['common', 'agents'])
  const { handleError } = useErrorHandler()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [execution, setExecution] = useState<AgentExecutionDetail | null>(null)
  const [loading, setLoading] = useState(false)
  const [expandedDataIndices, setExpandedDataIndices] = useState<Set<number>>(new Set())

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    if (open && agentId && executionId) {
      loadExecution()
    }
  }, [open, agentId, executionId])

  const loadExecution = async () => {
    setLoading(true)
    try {
      const data = await api.getExecution(agentId, executionId)
      setExecution(data)
    } catch (error) {
      handleError(error, { operation: 'Load execution', showToast: false })
    } finally {
      setLoading(false)
    }
  }

  const handleClose = useCallback(() => {
    onOpenChange(false)
  }, [onOpenChange])

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Completed':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />
      case 'Failed':
        return <XCircle className="h-4 w-4 text-red-500" />
      case 'Running':
        return <Clock className="h-4 w-4 text-blue-500" />
      default:
        return <AlertCircle className="h-4 w-4 text-muted-foreground" />
    }
  }

  const toggleDataExpanded = (index: number) => {
    setExpandedDataIndices(prev => {
      const next = new Set(prev)
      if (next.has(index)) {
        next.delete(index)
      } else {
        next.add(index)
      }
      return next
    })
  }

  const isPureBase64 = (str: string): boolean => {
    if (!str || str.length < 100) return false
    const cleaned = str.trim()

    if (cleaned.startsWith('http://') || cleaned.startsWith('https://') || cleaned.startsWith('/')) {
      return false
    }
    if (cleaned.startsWith('data:')) {
      return false
    }

    const base64Regex = /^[A-Za-z0-9+/=_-]+$/
    if (!base64Regex.test(cleaned)) {
      return false
    }

    try {
      atob(cleaned.slice(0, 100))
      return true
    } catch {
      return false
    }
  }

  const detectImageFormat = (base64Data: string): { mime: string } | null => {
    try {
      const pureBase64 = base64Data.replace(/^data:image\/[^;]+;base64,/, '').replace(/^data:,/, '')
      const binaryString = atob(pureBase64.slice(0, 32))

      const magicBytes: Record<string, { magic: number[]; mime: string }> = {
        png: { magic: [0x89, 0x50, 0x4E, 0x47], mime: 'image/png' },
        jpeg: { magic: [0xFF, 0xD8, 0xFF], mime: 'image/jpeg' },
        gif: { magic: [0x47, 0x49, 0x46], mime: 'image/gif' },
        webp: { magic: [0x52, 0x49, 0x46, 0x46], mime: 'image/webp' },
        bmp: { magic: [0x42, 0x4D], mime: 'image/bmp' },
      }

      for (const info of Object.values(magicBytes)) {
        if (info.magic.every((byte, i) => binaryString.charCodeAt(i) === byte)) {
          return { mime: info.mime }
        }
      }
    } catch {
      // Invalid base64
    }
    return null
  }

  const normalizeImageUrl = (value: unknown): string | null => {
    if (!value) return null

    const valueStr = String(value)
    const trimmed = valueStr.trim()

    if (trimmed === '-' || trimmed === 'undefined' || trimmed === 'null' || trimmed === '') {
      return null
    }

    if (trimmed.startsWith('data:image/')) {
      return trimmed
    }

    if (trimmed.startsWith('data:base64,')) {
      const base64Data = trimmed.slice(12)
      const formatInfo = detectImageFormat(base64Data) || { mime: 'image/png' }
      return `data:${formatInfo.mime};base64,${base64Data}`
    }

    if (isPureBase64(trimmed)) {
      const formatInfo = detectImageFormat(trimmed) || { mime: 'image/png' }
      return `data:${formatInfo.mime};base64,${trimmed}`
    }

    return trimmed
  }

  const extractImageData = (data: DataCollected) => {
    const values = data.values
    if (!values) return null

    if (Array.isArray(values)) {
      for (const item of values) {
        if (typeof item === 'object' && item !== null) {
          const obj = item as Record<string, unknown>
          for (const key of ['image_base64', 'imageBase64', 'base64', 'image_url', 'imageUrl', 'url', 'image', 'src']) {
            const normalized = normalizeImageUrl(obj[key])
            if (normalized) {
              return { src: normalized, mimeType: obj.image_mime_type || obj.mimeType }
            }
          }
        }
      }
      for (const item of values) {
        if (typeof item === 'string') {
          const normalized = normalizeImageUrl(item)
          if (normalized) return { src: normalized, mimeType: null }
        }
      }
    }

    if (typeof values === 'object') {
      const obj = values as Record<string, unknown>
      for (const key of ['image_base64', 'imageBase64', 'base64', 'image_url', 'imageUrl', 'url', 'image', 'src', 'value']) {
        const normalized = normalizeImageUrl(obj[key])
        if (normalized) {
          return { src: normalized, mimeType: obj.image_mime_type || obj.mimeType }
        }
      }
    }

    if (typeof values === 'string') {
      const normalized = normalizeImageUrl(values)
      if (normalized) return { src: normalized, mimeType: null }
    }

    return null
  }

  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) return '-'
    if (typeof value === 'string') return value
    if (typeof value === 'number') return String(value)
    if (typeof value === 'boolean') return value ? 'true' : 'false'
    if (Array.isArray(value)) return `[${value.length} items]`
    if (typeof value === 'object') {
      const str = JSON.stringify(value)
      return str.length > 100 ? str.slice(0, 100) + '...' : str
    }
    return String(value)
  }

  const getDataDisplayPairs = (data: DataCollected) => {
    const values = data.values
    const pairs: { key: string; value: string }[] = []

    if (!values) return pairs

    if (Array.isArray(values)) {
      values.forEach((item, idx) => {
        if (typeof item !== 'object' || item === null) {
          pairs.push({ key: `[${idx}]`, value: formatValue(item) })
        } else {
          const obj = item as Record<string, unknown>
          for (const [k, v] of Object.entries(obj)) {
            if (!['image_base64', 'imageBase64', 'base64', 'image_url', 'imageUrl', 'url', 'image', 'src'].includes(k)) {
              pairs.push({ key: k, value: formatValue(v) })
            }
          }
        }
      })
    } else if (typeof values === 'object') {
      const obj = values as Record<string, unknown>
      for (const [k, v] of Object.entries(obj)) {
        if (!['image_base64', 'imageBase64', 'base64', 'image_url', 'imageUrl', 'url', 'image', 'src'].includes(k)) {
          pairs.push({ key: k, value: formatValue(v) })
        }
      }
    } else {
      pairs.push({ key: 'value', value: formatValue(values) })
    }

    return pairs
  }

  const ExecutionContent = () => {
    if (loading) {
      return (
        <div className="h-full flex items-center justify-center">
          <Clock className="h-5 w-5 animate-spin text-muted-foreground" />
        </div>
      )
    }

    if (!execution) {
      return (
        <div className="text-center py-12 text-muted-foreground">
          {t('agents:executionNotFound', { defaultValue: 'Execution not found' })}
        </div>
      )
    }

    return (
      <div className="space-y-3">
        {/* Status Bar */}
        <div className="flex items-center justify-between py-2 px-3 bg-muted/30 rounded-lg">
          <div className="flex items-center gap-2">
            {getStatusIcon(execution.status)}
            <span className="text-xs font-medium">{t(`agents:executionStatus.${execution.status.toLowerCase()}`)}</span>
          </div>
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Clock className="h-4 w-4" />
              {formatTimestamp(execution.timestamp, false)}
            </span>
            <span>{execution.duration_ms}ms</span>
          </div>
        </div>

        {execution.error && (
          <Card className="p-2 border-destructive/50 bg-destructive/5">
            <div className="flex items-start gap-1.5 text-destructive">
              <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
              <div className="text-xs break-words">{execution.error}</div>
            </div>
          </Card>
        )}

        {execution.decision_process && (
          <FormSectionGroup>
            {/* Situation Analysis */}
            <FormSection
              title={t('agents:execution.analysis', { defaultValue: 'Situation Analysis' })}
              
              collapsible
            >
              <p className="text-xs leading-relaxed">{execution.decision_process.situation_analysis}</p>
            </FormSection>

            {/* Input Data */}
            {execution.decision_process.data_collected.length > 0 && (
              <FormSection
                title={`${t('agents:execution.inputData', { defaultValue: 'Input Data' })} (${execution.decision_process.data_collected.length})`}
                
                collapsible
              >
                <div className="space-y-2">
                  {execution.decision_process.data_collected.map((data, idx) => {
                    const imageData = extractImageData(data)
                    const hasImage = imageData !== null
                    const dataPairs = getDataDisplayPairs(data)
                    const isExpanded = expandedDataIndices.has(idx)

                    return (
                      <div key={idx} className="border rounded-lg overflow-hidden">
                        <div
                          className="flex items-center justify-between p-2 bg-[var(--bg-50)] cursor-pointer hover:bg-[var(--bg-80)] transition-colors"
                          onClick={() => toggleDataExpanded(idx)}
                        >
                          <div className="flex items-center gap-2 min-w-0 flex-1">
                            {hasImage && <ImageIcon className="h-4 w-4 text-purple-500 shrink-0" />}
                            <span className="text-[10px] font-medium truncate">{data.source}</span>
                            <Badge variant="outline" className="text-[9px] h-4 px-1 shrink-0">{data.data_type}</Badge>
                          </div>
                          {dataPairs.length > 0 && (
                            isExpanded ? (
                              <ChevronUp className="h-4 w-4 text-muted-foreground shrink-0" />
                            ) : (
                              <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
                            )
                          )}
                        </div>

                        {hasImage && (
                          <div className="p-2 bg-black/5">
                            <img
                              src={imageData!.src}
                              alt={`${data.source} - ${t('agents:execution.inputImage', { defaultValue: 'Input Image' })}`}
                              className="w-full max-h-[200px] object-contain rounded-md bg-background"
                            />
                          </div>
                        )}

                        {isExpanded && dataPairs.length > 0 && (
                          <div className="p-2 border-t bg-background">
                            <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-[10px]">
                              {dataPairs.slice(0, 10).map((pair, pairIdx) => (
                                <div key={pairIdx} className="flex items-baseline gap-1 min-w-0">
                                  <span className="text-muted-foreground shrink-0">{pair.key}:</span>
                                  <span className="truncate font-mono">{pair.value}</span>
                                </div>
                              ))}
                              {dataPairs.length > 10 && (
                                <div className="col-span-2 text-muted-foreground text-[9px]">
                                  {t('agents:execution.moreFields', { count: dataPairs.length - 10, defaultValue: `+${dataPairs.length - 10} more fields` })}
                                </div>
                              )}
                            </div>
                          </div>
                        )}
                      </div>
                    )
                  })}
                </div>
              </FormSection>
            )}

            {/* Reasoning Steps */}
            {execution.decision_process.reasoning_steps.length > 0 && (
              <FormSection
                title={t('agents:execution.reasoningSteps', { defaultValue: 'Reasoning Steps' })}
                
                collapsible
              >
                <div className="space-y-2">
                  {execution.decision_process.reasoning_steps.map((step, idx, arr) => (
                    <div key={idx} className="flex gap-2">
                      <div className="flex flex-col items-center">
                        <div className="w-5 h-5 rounded-full bg-primary/10 text-primary text-[10px] flex items-center justify-center shrink-0">
                          {step.step_number}
                        </div>
                        {idx < arr.length - 1 && (
                          <div className="w-0.5 flex-1 bg-border my-0.5" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="text-xs font-medium">{step.description}</div>
                        <div className="flex items-center gap-2 mt-1">
                          <Badge variant="outline" className="text-[9px] h-4 px-1">{step.step_type}</Badge>
                          <span className="text-[10px] text-muted-foreground">
                            {Math.round(step.confidence * 100)}%
                          </span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </FormSection>
            )}

            {/* Decisions */}
            {execution.decision_process.decisions.length > 0 && (
              <FormSection
                title={t('agents:execution.decisions', { defaultValue: 'Decisions' })}
                
                collapsible
              >
                <div className="space-y-1.5">
                  {execution.decision_process.decisions.map((decision, idx) => (
                    <div key={idx} className="p-2 bg-background rounded border">
                      <div className="text-xs font-medium mb-1">{decision.description}</div>
                      <div className="flex items-center justify-between">
                        <span className="text-[10px] text-muted-foreground truncate flex-1 mr-2">{decision.rationale}</span>
                        <Badge variant="secondary" className="text-[9px] h-4 px-1 shrink-0">{decision.action}</Badge>
                      </div>
                    </div>
                  ))}
                </div>
              </FormSection>
            )}

            {/* Conclusion */}
            <Card className="p-3 bg-primary/5 border-primary/20">
              <div className="text-xs font-semibold text-primary mb-2">{t('agents:execution.conclusionLabel', { defaultValue: 'Conclusion' })}</div>
              <MarkdownMessage content={execution.decision_process.conclusion} />
            </Card>

            {/* LLM Final Response */}
            {execution.result?.summary && (() => {
              const summary = execution.result.summary.trim()
              const conclusion = execution.decision_process?.conclusion?.trim() ?? ''
              const isGeneric = summary === 'Completed tool execution rounds.'
                || summary === 'LLM generation failed during tool execution.'
              const normalize = (s: string) => s.replace(/\s+/g, ' ').trim()
              const isDuplicate = normalize(summary) === normalize(conclusion)
                || (conclusion.length > 100 && normalize(summary).includes(normalize(conclusion).slice(0, 200)))
                || (summary.length > 100 && normalize(conclusion).includes(normalize(summary).slice(0, 200)))
              if (!summary || isGeneric || isDuplicate) return null
              return (
                <FormSection
                  title={t('agents:memory.llmResponse', 'LLM Response')}
                  collapsible
                >
                  <pre className="text-xs whitespace-pre-wrap font-mono bg-muted/50 p-3 rounded-lg border max-h-60 overflow-auto break-words leading-relaxed">
                    {summary}
                  </pre>
                </FormSection>
              )
            })()}

            {/* Actions Executed */}
            {execution.result?.actions_executed && execution.result.actions_executed.length > 0 && (
              <FormSection
                title={t('agents:execution.actionsExecuted', { defaultValue: 'Actions Executed' })}
                
                collapsible
              >
                <div className="space-y-1">
                  {execution.result.actions_executed.map((action, idx) => (
                    <div key={idx} className="flex items-center justify-between p-1.5 bg-background rounded border">
                      <div className="flex-1 min-w-0 mr-2">
                        <div className="text-xs truncate">{action.description}</div>
                        <div className="text-[10px] text-muted-foreground truncate">{action.target}</div>
                      </div>
                      <Badge variant={action.success ? "default" : "destructive"} className="text-[9px] h-4 px-1 shrink-0">
                        {action.success ? '✓' : '✗'}
                      </Badge>
                    </div>
                  ))}
                </div>
              </FormSection>
            )}
          </FormSectionGroup>
        )}
      </div>
    )
  }

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-[100] bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <Sparkles className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">
                    {t('agents:execution.title', { defaultValue: 'Execution' })} #{executionId.slice(-6)}
                  </h1>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <ExecutionContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button onClick={handleClose} className="min-w-[80px]">
                {t('common:close')}
              </Button>
            </div>
          </div>
        </div>
      ) : null,
      document.body
    )
  }

  // Desktop: Traditional dialog
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={handleClose}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-50',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[calc(100vh-2rem)] sm:max-h-[85vh]',
            'flex flex-col',
            'max-w-2xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <Sparkles className="h-5 w-5 text-primary" />
              <h2 className="text-lg font-semibold leading-none truncate">
                {t('agents:execution.title', { defaultValue: 'Execution' })} #{executionId.slice(-6)}
              </h2>
            </div>
            <button
              onClick={handleClose}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <ExecutionContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted/30">
            <Button size="sm" onClick={handleClose}>
              {t('common:close')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
