import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { Brain, Trash2, Loader2 } from "lucide-react"
import { api } from "@/lib/api"
import { formatTimestamp } from "@/lib/utils/format"
import type { AgentMemory } from "@/types"
import { confirm } from "@/hooks/use-confirm"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { UnifiedFormDialog } from "@/components/dialog/UnifiedFormDialog"

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

  const [memory, setMemory] = useState<AgentMemory | null>(null)
  const [loading, setLoading] = useState(false)
  const [clearing, setClearing] = useState(false)

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

  const isBusy = loading || clearing

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('agents:memory.title')}
      description={agentName}
      icon={<Brain className="h-5 w-5 text-primary" />}
      width="xl"
      loading={loading && !memory}
      preventCloseOnSubmit={false}
      hideFooter
      footer={
        <>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={clearing}>
            {t('common:close')}
          </Button>
          <Button
            variant="destructive"
            onClick={handleClearMemory}
            disabled={isBusy || !memory || Object.keys(memory?.state_variables || {}).length === 0}
          >
            {clearing ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Trash2 className="h-4 w-4 mr-2" />}
            {t('agents:memory.clear')}
          </Button>
        </>
      }
    >
      {!memory ? (
        <div className="text-center py-8 text-muted-foreground">
          {t('agents:memory.noMemory')}
        </div>
      ) : (
        <FormSectionGroup>
          {/* State Variables */}
          {Object.keys(memory.state_variables).length > 0 && (
            <FormSection
              title={t('agents:memory.stateVariables')}
            >
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
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
      )}
    </UnifiedFormDialog>
  )
}
