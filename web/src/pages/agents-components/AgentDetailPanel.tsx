/**
 * Agent Detail Panel - Right side of Agents page
 *
 * Shows detailed view of a selected agent with tabs.
 */

import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import {
  Bot,
  Edit,
  Play,
  Clock,
  Activity,
  Brain,
  Eye,
  Zap,
  BarChart3,
  Loader2,
  CheckCircle2,
  XCircle,
  RefreshCw,
  Settings,
  FileText,
  TrendingUp,
  Database,
  MessageSquare,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { api } from "@/lib/api"
import type { AiAgentDetail } from "@/types"
import type { AgentExecutionStartedEvent, AgentExecutionCompletedEvent } from "@/lib/events"
import { useEvents } from "@/hooks/useEvents"

// Import sub-components
import { AgentExecutionTimeline } from "./AgentExecutionTimeline"
import { AgentThinkingPanel } from "./AgentThinkingPanel"
import { AgentUserMessages } from "./AgentUserMessages"

interface AgentDetailPanelProps {
  agent: AiAgentDetail | null
  onEdit: (agent: AiAgentDetail) => void
  onExecute: (agent: AiAgentDetail) => void
  onViewExecutionDetail: (agentId: string, executionId: string) => void
  onRefresh: () => void
  inlineMode?: boolean  // When true, used inside dialog (no empty state)
}

type DetailTab = 'overview' | 'history' | 'memory' | 'messages'

// Role configuration - labels use i18n
const ROLE_CONFIG: Record<string, { icon: typeof Activity; color: string }> = {
  Monitor: { icon: Activity, color: 'text-blue-600' },
  Executor: { icon: Zap, color: 'text-orange-600' },
  Analyst: { icon: BarChart3, color: 'text-purple-600' },
}

// Status configuration - labels use i18n
const STATUS_CONFIG: Record<string, { icon: typeof CheckCircle2 | typeof Loader2 | typeof XCircle; color: string }> = {
  Active: { icon: CheckCircle2, color: 'text-green-600 bg-green-50 dark:bg-green-950/30' },
  Paused: { icon: XCircle, color: 'text-muted-foreground bg-muted/50' },
  Error: { icon: XCircle, color: 'text-red-500 bg-red-50 dark:bg-red-950/30' },
  Executing: { icon: Loader2, color: 'text-blue-600 bg-blue-50 dark:bg-blue-950/30' },
}

export function AgentDetailPanel({
  agent,
  onEdit,
  onExecute,
  onViewExecutionDetail,
  onRefresh,
  inlineMode = false,
}: AgentDetailPanelProps) {
  const { t } = useTranslation(['common', 'agents'])
  const [activeTab, setActiveTab] = useState<DetailTab>('overview')
  const [executions, setExecutions] = useState<any[]>([])
  const [executionsLoading, setExecutionsLoading] = useState(false)
  const [memory, setMemory] = useState<any>(null)
  const [memoryLoading, setMemoryLoading] = useState(false)
  const [availableResources, setAvailableResources] = useState<any>(null)

  // Real-time status from WebSocket events
  const [realtimeStatus, setRealtimeStatus] = useState<string | null>(null)

  // Load executions when history tab is active
  useEffect(() => {
    if (agent && activeTab === 'history') {
      loadExecutions()
    }
  }, [agent, activeTab])

  // Load memory when memory tab is active
  useEffect(() => {
    if (agent && activeTab === 'memory') {
      loadMemory()
    }
  }, [agent, activeTab])

  // Load available resources
  useEffect(() => {
    if (agent?.id) {
      loadAvailableResources()
    }
  }, [agent?.id])

  // Listen to WebSocket events for real-time agent status updates
  useEvents({
    enabled: !!agent?.id,
    eventTypes: ['AgentExecutionStarted', 'AgentExecutionCompleted'],
    onEvent: (event) => {
      if (!agent) return

      const eventData = event.data as { agent_id?: string }

      switch (event.type) {
        case 'AgentExecutionStarted': {
          const startedData = (event as AgentExecutionStartedEvent).data
          if (startedData.agent_id === agent.id) {
            setRealtimeStatus('Executing')
          }
          break
        }

        case 'AgentExecutionCompleted': {
          const completedData = (event as AgentExecutionCompletedEvent).data
          if (completedData.agent_id === agent.id) {
            // Clear realtime status - agent's original status will be used
            setRealtimeStatus(null)
            // Reload agent data to get updated stats
            api.getAgent(agent.id).then(updatedAgent => {
              // Notify parent to refresh if needed
              onRefresh()
            }).catch(console.error)
          }
          break
        }
      }
    },
  })

  const loadExecutions = async () => {
    if (!agent) return
    setExecutionsLoading(true)
    try {
      const data = await api.getAgentExecutions(agent.id)
      setExecutions(data.executions || [])
    } catch (error) {
      console.error('Failed to load executions:', error)
    } finally {
      setExecutionsLoading(false)
    }
  }

  const loadMemory = async () => {
    if (!agent) return
    setMemoryLoading(true)
    try {
      const data = await api.getAgentMemory(agent.id)
      setMemory(data)
    } catch (error) {
      console.error('Failed to load memory:', error)
    } finally {
      setMemoryLoading(false)
    }
  }

  const loadAvailableResources = async () => {
    // This endpoint is not implemented yet, skip for now
    // TODO: Implement /api/agents/{id}/available-resources endpoint
    return
  }

  // Empty state (only in non-inline mode)
  if (!agent && !inlineMode) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center text-muted-foreground">
          <Bot className="h-16 w-16 mx-auto mb-4 opacity-20" />
          <p className="text-lg">{t('agents:detail.selectAgent')}</p>
        </div>
      </div>
    )
  }

  // Return null if no agent in inline mode (dialog will handle it)
  if (!agent) return null

  // Use realtime status from WebSocket if available, otherwise use agent's status
  const currentStatus = realtimeStatus || agent.status
  const statusConfig = STATUS_CONFIG[currentStatus] || STATUS_CONFIG.Paused
  const StatusIcon = statusConfig.icon

  // Get status label from i18n
  const getStatusLabel = (status: string) => {
    const key = status.toLowerCase() as 'active' | 'paused' | 'error' | 'executing'
    return t(`agents:status.${key}`)
  }

  // Format duration - handles undefined/null/NaN values
  const formatDuration = (ms: number | undefined | null) => {
    if (ms === undefined || ms === null || Number.isNaN(ms) || ms < 0) {
      return '--'
    }
    if (ms < 1000) return `${ms}ms`
    return `${(ms / 1000).toFixed(1)}s`
  }

  // Safe number to string conversion
  const formatCount = (count: number | undefined | null) => {
    return count !== undefined && count !== null && !Number.isNaN(count) ? count : '--'
  }

  return (
    <div className="h-full flex flex-col">
      {/* Unified Header */}
      <div className="p-4 border-b bg-muted/20">
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-center gap-3">
            <div className={cn(
              "w-10 h-10 rounded-lg flex items-center justify-center",
              currentStatus === 'Active' || currentStatus === 'Executing'
                ? "bg-purple-500/20 text-purple-600"
                : "bg-muted text-muted-foreground"
            )}>
              <Bot className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-lg font-semibold">{agent.name}</h2>
                <Badge className={cn("text-xs gap-1", statusConfig.color)}>
                  <StatusIcon className={cn("h-3 w-3", currentStatus === 'Executing' && "animate-spin")} />
                  {getStatusLabel(currentStatus)}
                </Badge>
              </div>
              <p className="text-sm text-muted-foreground mt-0.5 line-clamp-1 max-w-lg">
                {agent.description || t('agents:card.noDescription')}
              </p>
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1.5">
            <Button variant="ghost" size="icon" className="h-9 w-9" onClick={onRefresh}>
              <RefreshCw className="h-4 w-4" />
            </Button>
            <Button variant="outline" size="sm" onClick={() => onEdit(agent)}>
              <Edit className="h-3.5 w-3.5 mr-1.5" />
              {t('agents:detail.edit')}
            </Button>
            <Button size="sm" onClick={() => onExecute(agent)} disabled={currentStatus === 'Executing'}>
              <Play className="h-3.5 w-3.5 mr-1.5" />
              {t('agents:detail.execute')}
            </Button>
          </div>
        </div>

        {/* Stats - use agent.stats for detailed view, with fallback to inherited fields */}
        <div className="flex items-center gap-6 text-sm">
          <div className="flex items-center gap-1.5">
            <Activity className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-muted-foreground">{t('agents:detail.executions')}</span>
            <span className="font-medium">{formatCount(agent.stats?.total_executions ?? agent.execution_count)}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
            <span className="font-medium text-green-600">{formatCount(agent.stats?.successful_executions ?? agent.success_count)}</span>
            <span className="text-muted-foreground">{t('agents:detail.success')}</span>
          </div>
          {((agent.stats?.failed_executions ?? agent.error_count ?? 0) > 0) && (
            <div className="flex items-center gap-1.5">
              <XCircle className="h-3.5 w-3.5 text-red-500" />
              <span className="font-medium text-red-500">{formatCount(agent.stats?.failed_executions ?? agent.error_count)}</span>
              <span className="text-muted-foreground">{t('agents:detail.failed')}</span>
            </div>
          )}
          <div className="flex items-center gap-1.5">
            <Clock className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="font-medium">{formatDuration(agent.stats?.avg_duration_ms ?? agent.avg_duration_ms)}</span>
            <span className="text-muted-foreground">{t('agents:detail.avgDuration')}</span>
          </div>
        </div>
      </div>

      {/* Real-time Thinking Panel - shows during execution */}
      {agent.id && (
        <AgentThinkingPanel
          agentId={agent.id}
          isExecuting={currentStatus === 'Executing'}
        />
      )}

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as DetailTab)} className="flex-1 flex flex-col">
        <div className="px-4 pt-3">
          <TabsList className="w-full justify-start bg-muted/50 h-9">
            <TabsTrigger value="overview" className="h-7 text-sm">
              <Eye className="h-3.5 w-3.5 mr-1.5" />
              {t('agents:detail.overview')}
            </TabsTrigger>
            <TabsTrigger value="history" className="h-7 text-sm">
              <Clock className="h-3.5 w-3.5 mr-1.5" />
              {t('agents:detail.history')}
            </TabsTrigger>
            <TabsTrigger value="memory" className="h-7 text-sm">
              <Brain className="h-3.5 w-3.5 mr-1.5" />
              {t('agents:detail.memory')}
            </TabsTrigger>
            <TabsTrigger value="messages" className="h-7 text-sm">
              <MessageSquare className="h-3.5 w-3.5 mr-1.5" />
              {t('agents:detail.messages')}
            </TabsTrigger>
          </TabsList>
        </div>

        {/* Tab Contents */}
        <div className="flex-1 min-h-0">
          {/* Overview Tab */}
          <TabsContent value="overview" className="h-full m-0 p-4 pt-2">
            <ScrollArea className="h-full">
              <div className="space-y-4 pr-2">
                {/* Stats Grid - Top section */}
                <DetailSection title="" icon={null}>
                  <div className="grid grid-cols-4 gap-2">
                    <StatItem
                      icon={<Activity className="h-3.5 w-3.5" />}
                      label={t('agents:detail.executions')}
                      value={formatCount(agent.stats?.total_executions ?? agent.execution_count)}
                      color="text-blue-500"
                    />
                    <StatItem
                      icon={<CheckCircle2 className="h-3.5 w-3.5" />}
                      label={t('agents:detail.success')}
                      value={formatCount(agent.stats?.successful_executions ?? agent.success_count)}
                      color="text-green-500"
                    />
                    <StatItem
                      icon={<XCircle className="h-3.5 w-3.5" />}
                      label={t('agents:detail.failed')}
                      value={formatCount(agent.stats?.failed_executions ?? agent.error_count)}
                      color="text-red-500"
                    />
                    <StatItem
                      icon={<Clock className="h-3.5 w-3.5" />}
                      label={t('agents:detail.avgDuration')}
                      value={formatDuration(agent.stats?.avg_duration_ms ?? agent.avg_duration_ms)}
                      color="text-orange-500"
                    />
                  </div>
                </DetailSection>

                {/* User Intent */}
                <DetailSection title={t('agents:userPrompt')} icon={FileText}>
                  <div className="text-sm leading-relaxed whitespace-pre-wrap text-foreground/80">
                    {agent.user_prompt || t('agents:card.noDescription')}
                  </div>
                  {agent.parsed_intent && (
                    <div className="mt-3 pt-3 border-t border-border/50">
                      <div className="text-xs text-muted-foreground mb-1.5">{t('agents:creator.basicInfo.requirement')}</div>
                      <div className="text-sm">
                        <span className="inline-flex items-center gap-1.5 px-2 py-1 rounded bg-blue-500/10 text-blue-600 dark:text-blue-400">
                          {agent.parsed_intent.intent_type || '-'}
                        </span>
                      </div>
                    </div>
                  )}
                </DetailSection>

                {/* Schedule & Config - Two columns */}
                <div className="grid grid-cols-2 gap-4">
                  {/* Schedule */}
                  <DetailSection title={t('agents:detail.schedule')} icon={Clock}>
                    <div className="space-y-1.5">
                      <InfoRow label={t('agents:detail.type')} value={agent.schedule.schedule_type} />
                      {agent.schedule.interval_seconds && (
                        <InfoRow label={t('agents:detail.interval')} value={`${agent.schedule.interval_seconds}s`} />
                      )}
                      {agent.schedule.cron_expression && (
                        <InfoRow label="Cron" value={agent.schedule.cron_expression} mono />
                      )}
                      {agent.schedule.event_filter && (
                        <InfoRow label={t('agents:creator.schedule.event.triggerEvent')} value={agent.schedule.event_filter} mono />
                      )}
                    </div>
                  </DetailSection>

                  {/* LLM Config */}
                  {agent.llm_backend_id ? (
                    <DetailSection title={t('agents:creator.basicInfo.llmBackend')} icon={Brain}>
                      <InfoRow label={t('agents:creator.basicInfo.llmBackend')} value={agent.llm_backend_id} mono />
                    </DetailSection>
                  ) : (
                    <DetailSection title={t('common:info')} icon={Settings}>
                      <div className="space-y-1.5">
                        <InfoRow label={t('common:createdAt')} value={new Date(agent.created_at).toLocaleString()} />
                        <InfoRow label={t('common:updatedAt')} value={new Date(agent.updated_at).toLocaleString()} />
                        {agent.last_execution_at && (
                          <InfoRow label={t('agents:lastExecution')} value={new Date(agent.last_execution_at).toLocaleString()} />
                        )}
                      </div>
                    </DetailSection>
                  )}
                </div>

                {/* Resources - Full width */}
                <DetailSection title={`${t('agents:detail.resources')} (${agent.resources.length})`} icon={Zap}>
                  <div className="space-y-3">
                    {/* Resource summary counts - group by actual types */}
                    <div className="flex flex-wrap gap-3">
                      {Object.entries(
                        agent.resources.reduce((acc, r) => {
                          const type = r.resource_type.toLowerCase()
                          acc[type] = (acc[type] || 0) + 1
                          return acc
                        }, {} as Record<string, number>)
                      ).map(([type, count]) => (
                        <div key={type} className="flex items-center gap-2 px-3 py-1.5 rounded-md bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-500/20 text-sm">
                          <span className="capitalize text-muted-foreground">{type}:</span>
                          <span className="font-semibold">{count}</span>
                        </div>
                      ))}
                    </div>
                    {/* Resource list */}
                    <div className="grid grid-cols-2 gap-2">
                      {agent.resources.slice(0, 8).map((resource, idx) => (
                        <div key={idx} className="flex items-center justify-between px-2.5 py-1.5 rounded bg-background border">
                          <span className="text-sm truncate flex-1 mr-2" title={resource.resource_id}>
                            {resource.name || resource.resource_id}
                          </span>
                          <Badge variant="secondary" className="text-xs shrink-0">
                            {resource.resource_type}
                          </Badge>
                        </div>
                      ))}
                    </div>
                    {agent.resources.length > 8 && (
                      <div className="text-xs text-muted-foreground text-center pt-1">
                        {t('agents:detail.moreResources', { count: agent.resources.length - 8 })}
                      </div>
                    )}
                  </div>
                </DetailSection>

                {/* Timestamps - if LLM backend was shown above */}
                {agent.llm_backend_id && (
                  <DetailSection title={t('common:info')} icon={Settings}>
                    <div className="grid grid-cols-3 gap-2">
                      <InfoRow label={t('common:createdAt')} value={new Date(agent.created_at).toLocaleString()} />
                      <InfoRow label={t('common:updatedAt')} value={new Date(agent.updated_at).toLocaleString()} />
                      {agent.last_execution_at && (
                        <InfoRow label={t('agents:lastExecution')} value={new Date(agent.last_execution_at).toLocaleString()} />
                      )}
                    </div>
                  </DetailSection>
                )}
              </div>
            </ScrollArea>
          </TabsContent>

          {/* History Tab */}
          <TabsContent value="history" className="h-full m-0">
            <AgentExecutionTimeline
              executions={executions}
              loading={executionsLoading}
              agentId={agent.id}
              onViewExecutionDetail={onViewExecutionDetail}
            />
          </TabsContent>

          {/* Memory Tab */}
          <TabsContent value="memory" className="h-full m-0 p-4 pt-2">
            <MemoryContent memory={memory} loading={memoryLoading} />
          </TabsContent>

          {/* Messages Tab */}
          <TabsContent value="messages" className="h-full m-0">
            <AgentUserMessages
              agentId={agent.id}
              onMessageAdded={() => {
                // Refresh agent data to show updated message count
                onRefresh()
              }}
            />
          </TabsContent>
        </div>
      </Tabs>
    </div>
  )
}

// ============================================================================
// Sub Components
// ============================================================================

// Unified Section Component for all detail displays
interface DetailSectionProps {
  title: string
  icon: React.ComponentType<{ className?: string }> | null
  children: React.ReactNode
}

function DetailSection({ title, icon: Icon, children }: DetailSectionProps) {
  return (
    <div className="bg-muted/20 rounded-lg p-3">
      {title && Icon && (
        <h3 className="text-sm font-medium flex items-center gap-2 mb-3 text-muted-foreground">
          <Icon className="h-4 w-4" />
          {title}
        </h3>
      )}
      {children}
    </div>
  )
}

// Compact Stat Item for stats grid
interface StatItemProps {
  icon: React.ReactNode
  label: string
  value: string | number
  color: string
}

function StatItem({ icon, label, value, color }: StatItemProps) {
  return (
    <div className="flex items-center gap-2 px-2.5 py-2 rounded bg-background border">
      <div className={cn("shrink-0", color)}>{icon}</div>
      <div className="flex-1 min-w-0">
        <div className="text-xs text-muted-foreground truncate">{label}</div>
        <div className="text-sm font-semibold truncate">{value}</div>
      </div>
    </div>
  )
}

// Resource Count Item
interface ResourceCountItemProps {
  color: 'blue' | 'purple' | 'green' | 'orange'
  label: string
  count: number
}

function ResourceCountItem({ color, label, count }: ResourceCountItemProps) {
  const colorMap = {
    blue: 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20',
    purple: 'bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20',
    green: 'bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20',
    orange: 'bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20',
  }
  return (
    <div className={cn("flex items-center gap-2 px-3 py-1.5 rounded border text-sm", colorMap[color])}>
      <span className="text-muted-foreground">{label}:</span>
      <span className="font-semibold">{count}</span>
    </div>
  )
}

// Info Row Component
interface InfoRowProps {
  label: string
  value: string | number
  mono?: boolean
}

function InfoRow({ label, value, mono }: InfoRowProps) {
  return (
    <div className="flex justify-between items-center py-1 text-sm">
      <span className="text-muted-foreground text-xs">{label}</span>
      <span className={cn("font-medium text-xs truncate max-w-[180px]", mono && "font-mono")}>{value}</span>
    </div>
  )
}

// ============================================================================
// Memory Content - Structured and readable display
// ============================================================================

interface MemoryContentProps {
  memory: any
  loading: boolean
}

function MemoryContent({ memory, loading }: MemoryContentProps) {
  const { t } = useTranslation(['common', 'agents'])

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!memory) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
        <Brain className="h-12 w-12 mb-3 opacity-20" />
        <p>{t('agents:detail.noMemory')}</p>
      </div>
    )
  }

  // Count memory items
  const stateVarCount = Object.keys(memory.state_variables || {}).length
  const learnedPatternsCount = memory.learned_patterns?.length || 0
  const baselinesCount = memory.baselines?.length || 0
  const trendDataCount = Object.keys(memory.trend_data || {}).length

  // Check if memory is empty
  const isEmptyMemory = stateVarCount === 0 && learnedPatternsCount === 0 && baselinesCount === 0 && trendDataCount === 0

  if (isEmptyMemory) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
        <Brain className="h-12 w-12 mb-3 opacity-20" />
        <p>{t('agents:detail.noMemory')}</p>
        <p className="text-xs mt-1">{t('agents:memory.title')}</p>
      </div>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="space-y-3 pr-2">
        {/* State Variables */}
        {stateVarCount > 0 && (
          <DetailSection
            title={t('agents:detail.stateVariables')}
            icon={Brain}
          >
            <div className="grid grid-cols-2 gap-2">
              {Object.entries(memory.state_variables || {}).map(([key, value]) => (
                <div key={key} className="flex items-center justify-between px-2.5 py-1.5 rounded bg-background border">
                  <span className="text-xs font-medium truncate flex-1 mr-2" title={key}>{key}</span>
                  <span className="text-xs font-mono text-muted-foreground truncate max-w-[120px]" title={String(value)}>
                    {typeof value === 'object' ? JSON.stringify(value) : String(value)}
                  </span>
                </div>
              ))}
            </div>
          </DetailSection>
        )}

        {/* Learned Patterns */}
        {learnedPatternsCount > 0 && (
          <DetailSection
            title={t('agents:detail.learnedPatterns')}
            icon={TrendingUp}
          >
            <div className="space-y-1.5">
              {memory.learned_patterns.map((pattern: string, idx: number) => (
                <div key={idx} className="px-2.5 py-2 rounded bg-background border text-sm">
                  {pattern}
                </div>
              ))}
            </div>
          </DetailSection>
        )}

        {/* Baselines - more structured */}
        {baselinesCount > 0 && (
          <DetailSection
            title={t('agents:detail.baselines')}
            icon={Database}
          >
            <div className="space-y-2">
              {memory.baselines.map((baseline: any, idx: number) => (
                <div key={idx} className="px-3 py-2.5 rounded bg-background border">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium">{baseline.metric || `Metric ${idx + 1}`}</span>
                    <span className="text-xs text-muted-foreground">
                      {t('agents:memory.dataPoints')}: {baseline.data_points || 0}
                    </span>
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    <div className="text-center">
                      <div className="text-[10px] text-muted-foreground uppercase tracking-wide">{t('agents:memory.value')}</div>
                      <div className="text-sm font-mono font-semibold">{baseline.mean?.toFixed(2) || '-'}</div>
                    </div>
                    <div className="text-center">
                      <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Min</div>
                      <div className="text-sm font-mono font-semibold text-green-600">{baseline.min?.toFixed(2) || '-'}</div>
                    </div>
                    <div className="text-center">
                      <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Max</div>
                      <div className="text-sm font-mono font-semibold text-red-600">{baseline.max?.toFixed(2) || '-'}</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </DetailSection>
        )}

        {/* Trend Data */}
        {trendDataCount > 0 && (
          <DetailSection
            title={t('agents:detail.trendData')}
            icon={Activity}
          >
            <div className="grid grid-cols-2 gap-2">
              {Object.entries(memory.trend_data || {}).map(([key, data]: [string, any]) => (
                <div key={key} className="px-3 py-2.5 rounded bg-background border">
                  <div className="flex items-center justify-between mb-1.5">
                    <span className="text-xs font-medium truncate flex-1" title={key}>{key}</span>
                    <Badge
                      variant={data.trend === 'up' ? 'default' : data.trend === 'down' ? 'destructive' : 'secondary'}
                      className="text-xs shrink-0 ml-1"
                    >
                      {data.trend || 'neutral'}
                    </Badge>
                  </div>
                  {data.current_value !== undefined && (
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-muted-foreground">{t('agents:memory.value')}:</span>
                      <span className="font-semibold">{data.current_value}</span>
                      {data.change !== undefined && (
                        <span className={cn(
                          "ml-auto",
                          data.change > 0 ? "text-green-600" : data.change < 0 ? "text-red-600" : "text-muted-foreground"
                        )}>
                          {data.change > 0 ? '↑' : data.change < 0 ? '↓' : '→'}
                          {Math.abs(data.change).toFixed(1)}%
                        </span>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </DetailSection>
        )}

        {/* Updated At footer */}
        {memory.updated_at && (
          <div className="text-xs text-center text-muted-foreground py-2">
            {t('agents:memory.updatedAt')}: {
              typeof memory.updated_at === 'number'
                ? new Date(memory.updated_at * 1000).toLocaleString()
                : new Date(memory.updated_at).toLocaleString()
            }
          </div>
        )}
      </div>
    </ScrollArea>
  )
}
