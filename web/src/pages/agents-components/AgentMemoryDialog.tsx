import { useState, useEffect, useCallback } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { Brain, TrendingUp, Trash2, RefreshCw, X, Loader2 } from "lucide-react"
import { api } from "@/lib/api"
import { formatTimestamp } from "@/lib/utils/format"
import type { AgentMemory } from "@/types"
import { confirm } from "@/hooks/use-confirm"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"

interface AgentMemoryDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  agentId: string
  agentName: string
}

export function AgentMemoryDialog({
  open,
  onOpenChange,
  agentId,
  agentName,
}: AgentMemoryDialogProps) {
  const { t } = useTranslation(['common', 'agents'])
  const { handleError } = useErrorHandler()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [memory, setMemory] = useState<AgentMemory | null>(null)
  const [loading, setLoading] = useState(false)
  const [clearing, setClearing] = useState(false)

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    if (open && agentId) {
      loadMemory()
    }
  }, [open, agentId])

  const loadMemory = async () => {
    setLoading(true)
    try {
      const data = await api.getAgentMemory(agentId)
      setMemory(data)
    } catch (error) {
      handleError(error, { operation: 'Load agent memory', showToast: false })
    } finally {
      setLoading(false)
    }
  }

  const handleClearMemory = async () => {
    const confirmed = await confirm({
      title: t('agents:memory.clear'),
      description: t('agents:clearMemoryConfirm'),
      confirmText: t('common:confirm'),
      cancelText: t('common:cancel'),
      variant: "destructive"
    })
    if (!confirmed) return

    setClearing(true)
    try {
      await api.clearAgentMemory(agentId)
      await loadMemory()
    } catch (error) {
      handleError(error, { operation: 'Clear memory', showToast: false })
    } finally {
      setClearing(false)
    }
  }

  const handleClose = useCallback(() => {
    if (!clearing) {
      onOpenChange(false)
    }
  }, [clearing, onOpenChange])

  const isBusy = loading || clearing

  const MemoryContent = () => {
    if (loading && !memory) {
      return (
        <div className="flex items-center justify-center py-8">
          <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )
    }

    if (!memory) {
      return (
        <div className="text-center py-8 text-muted-foreground">
          {t('agents:memory.noMemory')}
        </div>
      )
    }

    return (
      <FormSectionGroup>
        {/* State Variables */}
        {Object.keys(memory.state_variables).length > 0 && (
          <FormSection
            title={t('agents:memory.stateVariables')}
            
            
          >
            <div className="grid grid-cols-2 gap-4">
              {Object.entries(memory.state_variables).map(([key, value]) => (
                <div key={key} className="space-y-1">
                  <div className="text-xs text-muted-foreground">{key}</div>
                  <div className="text-sm font-mono bg-muted px-2 py-1 rounded truncate">
                    {String(value)}
                  </div>
                </div>
              ))}
            </div>
          </FormSection>
        )}

        {/* Baselines */}
        {memory.baselines && Object.keys(memory.baselines).length > 0 && (
          <FormSection
            title={t('agents:memory.baselines')}
            
          >
            <div className="space-y-2">
              {Object.entries(memory.baselines).map(([metric, value]) => (
                <div key={metric} className="flex justify-between items-center">
                  <span className="text-sm">{metric}</span>
                  <Badge variant="secondary">{value.toFixed(2)}</Badge>
                </div>
              ))}
            </div>
          </FormSection>
        )}

        {/* Learned Patterns */}
        {memory.learned_patterns.length > 0 && (
          <FormSection
            title={t('agents:memory.learnedPatterns')}
            
          >
            <div className="space-y-2">
              {memory.learned_patterns.map((pattern, idx) => (
                <Card key={idx} className="p-3">
                  <div className="font-medium text-sm">{pattern.description}</div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {t('agents:memory.confidence', { defaultValue: 'Confidence' })}: {(pattern.confidence * 100).toFixed(0)}%
                  </div>
                </Card>
              ))}
            </div>
          </FormSection>
        )}

        {/* Trend Data Summary */}
        {memory.trend_data.length > 0 && (
          <FormSection
            title={`${t('agents:memory.trendData')} (${memory.trend_data.length} ${t('common:points')})`}
            
          >
            <div className="text-sm text-muted-foreground">
              {t('agents:memory.trendDataHint', { count: memory.trend_data.length })}
            </div>
          </FormSection>
        )}

        {/* Updated At */}
        <div className="text-xs text-muted-foreground pt-2">
          {t('agents:memory.updatedAt')}: {formatTimestamp(memory.updated_at)}
        </div>
      </FormSectionGroup>
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
                <Brain className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('agents:memory.title')}</h1>
                  <p className="text-xs text-muted-foreground truncate">{agentName}</p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={clearing} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <MemoryContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={clearing} className="min-w-[80px]">
                {t('common:close')}
              </Button>
              <Button
                variant="destructive"
                onClick={handleClearMemory}
                disabled={isBusy || !memory || Object.keys(memory?.state_variables || {}).length === 0}
                className="min-w-[80px]"
              >
                {clearing ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Trash2 className="h-4 w-4 mr-2" />
                )}
                {t('agents:memory.clear')}
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
            'max-h-[calc(100vh-2rem)] sm:max-h-[80vh]',
            'flex flex-col',
            'max-w-2xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <Brain className="h-5 w-5 text-primary" />
                <h2 className="text-lg font-semibold leading-none truncate">
                  {t('agents:memory.title')} - {agentName}
                </h2>
              </div>
              <p className="text-sm text-muted-foreground">
                {t('agents:memory.description')}
              </p>
            </div>
            <button
              onClick={handleClose}
              disabled={clearing}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <MemoryContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t shrink-0 bg-[var(--muted-30)]">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={clearing}>
              {t('common:close')}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleClearMemory}
              disabled={isBusy || !memory || Object.keys(memory?.state_variables || {}).length === 0}
            >
              {clearing ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4 mr-2" />
              )}
              {t('agents:memory.clear')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
