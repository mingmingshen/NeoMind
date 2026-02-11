import { useState, useCallback, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Card } from "@/components/ui/card"
import {
  Clock,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Brain,
  Database,
  Play,
  FileText,
  ChevronDown,
  ChevronRight,
  Loader2,
  Zap,
  Bell,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils/format"
import { api } from "@/lib/api"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import type { AgentExecution, AgentExecutionDetail, DataCollected, ReasoningStep, Decision } from "@/types"

interface AgentExecutionTimelineProps {
  executions: AgentExecution[]
  loading: boolean
  agentId: string
  onViewExecutionDetail?: (agentId: string, executionId: string) => void
}

export function AgentExecutionTimeline({
  executions,
  loading,
  agentId,
  onViewExecutionDetail,
}: AgentExecutionTimelineProps) {
  const { t } = useTranslation(['common', 'agents'])
  const { handleError } = useErrorHandler()
  const [expandedExecutions, setExpandedExecutions] = useState<Set<string>>(new Set())
  const [executionDetails, setExecutionDetails] = useState<Record<string, AgentExecutionDetail>>({})
  const [loadingDetails, setLoadingDetails] = useState<Set<string>>(new Set())

  const toggleExecution = async (executionId: string) => {
    const newExpanded = new Set(expandedExecutions)
    const isExpanding = !newExpanded.has(executionId)

    if (isExpanding) {
      newExpanded.add(executionId)
      // Load details if not already loaded
      if (!executionDetails[executionId]) {
        await loadExecutionDetail(executionId)
      }
    } else {
      newExpanded.delete(executionId)
    }
    setExpandedExecutions(newExpanded)
  }

  const loadExecutionDetail = async (executionId: string) => {
    setLoadingDetails(prev => new Set(prev).add(executionId))
    try {
      const data = await api.getExecution(agentId, executionId)
      setExecutionDetails(prev => ({ ...prev, [executionId]: data }))
    } catch (error) {
      handleError(error, { operation: 'Load execution detail', showToast: false })
    } finally {
      setLoadingDetails(prev => {
        const next = new Set(prev)
        next.delete(executionId)
        return next
      })
    }
  }

  const getStatusConfig = (status: string) => {
    switch (status) {
      case 'Running':
        return { icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-500/10 border-blue-500/20', label: t('agents:executionStatus.running') }
      case 'Completed':
        return { icon: CheckCircle2, color: 'text-green-500', bg: 'bg-green-500/10 border-green-500/20', label: t('agents:executionStatus.completed') }
      case 'Failed':
        return { icon: XCircle, color: 'text-red-500', bg: 'bg-red-500/10 border-red-500/20', label: t('agents:executionStatus.failed') }
      case 'Cancelled':
        return { icon: XCircle, color: 'text-gray-500', bg: 'bg-gray-500/10 border-gray-500/20', label: t('agents:executionStatus.cancelled') }
      default:
        return { icon: AlertCircle, color: 'text-gray-500', bg: 'bg-gray-500/10 border-gray-500/20', label: status }
    }
  }

  // Format duration
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    return `${(ms / 1000).toFixed(2)}s`
  }

  return (
    <div className="h-full flex flex-col">
      <ScrollArea className="flex-1">
        <div className="p-4">
          {loading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : executions.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <Clock className="h-12 w-12 mx-auto mb-3 opacity-20" />
              <p>{t('agents:noExecutions')}</p>
            </div>
          ) : (
            <div className="relative">
              {/* Timeline Line - aligned to center of dots (left-[16px] = 8px position + 8px half of 16px dot) */}
              <div className="absolute left-[16px] top-2 bottom-2 w-0.5 bg-border" />

              {/* Timeline Items */}
              <div className="space-y-4">
                {executions.map((execution, index) => {
                  const isExpanded = expandedExecutions.has(execution.id)
                  const detail = executionDetails[execution.id]
                  const isLoadingDetail = loadingDetails.has(execution.id)
                  const statusConfig = getStatusConfig(execution.status)
                  const StatusIcon = statusConfig.icon

                  return (
                    <div key={execution.id} className="relative pl-12">
                      {/* Timeline Node - position at left-2 (8px) with w-4 (16px) so center is at 16px */}
                      <div className={cn(
                        "absolute left-2 top-3 w-4 h-4 rounded-full border-2 flex items-center justify-center bg-background",
                        statusConfig.bg.replace('/10', '/30'),
                        statusConfig.color.replace('text-', 'border-')
                      )}>
                        <div className={cn("w-2 h-2 rounded-full", statusConfig.color.replace('text-', 'bg-'))} />
                      </div>

                      {/* Timeline Card */}
                      <div
                        className={cn(
                          "border rounded-lg overflow-hidden transition-all",
                          isExpanded && statusConfig.bg,
                          !isExpanded && "hover:bg-muted/30"
                        )}
                      >
                        {/* Header - Always Visible */}
                        <button
                          type="button"
                          onClick={() => void toggleExecution(execution.id)}
                          className="w-full p-3 flex items-start gap-3 text-left"
                        >
                          <StatusIcon className={cn("h-5 w-5 mt-0.5 shrink-0", execution.status === 'Running' && "animate-spin")} />
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 flex-wrap mb-1">
                              <Badge variant="outline" className="text-xs">
                                #{executions.length - index}
                              </Badge>
                              <Badge className={cn("text-xs", statusConfig.bg, statusConfig.color)}>
                                {statusConfig.label}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-3 text-sm text-muted-foreground">
                              <span className="flex items-center gap-1">
                                <Clock className="h-3.5 w-3.5" />
                                {formatTimestamp(execution.timestamp, false)}
                              </span>
                              {execution.duration_ms > 0 && (
                                <span className="flex items-center gap-1">
                                  <Zap className="h-3.5 w-3.5" />
                                  {formatDuration(execution.duration_ms)}
                                </span>
                              )}
                              {execution.error && (
                                <span className="flex items-center gap-1 text-destructive">
                                  <AlertCircle className="h-3.5 w-3.5" />
                                  <span className="truncate max-w-[200px]">{execution.error}</span>
                                </span>
                              )}
                            </div>
                          </div>
                          <div className="shrink-0 mt-1">
                            {isExpanded ? (
                              <ChevronDown className="h-4 w-4 text-muted-foreground" />
                            ) : (
                              <ChevronRight className="h-4 w-4 text-muted-foreground" />
                            )}
                          </div>
                        </button>

                        {/* Expanded Details */}
                        {isExpanded && (
                          <div className="border-t p-4 space-y-4">
                            {isLoadingDetail ? (
                              <div className="flex items-center justify-center py-8">
                                <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                              </div>
                            ) : detail ? (
                              <>
                                {/* Situation Analysis */}
                                {detail.decision_process?.situation_analysis && (
                                  <TimelineSection
                                    icon={<Brain className="h-4 w-4 text-purple-500" />}
                                    title={t('agents:memory.situationAnalysis')}
                                  >
                                    <p className="text-sm">{detail.decision_process.situation_analysis}</p>
                                  </TimelineSection>
                                  )}

                                {/* Data Collected */}
                                {detail.decision_process?.data_collected && detail.decision_process.data_collected.length > 0 && (
                                  <TimelineSection
                                    icon={<Database className="h-4 w-4 text-blue-500" />}
                                    title={t('agents:memory.dataCollected')}
                                    subtitle={`${detail.decision_process.data_collected.filter(d => d.data_type !== 'device_info').length} ${t('agents:memory.sources')}`}
                                  >
                                    <div className="grid grid-cols-1 gap-2">
                                      {detail.decision_process.data_collected
                                        .filter(data => data.data_type !== 'device_info')
                                        .map((data, idx) => (
                                        <DataCollectedItem key={idx} data={data} />
                                      ))}
                                    </div>
                                  </TimelineSection>
                                )}

                                {/* Reasoning Steps */}
                                {detail.decision_process?.reasoning_steps && detail.decision_process.reasoning_steps.length > 0 && (
                                  <TimelineSection
                                    icon={<ChevronRight className="h-4 w-4 text-orange-500" />}
                                    title={t('agents:memory.reasoningSteps')}
                                  >
                                    <div className="space-y-2">
                                      {detail.decision_process.reasoning_steps.map((step, idx) => (
                                        <ReasoningStepItem key={idx} step={step} />
                                      ))}
                                    </div>
                                  </TimelineSection>
                                )}

                                {/* Decisions */}
                                {detail.decision_process?.decisions && detail.decision_process.decisions.length > 0 && (
                                  <TimelineSection
                                    icon={<Play className="h-4 w-4 text-green-500" />}
                                    title={t('agents:memory.decisions')}
                                  >
                                    <div className="grid grid-cols-1 gap-2">
                                      {detail.decision_process.decisions.map((decision, idx) => (
                                        <DecisionItem key={idx} decision={decision} />
                                      ))}
                                    </div>
                                  </TimelineSection>
                                )}

                                {/* Confidence */}
                                {detail.decision_process?.confidence !== undefined && (
                                  <div className="flex items-center justify-between text-sm p-3 bg-muted/50 rounded-lg">
                                    <span className="text-muted-foreground">{t('agents:memory.confidence')}</span>
                                    <Badge variant={detail.decision_process.confidence > 0.7 ? "default" : "secondary"}>
                                      {(detail.decision_process.confidence * 100).toFixed(0)}%
                                    </Badge>
                                  </div>
                                )}

                                {/* Conclusion */}
                                {detail.decision_process?.conclusion && (
                                  <Card className="p-3 bg-muted/50">
                                    <div className="text-sm">
                                      <span className="font-medium">{t('agents:memory.conclusion')}:</span>
                                      <span className="ml-2">{detail.decision_process.conclusion}</span>
                                    </div>
                                  </Card>
                                )}

                                {/* Report */}
                                {detail.result?.report && (
                                  <TimelineSection
                                    icon={<FileText className="h-4 w-4 text-gray-500" />}
                                    title={t('agents:memory.generatedReport')}
                                  >
                                    <Card className="p-3">
                                      <pre className="text-sm whitespace-pre-wrap font-mono text-xs overflow-x-auto max-h-60">
                                        {detail.result.report}
                                      </pre>
                                    </Card>
                                  </TimelineSection>
                                )}

                                {/* Actions Executed */}
                                {detail.result?.actions_executed && detail.result.actions_executed.length > 0 && (
                                  <TimelineSection
                                    icon={<Zap className="h-4 w-4 text-yellow-500" />}
                                    title={t('agents:memory.actionsExecuted')}
                                  >
                                    <div className="space-y-2">
                                      {detail.result.actions_executed.map((action, idx) => (
                                        <Card key={idx} className="p-3 min-w-0">
                                          <div className="flex items-start justify-between gap-3 mb-2">
                                            <div className="text-sm flex-1 min-w-0">
                                              <div className="font-medium truncate" title={action.description}>
                                                {action.description}
                                              </div>
                                              <div className="text-xs text-muted-foreground truncate" title={action.target}>
                                                {action.target}
                                              </div>
                                            </div>
                                            <Badge variant={action.success ? "default" : "destructive"} className="shrink-0">
                                              {action.success ? t('common:success') : t('common:failed')}
                                            </Badge>
                                          </div>
                                          {/* Parameters */}
                                          {action.parameters && Object.keys(action.parameters).length > 0 && (
                                            <div className="mt-2 pt-2 border-t">
                                              <div className="text-xs text-muted-foreground mb-1">
                                                {t('agents:memory.parameters')}:
                                              </div>
                                              <pre className="text-xs bg-muted p-2 rounded overflow-x-auto max-h-20 w-full break-all">
                                                {JSON.stringify(action.parameters, null, 2)}
                                              </pre>
                                            </div>
                                          )}
                                          {/* Result */}
                                          {action.result && (
                                            <div className="mt-2 pt-2 border-t">
                                              <div className="text-xs text-muted-foreground mb-1">
                                                {t('agents:memory.result')}:
                                              </div>
                                              <div className="text-xs bg-muted p-2 rounded max-h-20 overflow-auto break-words">
                                                {action.result}
                                              </div>
                                            </div>
                                          )}
                                        </Card>
                                      ))}
                                    </div>
                                  </TimelineSection>
                                )}

                                {/* Notifications Sent */}
                                {detail.result?.notifications_sent && detail.result.notifications_sent.length > 0 && (
                                  <TimelineSection
                                    icon={<Bell className="h-4 w-4 text-blue-500" />}
                                    title={t('agents:memory.notificationsSent')}
                                  >
                                    <div className="space-y-2">
                                      {detail.result.notifications_sent.map((notification, idx) => (
                                        <Card key={idx} className="p-3">
                                          <div className="flex items-start justify-between gap-3">
                                            <div className="text-sm flex-1 min-w-0">
                                              <div className="flex items-center gap-2 mb-1">
                                                <span className="font-medium">{notification.channel}</span>
                                                <span className="text-xs text-muted-foreground">→</span>
                                                <span className="text-xs">{notification.recipient}</span>
                                              </div>
                                              <div className="text-xs text-muted-foreground mb-2" title={notification.message}>
                                                {notification.message}
                                              </div>
                                              {notification.sent_at && (
                                                <div className="text-xs text-muted-foreground flex items-center gap-1">
                                                  <Clock className="h-3 w-3" />
                                                  {formatTimestamp(notification.sent_at, false)}
                                                </div>
                                              )}
                                            </div>
                                            <Badge variant={notification.success ? "default" : "destructive"} className="shrink-0">
                                              {notification.success ? t('common:sent') : t('common:failed')}
                                            </Badge>
                                          </div>
                                        </Card>
                                      ))}
                                    </div>
                                  </TimelineSection>
                                )}
                              </>
                            ) : (
                              <div className="text-center py-4 text-muted-foreground text-sm">
                                {t('agents:noExecutions')}
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}

// ============================================================================
// Sub Components
// ============================================================================

interface TimelineSectionProps {
  icon: React.ReactNode
  title: string
  subtitle?: string
  children: React.ReactNode
}

function TimelineSection({ icon, title, subtitle, children }: TimelineSectionProps) {
  return (
    <div>
      <div className="flex items-center gap-2 mb-2">
        {icon}
        <h4 className="text-sm font-semibold">{title}</h4>
        {subtitle && (
          <span className="text-xs text-muted-foreground">({subtitle})</span>
        )}
      </div>
      {children}
    </div>
  )
}

function DataCollectedItem({ data }: { data: DataCollected }) {
  const { t } = useTranslation(['common', 'agents'])
  return (
    <Card className="p-2 min-w-0">
      <div className="flex items-center justify-between mb-1 gap-2">
        <span className="text-xs font-medium truncate flex-1 min-w-0" title={data.source}>{data.source}</span>
        <Badge variant="outline" className="text-xs h-5 shrink-0">{data.data_type}</Badge>
      </div>
      <pre className="text-xs bg-muted p-1.5 rounded overflow-x-auto max-h-24 w-full min-w-0 break-all">
        {typeof data.values === 'object'
          ? JSON.stringify(data.values, null, 2)
          : String(data.values)}
      </pre>
    </Card>
  )
}

function ReasoningStepItem({ step }: { step: ReasoningStep }) {
  const { t } = useTranslation(['common', 'agents'])
  return (
    <div className="flex gap-3 min-w-0">
      <div className="flex flex-col items-center shrink-0">
        <div className="w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs flex items-center justify-center">
          {step.step_number}
        </div>
        {step.step_number < 10 && <div className="w-0.5 flex-1 bg-border min-h-[24px]" />}
      </div>
      <div className="flex-1 min-w-0 pb-4">
        <div className="text-sm break-words">{step.description}</div>
        {step.input && (
          <div className="text-xs text-muted-foreground mt-1 break-words">
            {t('agents:memory.input')}: {step.input}
          </div>
        )}
        {step.output && (
          <div className="text-xs bg-muted p-2 rounded mt-2 break-words">
            {t('agents:memory.output')}: {step.output}
          </div>
        )}
        <div className="flex items-center gap-2 mt-2 flex-wrap">
          <Badge variant="outline" className="text-xs h-5">{step.step_type}</Badge>
          <span className="text-xs text-muted-foreground">
            {t('agents:memory.confidence')}: {(step.confidence * 100).toFixed(0)}%
          </span>
        </div>
      </div>
    </div>
  )
}

function DecisionItem({ decision }: { decision: Decision }) {
  const { t } = useTranslation(['common', 'agents'])
  return (
    <Card className="p-2 min-w-0">
      <div className="text-sm font-medium mb-1 break-words">{decision.description}</div>
      {decision.rationale && (
        <div className="text-xs text-muted-foreground mb-2 break-words">{decision.rationale}</div>
      )}
      <div className="flex items-center justify-between text-xs gap-2">
        <span className="text-muted-foreground shrink-0">{t('agents:memory.action')}</span>
        <Badge variant="secondary" className="h-5 truncate max-w-[150px]">{decision.action}</Badge>
      </div>
    </Card>
  )
}
