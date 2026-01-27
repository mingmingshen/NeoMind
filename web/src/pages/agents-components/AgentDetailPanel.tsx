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
} from "lucide-react"
import { cn } from "@/lib/utils"
import { api } from "@/lib/api"
import { useAgentEvents, useAgentStatus, type AgentThinkingStep } from "@/hooks/useAgentEvents"
import type { AiAgentDetail } from "@/types"

// Import sub-components
import { AgentExecutionsList } from "./AgentExecutionsList"

interface AgentDetailPanelProps {
  agent: AiAgentDetail | null
  onEdit: (agent: AiAgentDetail) => void
  onExecute: (agent: AiAgentDetail) => void
  onViewExecutionDetail: (agentId: string, executionId: string) => void
  onRefresh: () => void
  inlineMode?: boolean  // When true, used inside dialog (no empty state)
}

type DetailTab = 'overview' | 'live' | 'history' | 'memory'

// Role configuration
const ROLE_CONFIG: Record<string, { label: string; icon: typeof Activity; color: string }> = {
  Monitor: { label: '监控', icon: Activity, color: 'text-blue-600' },
  Executor: { label: '执行', icon: Zap, color: 'text-orange-600' },
  Analyst: { label: '分析', icon: BarChart3, color: 'text-purple-600' },
}

// Status configuration
const STATUS_CONFIG: Record<string, { label: string; icon: typeof CheckCircle2; color: string }> = {
  Active: { label: '运行中', icon: CheckCircle2, color: 'text-green-600 bg-green-50 dark:bg-green-950/30' },
  Paused: { label: '已暂停', icon: XCircle, color: 'text-muted-foreground bg-muted/50' },
  Error: { label: '错误', icon: XCircle, color: 'text-red-500 bg-red-50 dark:bg-red-950/30' },
  Executing: { label: '执行中', icon: Loader2, color: 'text-blue-600 bg-blue-50 dark:bg-blue-950/30' },
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

  // Use agent events hook for real-time updates
  const { isConnected, currentExecution, thinkingSteps, decisions } = useAgentEvents(
    agent?.id || '',
    { enabled: !!agent && activeTab === 'live' }
  )

  // Use agent status polling
  const { status: polledStatus } = useAgentStatus(agent?.id || '', { enabled: !!agent })

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
    if (!agent?.id) return
    try {
      const data = await api.getAgentAvailableResources(agent.id)
      setAvailableResources(data)
    } catch (error) {
      console.error('Failed to load available resources:', error)
    }
  }

  // Empty state (only in non-inline mode)
  if (!agent && !inlineMode) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center text-muted-foreground">
          <Bot className="h-16 w-16 mx-auto mb-4 opacity-20" />
          <p className="text-lg">选择一个智能体查看详情</p>
        </div>
      </div>
    )
  }

  // Return null if no agent in inline mode (dialog will handle it)
  if (!agent) return null

  const roleConfig = ROLE_CONFIG[agent.role] || ROLE_CONFIG.Monitor
  const RoleIcon = roleConfig.icon
  const currentStatus = polledStatus || agent.status
  const statusConfig = STATUS_CONFIG[currentStatus] || STATUS_CONFIG.Paused
  const StatusIcon = statusConfig.icon

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
                <Badge variant="secondary" className="text-xs">
                  <RoleIcon className="h-3 w-3 mr-1" />
                  {roleConfig.label}
                </Badge>
                <Badge className={cn("text-xs gap-1", statusConfig.color)}>
                  <StatusIcon className={cn("h-3 w-3", currentStatus === 'Executing' && "animate-spin")} />
                  {statusConfig.label}
                </Badge>
              </div>
              <p className="text-sm text-muted-foreground mt-0.5 line-clamp-1 max-w-lg">
                {agent.user_prompt}
              </p>
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1.5">
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onRefresh}>
              <RefreshCw className="h-4 w-4" />
            </Button>
            <Button variant="outline" size="sm" className="h-8" onClick={() => onEdit(agent)}>
              <Edit className="h-3.5 w-3.5 mr-1.5" />
              编辑
            </Button>
            <Button size="sm" className="h-8" onClick={() => onExecute(agent)} disabled={currentStatus === 'Executing'}>
              <Play className="h-3.5 w-3.5 mr-1.5" />
              执行
            </Button>
          </div>
        </div>

        {/* Stats */}
        <div className="flex items-center gap-6 text-sm">
          <div className="flex items-center gap-1.5">
            <Activity className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-muted-foreground">执行</span>
            <span className="font-medium">{agent.execution_count}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
            <span className="font-medium text-green-600">{agent.success_count}</span>
            <span className="text-muted-foreground">成功</span>
          </div>
          {agent.error_count > 0 && (
            <div className="flex items-center gap-1.5">
              <XCircle className="h-3.5 w-3.5 text-red-500" />
              <span className="font-medium text-red-500">{agent.error_count}</span>
              <span className="text-muted-foreground">失败</span>
            </div>
          )}
          <div className="flex items-center gap-1.5">
            <Clock className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="font-medium">{agent.avg_duration_ms}ms</span>
            <span className="text-muted-foreground">平均</span>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as DetailTab)} className="flex-1 flex flex-col">
        <div className="px-4 pt-3">
          <TabsList className="w-full justify-start bg-muted/50 h-9">
            <TabsTrigger value="overview" className="h-7 text-sm">
              <Eye className="h-3.5 w-3.5 mr-1.5" />
              概览
            </TabsTrigger>
            <TabsTrigger value="live" className="h-7 text-sm">
              <Activity className="h-3.5 w-3.5 mr-1.5" />
              实时监控
              {isConnected && <span className="ml-1 w-1.5 h-1.5 bg-green-500 rounded-full animate-pulse" />}
            </TabsTrigger>
            <TabsTrigger value="history" className="h-7 text-sm">
              <Clock className="h-3.5 w-3.5 mr-1.5" />
              历史
            </TabsTrigger>
            <TabsTrigger value="memory" className="h-7 text-sm">
              <Brain className="h-3.5 w-3.5 mr-1.5" />
              记忆
            </TabsTrigger>
          </TabsList>
        </div>

        {/* Tab Contents */}
        <div className="flex-1 min-h-0">
          {/* Overview Tab */}
          <TabsContent value="overview" className="h-full m-0 p-4 pt-2">
            <ScrollArea className="h-full">
              <div className="space-y-4 pr-2">
                {/* Schedule Info */}
                <Section title="执行策略" icon={Clock}>
                  <InfoRow label="类型" value={agent.schedule.schedule_type} />
                  {agent.schedule.interval_seconds && (
                    <InfoRow label="间隔" value={`${agent.schedule.interval_seconds}s`} />
                  )}
                  {agent.schedule.cron_expression && (
                    <InfoRow label="Cron" value={agent.schedule.cron_expression} mono />
                  )}
                </Section>

                {/* Resources */}
                <Section title={`资源 (${agent.resources.length})`} icon={Zap}>
                  <div className="grid grid-cols-2 gap-2">
                    {agent.resources.slice(0, 6).map((resource, idx) => (
                      <div key={idx} className="flex items-center justify-between text-sm p-2 bg-muted/30 rounded">
                        <span className="truncate text-muted-foreground">{resource.resource_id}</span>
                        <Badge variant="outline" className="text-xs border-0 bg-muted/50">{resource.resource_type}</Badge>
                      </div>
                    ))}
                  </div>
                  {agent.resources.length > 6 && (
                    <div className="text-xs text-muted-foreground mt-2">
                      还有 {agent.resources.length - 6} 个资源...
                    </div>
                  )}
                </Section>

                {/* System Assets */}
                {availableResources && (
                  <Section title="系统资产" icon={Zap}>
                    <div className="flex items-center gap-4 text-sm mb-3">
                      <div className="flex items-center gap-1.5">
                        <div className="w-2 h-2 rounded-full bg-green-500" />
                        <span className="text-muted-foreground">设备:</span>
                        <span className="font-medium">{availableResources.summary?.total_devices || 0}</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <div className="w-2 h-2 rounded-full bg-blue-500" />
                        <span className="text-muted-foreground">指标:</span>
                        <span className="font-medium">{availableResources.summary?.total_metrics || 0}</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <div className="w-2 h-2 rounded-full bg-purple-500" />
                        <span className="text-muted-foreground">指令:</span>
                        <span className="font-medium">{availableResources.summary?.total_commands || 0}</span>
                      </div>
                    </div>
                  </Section>
                )}
              </div>
            </ScrollArea>
          </TabsContent>

          {/* Live Monitor Tab */}
          <TabsContent value="live" className="h-full m-0 p-4 pt-2">
            <LiveMonitorContent
              agent={agent}
              isConnected={isConnected}
              currentExecution={currentExecution}
              thinkingSteps={thinkingSteps}
              decisions={decisions}
              availableResources={availableResources}
              onLoadResources={loadAvailableResources}
            />
          </TabsContent>

          {/* History Tab */}
          <TabsContent value="history" className="h-full m-0">
            <AgentExecutionsList
              executions={executions}
              loading={executionsLoading}
              agentId={agent.id}
              onViewDetail={onViewExecutionDetail}
            />
          </TabsContent>

          {/* Memory Tab */}
          <TabsContent value="memory" className="h-full m-0 p-4 pt-2">
            <MemoryContent memory={memory} loading={memoryLoading} />
          </TabsContent>
        </div>
      </Tabs>
    </div>
  )
}

// ============================================================================
// Sub Components
// ============================================================================

interface SectionProps {
  title: string
  icon: React.ComponentType<{ className?: string }>
  children: React.ReactNode
}

function Section({ title, icon: Icon, children }: SectionProps) {
  return (
    <div>
      <h3 className="text-sm font-medium flex items-center gap-2 mb-2 text-muted-foreground">
        <Icon className="h-4 w-4" />
        {title}
      </h3>
      <div className="bg-muted/20 rounded-lg p-3">
        {children}
      </div>
    </div>
  )
}

interface InfoRowProps {
  label: string
  value: string | number
  mono?: boolean
}

function InfoRow({ label, value, mono }: InfoRowProps) {
  return (
    <div className="flex justify-between py-1.5 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className={cn("font-medium", mono && "font-mono text-xs")}>{value}</span>
    </div>
  )
}

// ============================================================================
// Live Monitor Content
// ============================================================================

interface LiveMonitorContentProps {
  agent: AiAgentDetail
  isConnected: boolean
  currentExecution: any
  thinkingSteps: AgentThinkingStep[]
  decisions: Array<{
    description: string
    rationale: string
    action: string
    confidence: number
    timestamp: number
  }>
  availableResources: any
  onLoadResources: () => void
}

function LiveMonitorContent({
  agent,
  isConnected,
  currentExecution,
  thinkingSteps,
  decisions,
  availableResources,
  onLoadResources,
}: LiveMonitorContentProps) {
  const { t } = useTranslation(['common', 'agents'])

  return (
    <ScrollArea className="h-full">
      <div className="space-y-4 pr-2">
        {/* Connection Status */}
        <div className="flex items-center justify-between p-3 bg-muted/20 rounded-lg">
          <div className="flex items-center gap-2 text-sm">
            <div className={cn(
              "w-2 h-2 rounded-full",
              isConnected ? "bg-green-500" : "bg-muted"
            )} />
            <span className="text-muted-foreground">
              {isConnected ? '已连接' : '未连接'}
            </span>
          </div>
          {agent.status === 'Executing' && (
            <Badge variant="outline" className="gap-1 text-xs">
              <Loader2 className="h-3 w-3 animate-spin" />
              执行中
            </Badge>
          )}
        </div>

        {/* Current Execution */}
        {currentExecution || agent.status === 'Executing' ? (
          <div>
            <h3 className="text-sm font-medium mb-2 text-muted-foreground flex items-center gap-2">
              <Activity className="h-4 w-4" />
              当前执行
              <span className="text-xs">#{currentExecution?.id?.slice(0, 8) || '...'}</span>
            </h3>
            <div className="bg-muted/20 rounded-lg p-3">
              {thinkingSteps.length === 0 ? (
                <div className="flex items-center justify-center py-6 text-muted-foreground">
                  <Loader2 className="h-5 w-5 animate-spin mr-2" />
                  <span className="text-sm">等待数据...</span>
                </div>
              ) : (
                <div className="space-y-3">
                  {thinkingSteps.map((step, idx) => (
                    <div key={idx} className="flex gap-3">
                      <div className="flex flex-col items-center">
                        <div className={cn(
                          "w-6 h-6 rounded-full flex items-center justify-center text-xs",
                          step.status === 'completed' ? "bg-green-500 text-white" :
                          step.status === 'in_progress' ? "bg-blue-500 text-white animate-pulse" :
                          "bg-muted text-muted-foreground"
                        )}>
                          {step.status === 'completed' ? '✓' : idx + 1}
                        </div>
                        {idx < thinkingSteps.length - 1 && (
                          <div className="w-0.5 flex-1 bg-border min-h-[2rem]" />
                        )}
                      </div>
                      <div className={cn(
                        "flex-1 pb-3",
                        step.status === 'in_progress' && "animate-pulse"
                      )}>
                        <div className="text-sm font-medium">{step.description}</div>
                        {step.details?.data != null && (
                          <div className="text-xs bg-muted p-2 rounded mt-2 font-mono truncate">
                            {String(JSON.stringify(step.details.data))}
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ) : (
          <div className="text-center py-8 text-muted-foreground bg-muted/20 rounded-lg">
            <Activity className="h-10 w-10 mx-auto mb-2 opacity-30" />
            <p className="text-sm">暂无活动执行</p>
          </div>
        )}

        {/* Decisions */}
        {decisions.length > 0 && (
          <div>
            <h3 className="text-sm font-medium mb-2 text-muted-foreground flex items-center gap-2">
              <Brain className="h-4 w-4" />
              最近决策
            </h3>
            <div className="space-y-2">
              {decisions.map((decision, idx) => (
                <div key={idx} className="p-3 bg-muted/20 rounded-lg">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-medium">{decision.description}</span>
                    <Badge variant="outline" className="text-xs">
                      {Math.round(decision.confidence * 100)}%
                    </Badge>
                  </div>
                  <div className="text-xs text-muted-foreground">{decision.rationale}</div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </ScrollArea>
  )
}

// ============================================================================
// Memory Content
// ============================================================================

interface MemoryContentProps {
  memory: any
  loading: boolean
}

function MemoryContent({ memory, loading }: MemoryContentProps) {
  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!memory) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <Brain className="h-12 w-12 mx-auto mb-3 opacity-20" />
        <p>暂无记忆数据</p>
      </div>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="space-y-4 pr-2">
        {Object.keys(memory.state_variables || {}).length > 0 && (
          <div>
            <h3 className="text-sm font-medium mb-2 text-muted-foreground flex items-center gap-2">
              <Brain className="h-4 w-4" />
              状态变量
            </h3>
            <div className="bg-muted/20 rounded-lg p-3">
              <div className="grid grid-cols-2 gap-2">
                {Object.entries(memory.state_variables || {}).map(([key, value]) => (
                  <div key={key} className="p-2 bg-background rounded">
                    <div className="text-xs text-muted-foreground">{key}</div>
                    <div className="text-sm font-mono truncate">{String(value)}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {memory.learned_patterns && memory.learned_patterns.length > 0 && (
          <div>
            <h3 className="text-sm font-medium mb-2 text-muted-foreground flex items-center gap-2">
              <TrendingUp className="h-4 w-4" />
              学习模式
            </h3>
            <div className="bg-muted/20 rounded-lg p-3">
              <div className="space-y-2">
                {memory.learned_patterns.map((pattern: string, idx: number) => (
                  <div key={idx} className="text-sm p-2 bg-background rounded">
                    "{pattern}"
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </ScrollArea>
  )
}
