import { useState, useCallback, useEffect, useMemo } from "react"
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
  ChevronUp,
  Wrench,
  Sparkles,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { textNano, textMini } from "@/design-system/tokens/typography"
import { formatTimestamp } from "@/lib/utils/format"
import { api } from "@/lib/api"
import { MarkdownMessage } from "@/components/chat/MarkdownMessage"
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
        return { icon: Loader2, color: 'text-info', bg: 'bg-info-light border-info', label: t('agents:executionStatus.running') }
      case 'Completed':
        return { icon: CheckCircle2, color: 'text-success', bg: 'bg-success-light border-success-light', label: t('agents:executionStatus.completed') }
      case 'Failed':
        return { icon: XCircle, color: 'text-error', bg: 'bg-error-light border-error', label: t('agents:executionStatus.failed') }
      case 'Cancelled':
        return { icon: XCircle, color: 'text-muted-foreground', bg: 'bg-muted border-border', label: t('agents:executionStatus.cancelled') }
      default:
        return { icon: AlertCircle, color: 'text-muted-foreground', bg: 'bg-muted border-border', label: status }
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
                          !isExpanded && "hover:bg-muted-30"
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
                                <Clock className="h-4 w-4" />
                                {formatTimestamp(execution.timestamp, false)}
                              </span>
                              {execution.duration_ms > 0 && (
                                <span className="flex items-center gap-1">
                                  <Zap className="h-4 w-4" />
                                  {formatDuration(execution.duration_ms)}
                                </span>
                              )}
                              {execution.error && (
                                <span className="flex items-center gap-1 text-destructive">
                                  <AlertCircle className="h-4 w-4" />
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
                                {/* ① Situation Analysis */}
                                {detail.decision_process?.situation_analysis && (
                                  <TimelineSection
                                    icon={<Brain className="h-4 w-4 text-accent-purple" />}
                                    title={t('agents:memory.situationAnalysis')}
                                  >
                                    <CollapsibleText content={detail.decision_process.situation_analysis} maxLines={3} />
                                  </TimelineSection>
                                )}

                                {/* ② Execution Process — reasoning_steps with tool_call cards */}
                                {detail.decision_process?.reasoning_steps && detail.decision_process.reasoning_steps.length > 0 && (
                                  <TimelineSection
                                    icon={<ChevronRight className="h-4 w-4 text-accent-orange" />}
                                    title={t('agents:memory.executionProcess')}
                                  >
                                    <div className="space-y-2">
                                      {detail.decision_process.reasoning_steps.map((step, idx, steps) => {
                                        // Detect round boundaries
                                        const prevStep = idx > 0 ? steps[idx - 1] : null;
                                        const isNewRound = step.step_type === 'thought' &&
                                          (prevStep?.step_type === 'tool_call' || prevStep?.step_type === 'error' || idx === 0);
                                        const roundNumber = steps.slice(0, idx + 1).filter(s => s.step_type === 'thought').length;

                                        // Use ToolCallStep for tool-related types only
                                        // llm_analysis, data_collection, condition_eval etc. use ReasoningStepItem
                                        const isToolStep = step.step_type === 'tool_call' || step.step_type === 'error';
                                        if (isToolStep) {
                                          return <ToolCallStep key={idx} step={step} />;
                                        }
                                        return (
                                          <ReasoningStepItem
                                            key={idx}
                                            step={step}
                                            showRoundSeparator={isNewRound}
                                            roundNumber={roundNumber}
                                          />
                                        );
                                      })}
                                    </div>
                                  </TimelineSection>
                                )}

                                {/* Report */}
                                {/* Report */}
                                {detail.result?.report && (
                                  <TimelineSection
                                    icon={<FileText className="h-4 w-4 text-muted-foreground" />}
                                    title={t('agents:memory.generatedReport')}
                                  >
                                    <Card className="p-3">
                                      <pre className="text-sm whitespace-pre-wrap font-mono text-xs overflow-x-auto max-h-60">
                                        {detail.result.report}
                                      </pre>
                                    </Card>
                                  </TimelineSection>
                                )}

                                {/* ③ Conclusion + Confidence */}
                                {(() => {
                                  const dp = detail.decision_process
                                  const hasConclusion = !!dp?.conclusion
                                  const hasConfidence = dp?.confidence !== undefined
                                  if (!hasConclusion && !hasConfidence) return null
                                  return (
                                    <TimelineSection
                                      icon={<CheckCircle2 className="h-4 w-4 text-success" />}
                                      title={t('agents:memory.conclusion')}
                                    >
                                      <div className="space-y-2">
                                        {hasConclusion && (
                                          <Card className="p-4 bg-muted border-border shadow-sm">
                                            <MarkdownMessage content={dp!.conclusion} />
                                          </Card>
                                        )}
                                        {hasConfidence && (
                                          <div className="flex items-center justify-between text-sm p-2 bg-muted-50 rounded-lg">
                                            <span className="text-muted-foreground">{t('agents:memory.confidence')}</span>
                                            <Badge variant={dp!.confidence! > 0.7 ? "default" : "secondary"}>
                                              {(dp!.confidence! * 100).toFixed(0)}%
                                            </Badge>
                                          </div>
                                        )}
                                      </div>
                                    </TimelineSection>
                                  )
                                })()}

                                {/* ④ LLM Final Response */}
                                {detail.result?.summary && (() => {
                                  const summary = detail.result.summary.trim()
                                  const conclusion = detail.decision_process?.conclusion?.trim() ?? ''
                                  const isGeneric = summary === 'Completed tool execution rounds.'
                                    || summary === 'LLM generation failed during tool execution.'
                                  // Skip if conclusion already contains the same content
                                  const normalize = (s: string) => s.replace(/\s+/g, ' ').trim()
                                  const isDuplicate = normalize(summary) === normalize(conclusion)
                                    || (conclusion.length > 100 && normalize(summary).includes(normalize(conclusion).slice(0, 200)))
                                    || (summary.length > 100 && normalize(conclusion).includes(normalize(summary).slice(0, 200)))
                                  if (!summary || isGeneric || isDuplicate) return null
                                  return (
                                    <TimelineSection
                                      icon={<Sparkles className="h-4 w-4 text-accent-indigo" />}
                                      title={t('agents:memory.llmResponse', 'LLM Response')}
                                    >
                                      <CollapsibleText content={summary} maxLines={6} />
                                    </TimelineSection>
                                  )
                                })()}

                                {/* ⑤ Execution Actions — filtered, only device/extension commands */}
                                {(() => {
                                  const realActions = detail.result?.actions_executed?.filter(
                                    (a: { action_type: string }) => a.action_type !== 'tool_call'
                                  ) ?? []
                                  if (realActions.length === 0) return null
                                  return (
                                    <TimelineSection
                                      icon={<Zap className="h-4 w-4 text-warning" />}
                                      title={t('agents:memory.actionsExecuted')}
                                    >
                                      <div className="space-y-2">
                                        {realActions.map((action: any, idx: number) => (
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
                                  )
                                })()}

                                {/* ④ Notifications */}
                                {detail.result?.notifications_sent && detail.result.notifications_sent.length > 0 && (
                                  <TimelineSection
                                    icon={<Bell className="h-4 w-4 text-info" />}
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
                                                  <Clock className="h-4 w-4" />
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
  const [expanded, setExpanded] = useState(false)

  // Format values for display
  const formatValues = (values: unknown): string => {
    if (typeof values === 'string') return values
    if (typeof values === 'number' || typeof values === 'boolean') return String(values)
    if (typeof values === 'object' && values !== null) {
      const obj = values as Record<string, unknown>
      // For simple objects with few keys, show as key-value pairs
      const keys = Object.keys(obj)
      if (keys.length <= 5) {
        const pairs = keys.map(k => {
          const v = obj[k]
          if (typeof v === 'object' && v !== null) {
            return `${k}: ${JSON.stringify(v)}`
          }
          return `${k}: ${v}`
        })
        return pairs.join(', ')
      }
      return JSON.stringify(values, null, 2)
    }
    return String(values)
  }

  const formatted = formatValues(data.values)
  const isLong = formatted.length > 200
  const displayContent = expanded ? formatted : (isLong ? formatted.slice(0, 200) + '...' : formatted)

  return (
    <Card className="p-2 min-w-0">
      <div className="flex items-center justify-between mb-1 gap-2">
        <span className="text-xs font-medium truncate flex-1 min-w-0" title={data.source}>{data.source}</span>
        <Badge variant="outline" className="text-xs h-5 shrink-0">{data.data_type}</Badge>
      </div>
      <div className="text-xs bg-muted p-1.5 rounded w-full min-w-0 break-words whitespace-pre-wrap font-mono">
        {displayContent}
      </div>
      {isLong && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-primary hover:underline mt-0.5 flex items-center gap-0.5"
        >
          {expanded ? (
            <>{t('agents:memory.showLess', 'Show less')} <ChevronUp className="h-4 w-4" /></>
          ) : (
            <>{t('agents:memory.showMore', 'Show more')} <ChevronDown className="h-4 w-4" /></>
          )}
        </button>
      )}
    </Card>
  )
}

/// Collapsible text block for LLM responses and long content
function CollapsibleText({ content, maxLines = 6 }: { content: string; maxLines?: number }) {
  const { t } = useTranslation(['agents'])
  const [expanded, setExpanded] = useState(false)
  const lineCount = content.split('\n').length
  const isLong = lineCount > maxLines || content.length > 500

  return (
    <div>
      <div
        className={cn(
          "text-sm bg-muted-50 p-3 rounded-lg border whitespace-pre-wrap break-words leading-relaxed",
          !expanded && isLong && "max-h-40 overflow-hidden relative",
        )}
      >
        {expanded || !isLong ? content : content.split('\n').slice(0, maxLines).join('\n')}
        {!expanded && isLong && (
          <div className="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-muted/50 to-transparent" />
        )}
      </div>
      {isLong && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-primary hover:underline mt-1 flex items-center gap-0.5"
        >
          {expanded ? (
            <>{t('agents:memory.showLess', 'Show less')} <ChevronUp className="h-4 w-4" /></>
          ) : (
            <>{t('agents:memory.showMore', 'Show more')} <ChevronDown className="h-4 w-4" /></>
          )}
        </button>
      )}
    </div>
  )
}

/// Collapsible output display for long tool results
function CollapsibleOutput({ label, content }: { label: string; content: string }) {
  const { t } = useTranslation(['agents'])
  const [expanded, setExpanded] = useState(false)
  const isLong = content.length > 300
  const displayContent = expanded ? content : (isLong ? content.slice(0, 300) + '...' : content)

  return (
    <div className="mt-1.5">
      <div className="text-xs text-muted-foreground mb-0.5 font-medium">{label}:</div>
      <div className="text-xs bg-muted p-2 rounded break-words font-mono whitespace-pre-wrap">
        {displayContent}
      </div>
      {isLong && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-primary hover:underline mt-0.5 flex items-center gap-0.5"
        >
          {expanded ? (
            <>{t('memory.showLess', 'Show less')} <ChevronUp className="h-4 w-4" /></>
          ) : (
            <>{t('memory.showMore', 'Show more')} <ChevronDown className="h-4 w-4" /></>
          )}
        </button>
      )}
    </div>
  )
}

function ReasoningStepItem({ step, showRoundSeparator, roundNumber }: { step: ReasoningStep; showRoundSeparator?: boolean; roundNumber?: number }) {
  const { t } = useTranslation(['common', 'agents'])
  const [descExpanded, setDescExpanded] = useState(false)

  // Different visual styles based on step type
  const isThought = step.step_type === 'thought'
  const isError = step.step_type === 'error'
  const isLongDesc = step.description.length > 300
  const displayDesc = descExpanded ? step.description : (isLongDesc ? step.description.slice(0, 300) + '...' : step.description)

  const numberBg = isError ? 'bg-error text-primary-foreground' :
                    isThought ? 'bg-info text-primary-foreground' :
                    'bg-primary text-primary-foreground'
  const borderColor = isError ? 'border-error' :
                      isThought ? 'border-info-light' :
                      'border-border'

  return (
    <div>
      {/* Round separator - shown above the step content */}
      {showRoundSeparator && roundNumber !== undefined && (
        <div className="flex items-center gap-2 mb-3 -mt-1">
          <div className="h-px flex-1 bg-gradient-to-r from-transparent via-border to-transparent" />
          <span className="text-xs text-muted-foreground font-medium shrink-0 px-2">
            {t('agents:memory.round', 'Round {{round}}', { round: roundNumber })}
          </span>
          <div className="h-px flex-1 bg-gradient-to-l from-transparent via-border to-transparent" />
        </div>
      )}
      <div className="flex gap-3 min-w-0">
        <div className="flex flex-col items-center shrink-0">
          <div className={cn("w-6 h-6 rounded-full text-xs flex items-center justify-center", numberBg)}>
            {step.step_number}
          </div>
          <div className={cn("w-0.5 flex-1 min-h-[24px]", isError ? "bg-error-light" : isThought ? "bg-info-light" : "bg-border")} />
        </div>
        <div className={cn("flex-1 min-w-0 pb-4 pl-1")}>
        {/* Description with icon */}
        <div className="flex items-start gap-1.5">
          {isThought && <span className="text-info text-xs mt-0.5 shrink-0">&#x1F4AD;</span>}
          {isError && <span className="text-error text-xs mt-0.5 shrink-0">&#x26A0;</span>}
          <div className={cn(
            "text-sm break-words",
            isThought && "italic text-info",
            isError && "text-error"
          )}>
            {displayDesc}
          </div>
        </div>
        {isLongDesc && (
          <button
            type="button"
            onClick={() => setDescExpanded(!descExpanded)}
            className="text-xs text-primary hover:underline mt-0.5 flex items-center gap-0.5"
          >
            {descExpanded ? (
              <>{t('agents:memory.showLess', 'Show less')} <ChevronUp className="h-4 w-4" /></>
            ) : (
              <>{t('agents:memory.showMore', 'Show more')} <ChevronDown className="h-4 w-4" /></>
            )}
          </button>
        )}

        {/* Tool input */}
        {step.input && (
          <div className="text-xs text-muted-foreground mt-1.5 flex gap-1">
            <span className="font-medium shrink-0">{t('agents:memory.input')}:</span>
            <code className="bg-muted px-1.5 py-0.5 rounded text-xs break-all flex-1">{step.input}</code>
          </div>
        )}

        {/* Tool output - collapsible for long outputs */}
        {step.output && (
          <CollapsibleOutput label={t('agents:memory.output')} content={step.output} />
        )}

        {/* Step type badge only - no per-step confidence */}
        <Badge variant="outline" className={cn(
          "text-xs h-5 mt-1.5",
          isError && "border-error text-error",
          isThought && "border-info text-info"
        )}>
          {step.step_type}
        </Badge>
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

// --- ToolCallStep: collapsible card for tool_call / error reasoning steps ---

function extractToolName(desc: string): string {
  const match = desc.match(/tool ['"]([^'"]+)['"]/i)
  return match ? match[1] : desc.slice(0, 60)
}

function formatJsonStr(str: string): string {
  try { return JSON.stringify(JSON.parse(str), null, 2) } catch { return str }
}

/** Extract a short summary from tool output JSON/string */
function summarizeOutput(output: string, maxLen = 120): string | null {
  if (!output || output === 'null' || output === '""') return null
  try {
    const parsed = JSON.parse(output)
    if (typeof parsed === 'string') return parsed.length > maxLen ? parsed.slice(0, maxLen) + '...' : parsed
    if (typeof parsed === 'object' && parsed !== null) {
      // Try common summary fields
      for (const key of ['message', 'summary', 'description', 'status', 'result', 'error']) {
        const val = (parsed as Record<string, unknown>)[key]
        if (typeof val === 'string' && val) return val.length > maxLen ? val.slice(0, maxLen) + '...' : val
      }
      // Fallback: show first key-value pairs
      const entries = Object.entries(parsed as Record<string, unknown>).slice(0, 3)
      const summary = entries.map(([k, v]) => {
        const vs = typeof v === 'string' ? v : JSON.stringify(v)
        return `${k}: ${vs.length > 30 ? vs.slice(0, 30) + '...' : vs}`
      }).join(', ')
      return summary.length > maxLen ? summary.slice(0, maxLen) + '...' : summary
    }
    return null
  } catch {
    return output.length > maxLen ? output.slice(0, maxLen) + '...' : output
  }
}

/** Extract a short input preview (e.g. the command or key params) */
function summarizeInput(input: string, maxLen = 80): string | null {
  if (!input) return null
  try {
    const parsed = JSON.parse(input)
    if (typeof parsed === 'string') return parsed.length > maxLen ? parsed.slice(0, maxLen) + '...' : parsed
    if (typeof parsed === 'object' && parsed !== null) {
      const entries = Object.entries(parsed as Record<string, unknown>).slice(0, 2)
      const preview = entries.map(([k, v]) => {
        const vs = typeof v === 'string' ? v : JSON.stringify(v)
        return `${k}=${vs.length > 20 ? vs.slice(0, 20) + '...' : vs}`
      }).join(', ')
      return preview.length > maxLen ? preview.slice(0, maxLen) + '...' : preview
    }
    return null
  } catch {
    return input.length > maxLen ? input.slice(0, maxLen) + '...' : input
  }
}

function JsonBlock({ label, content }: { label: string; content: string }) {
  return (
    <div>
      <div className="text-xs text-muted-foreground font-medium mb-0.5">{label}</div>
      <pre className="text-xs bg-muted p-2 rounded overflow-x-auto max-h-40 whitespace-pre-wrap break-all font-mono">
        {content}
      </pre>
    </div>
  )
}

function ToolCallStep({ step }: { step: ReasoningStep }) {
  const { t } = useTranslation(['agents'])
  const [expanded, setExpanded] = useState(false)
  const isSuccess = step.step_type === 'tool_call'
  const toolName = extractToolName(step.description)
  const outputSummary = summarizeOutput(step.output)
  const inputPreview = summarizeInput(step.input || '')

  return (
    <div className="rounded-lg border bg-muted-20 overflow-hidden my-2">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-start gap-2 px-3 py-2 hover:bg-muted-30 text-left"
      >
        {isSuccess ? (
          <CheckCircle2 className="h-4 w-4 text-accent-emerald shrink-0 mt-0.5" />
        ) : (
          <XCircle className="h-4 w-4 text-error shrink-0 mt-0.5" />
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <Wrench className="h-4 w-4 text-muted-foreground shrink-0" />
            <span className="font-mono text-sm truncate">{toolName}</span>
            <span className={cn(
              textNano, "px-1.5 py-0.5 rounded shrink-0",
              isSuccess ? "bg-accent-emerald-light text-accent-emerald" : "bg-error-light text-error"
            )}>
              {isSuccess ? t('agents:memory.success') : t('agents:memory.failed')}
            </span>
          </div>
          {inputPreview && !expanded && (
            <div className={cn(textMini, "text-muted-foreground mt-0.5 truncate font-mono")}>{inputPreview}</div>
          )}
          {outputSummary && !expanded && (
            <div className={cn(textMini, "mt-0.5 truncate")}>{outputSummary}</div>
          )}
        </div>
        <ChevronDown className={cn("h-4 w-4 text-muted-foreground transition-transform shrink-0 mt-0.5", expanded && "rotate-180")} />
      </button>
      {expanded && (
        <div className="border-t px-3 py-2 space-y-2">
          {step.input && <JsonBlock label={t('agents:memory.toolInput')} content={formatJsonStr(step.input)} />}
          {step.output && <JsonBlock label={t('agents:memory.toolOutput')} content={step.output} />}
        </div>
      )}
    </div>
  )
}
