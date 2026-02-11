/**
 * Agent Logic Preview Component
 *
 * Visualizes the agent's execution logic based on user configuration:
 * - Trigger conditions
 * - Data inputs (devices + extensions)
 * - Processing logic
 * - Available actions (device commands + extension commands)
 * - Expected outputs
 */

import React, { useMemo } from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Clock,
  Zap,
  Activity,
  BarChart3,
  Target,
  Puzzle,
  ArrowDown,
  ArrowRight,
  AlertTriangle,
  CheckCircle2,
  Info,
  Sparkles,
  Eye,
  Wrench,
  Bell,
  Split,
} from "lucide-react"
// Types are defined inline to avoid circular dependencies
export interface SelectedResource {
  id: string
  name: string
  type: 'device' | 'extension'
  deviceType?: string
  allMetrics: Array<{
    name: string
    display_name: string
    unit?: string
    data_type?: string
    source: 'device' | 'extension'
    extensionId?: string
  }>
  allCommands: Array<{
    name: string
    display_name: string
    description?: string
    source: 'device' | 'extension'
    extensionId?: string
  }>
  selectedMetrics: Set<string>
  selectedCommands: Set<string>
}

export interface AgentLogicPreviewProps {
  // Basic info
  name: string
  userPrompt: string

  // Schedule
  scheduleType: 'interval' | 'daily' | 'weekly' | 'event'
  intervalValue?: number
  scheduleHour?: number
  scheduleMinute?: number
  selectedWeekdays?: number[]
  eventConfig?: {
    type: 'device.metric' | 'manual'
    deviceId?: string
  }

  // Resources
  selectedResources: SelectedResource[]

  // Validation state
  isValid: boolean
}

type IntentType = 'monitor' | 'control' | 'analysis' | 'unknown'

interface ParsedIntent {
  type: IntentType
  confidence: number
  keywords: string[]
  triggerConditions: string[]
  targetDevices: string[]
  targetMetrics: string[]
  targetCommands: string[]
  alertChannels: string[]
}

// Parse intent from user prompt
function parseIntent(prompt: string): ParsedIntent {
  const lower = prompt.toLowerCase()

  // Detect intent type
  const monitorKeywords = ['monitor', 'watch', 'track', 'observe', 'check', 'detect', '监控', '监视', '检测', '检查', '观察', 'track']
  const controlKeywords = ['control', 'turn', 'switch', 'adjust', 'regulate', 'open', 'close', 'control', '控制', '打开', '关闭', '调节', '开关', '启动', 'stop']
  const analysisKeywords = ['analyze', 'report', 'trend', 'pattern', 'summary', 'statistics', '分析', '报告', '趋势', '模式', '统计', '汇总', 'summarize']
  const alertKeywords = ['alert', 'notify', 'warn', 'send', 'push', 'message', '告警', '通知', '发送', '警告', '提醒', 'alert']

  const monitorCount = monitorKeywords.filter(k => lower.includes(k)).length
  const controlCount = controlKeywords.filter(k => lower.includes(k)).length
  const analysisCount = analysisKeywords.filter(k => lower.includes(k)).length
  const alertCount = alertKeywords.filter(k => lower.includes(k)).length

  let type: IntentType = 'unknown'
  let confidence = 0

  if (monitorCount > 0 || alertCount > 0) {
    type = 'monitor'
    confidence = Math.max(monitorCount, alertCount) * 0.3
  }
  if (controlCount > 0) {
    if (type === 'monitor' && alertCount > 0) {
      // monitor + alert + control = control
      type = 'control'
      confidence = Math.max(confidence, controlCount * 0.4)
    } else if (controlCount >= 2) {
      type = 'control'
      confidence = controlCount * 0.4
    }
  }
  if (analysisCount >= 2) {
    type = 'analysis'
    confidence = analysisCount * 0.35
  }

  // Extract trigger conditions
  const triggerConditions: string[] = []
  const whenPattern = /when|if|一旦|当|exceeds?|超过|below|低于|>|</gi
  const matches = prompt.match(whenPattern)
  if (matches) triggerConditions.push(...matches.slice(0, 3))

  // Extract target metrics/values
  const metricPatterns = /temperature|humidity|light|motion|battery|温度|湿度|光照|电量/gi
  const metricMatches = prompt.match(metricPatterns)
  const targetMetrics = metricMatches ? [...new Set(metricMatches.map(m => m.toLowerCase()))] : []

  return {
    type,
    confidence: Math.min(confidence, 0.95),
    keywords: [...new Set([...monitorKeywords.filter(k => lower.includes(k)), ...controlKeywords.filter(k => lower.includes(k)), ...analysisKeywords.filter(k => lower.includes(k))])],
    triggerConditions,
    targetDevices: [],
    targetMetrics,
    targetCommands: [],
    alertChannels: alertKeywords.filter(k => lower.includes(k)),
  }
}

// Intent icon component
function IntentIcon({ type, size = 20 }: { type: IntentType; size?: number }) {
  const icons = {
    monitor: <Eye className="text-blue-500" size={size} />,
    control: <Zap className="text-orange-500" size={size} />,
    analysis: <BarChart3 className="text-purple-500" size={size} />,
    unknown: <Sparkles className="text-gray-400" size={size} />,
  }
  return icons[type] || icons.unknown
}

// Intent label
function getIntentLabel(type: IntentType, t: (key: string) => string) {
  const labels = {
    monitor: t('preview.intent.monitor'),
    control: t('preview.intent.control'),
    analysis: t('preview.intent.analysis'),
    unknown: t('preview.intent.unknown'),
  }
  return labels[type] || labels.unknown
}

// Trigger card
function TriggerCard({ props }: { props: AgentLogicPreviewProps }) {
  const { t } = useTranslation('agents')

  const getTriggerText = () => {
    switch (props.scheduleType) {
      case 'interval':
        return t('preview.trigger.interval', { minutes: props.intervalValue || 5 })
      case 'daily':
        return t('preview.trigger.daily', { hour: props.scheduleHour || 9, minute: (props.scheduleMinute || 0).toString().padStart(2, '0') })
      case 'weekly':
        const days = props.selectedWeekdays || []
        const dayNames = days.map(d => ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][d])
        return t('preview.trigger.weekly', { days: dayNames.join(', ') })
      case 'event':
        if (props.eventConfig?.type === 'manual') {
          return t('preview.trigger.manual')
        }
        return t('preview.trigger.event', { device: props.eventConfig?.deviceId || 'all' })
      default:
        return '-'
    }
  }

  return (
    <div className="bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-950/30 dark:to-indigo-950/30 rounded-xl p-4 border border-blue-200 dark:border-blue-800">
      <div className="flex items-center gap-2 mb-2">
        <Clock className="h-4 w-4 text-blue-600 dark:text-blue-400" />
        <span className="text-sm font-medium text-blue-900 dark:text-blue-100">
          {t('preview.trigger.title')}
        </span>
      </div>
      <p className="text-sm text-blue-700 dark:text-blue-300">{getTriggerText()}</p>
    </div>
  )
}

// Data Input Card
function DataInputCard({ resources }: { resources: SelectedResource[] }) {
  const { t } = useTranslation('agents')

  const deviceResources = resources.filter(r => r.type === 'device')
  const extResources = resources.filter(r => r.type === 'extension')

  const totalMetrics = resources.reduce((sum, r) => sum + r.selectedMetrics.size, 0)
  const totalCommands = resources.reduce((sum, r) => sum + r.selectedCommands.size, 0)

  if (resources.length === 0) {
    return (
      <div className="bg-muted/50 rounded-xl p-4 border border-dashed border-muted-foreground/30">
        <div className="flex items-center justify-center gap-2 text-muted-foreground">
          <Target className="h-4 w-4" />
          <span className="text-sm">{t('preview.noResources')}</span>
        </div>
      </div>
    )
  }

  return (
    <div className="bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-950/30 dark:to-emerald-950/30 rounded-xl p-4 border border-green-200 dark:border-green-800">
      <div className="flex items-center gap-2 mb-3">
        <Activity className="h-4 w-4 text-green-600 dark:text-green-400" />
        <span className="text-sm font-medium text-green-900 dark:text-green-100">
          {t('preview.dataInput.title')}
        </span>
        <div className="ml-auto flex gap-1">
          {totalMetrics > 0 && (
            <Badge variant="secondary" className="text-xs">
              {totalMetrics} {t('preview.dataInput.metrics')}
            </Badge>
          )}
          {totalCommands > 0 && (
            <Badge variant="secondary" className="text-xs">
              {totalCommands} {t('preview.dataInput.commands')}
            </Badge>
          )}
        </div>
      </div>

      <div className="space-y-2">
        {/* Devices */}
        {deviceResources.length > 0 && (
          <div className="space-y-1.5">
            <div className="text-xs text-green-700 dark:text-green-300 font-medium flex items-center gap-1">
              <Target className="h-3 w-3" />
              {t('preview.dataInput.devices')} ({deviceResources.length})
            </div>
            {deviceResources.map(r => (
              <div key={r.id} className="bg-white/50 dark:bg-black/20 rounded-lg p-2">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs font-medium">{r.name}</span>
                  <Badge variant="outline" className="text-xs h-5 px-1">
                    {r.deviceType}
                  </Badge>
                </div>
                <div className="flex gap-2 text-xs text-muted-foreground">
                  {r.selectedMetrics.size > 0 && (
                    <span>{r.selectedMetrics.size} {t('preview.dataInput.metrics').toLowerCase()}</span>
                  )}
                  {r.selectedCommands.size > 0 && (
                    <span>{r.selectedCommands.size} {t('preview.dataInput.commands').toLowerCase()}</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Extensions */}
        {extResources.length > 0 && (
          <div className="space-y-1.5">
            <div className="text-xs text-green-700 dark:text-green-300 font-medium flex items-center gap-1">
              <Puzzle className="h-3 w-3" />
              {t('preview.dataInput.extensions')} ({extResources.length})
            </div>
            {extResources.map(r => (
              <div key={r.id} className="bg-white/50 dark:bg-black/20 rounded-lg p-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium">{r.name}</span>
                  <Badge variant="outline" className="text-xs h-5 px-1">
                    {t('preview.dataInput.extension')}
                  </Badge>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// Processing Card
function ProcessingCard({ intent, prompt }: { intent: ParsedIntent; prompt: string }) {
  const { t } = useTranslation('agents')

  const getProcessingText = () => {
    switch (intent.type) {
      case 'monitor':
        return t('preview.processing.monitor')
      case 'control':
        return t('preview.processing.control')
      case 'analysis':
        return t('preview.processing.analysis')
      default:
        return t('preview.processing.default')
    }
  }

  return (
    <div className="bg-gradient-to-br from-purple-50 to-violet-50 dark:from-purple-950/30 dark:to-violet-950/30 rounded-xl p-4 border border-purple-200 dark:border-purple-800">
      <div className="flex items-center gap-2 mb-3">
        <Split className="h-4 w-4 text-purple-600 dark:text-purple-400" />
        <span className="text-sm font-medium text-purple-900 dark:text-purple-100">
          {t('preview.processing.title')}
        </span>
        <IntentIcon type={intent.type} size={16} />
      </div>

      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <Badge variant="secondary" className="text-xs">
            {getIntentLabel(intent.type, t)}
          </Badge>
          {intent.confidence > 0 && (
            <span className="text-xs text-purple-600 dark:text-purple-400">
              {Math.round(intent.confidence * 100)}% {t('preview.confidence')}
            </span>
          )}
        </div>

        <p className="text-xs text-purple-700 dark:text-purple-300 line-clamp-2">
          {getProcessingText()}
        </p>

        {/* User prompt preview */}
        {prompt && (
          <div className="mt-2 p-2 bg-white/50 dark:bg-black/20 rounded-lg">
            <p className="text-xs text-muted-foreground line-clamp-3 italic">
              "{prompt.length > 100 ? prompt.slice(0, 100) + '...' : prompt}"
            </p>
          </div>
        )}
      </div>
    </div>
  )
}

// Actions Card
function ActionsCard({ resources }: { resources: SelectedResource[] }) {
  const { t } = useTranslation('agents')

  // Collect all available commands
  const deviceCommands = resources.flatMap(r =>
    r.type === 'device'
      ? Array.from(r.selectedCommands).map(c => ({
          resource: r.name,
          command: c,
          type: 'device' as const,
        }))
      : []
  )

  const extCommands = resources.flatMap(r =>
    r.type === 'extension'
      ? Array.from(r.selectedCommands).map(c => ({
          resource: r.name,
          command: c,
          type: 'extension' as const,
        }))
      : []
  )

  const allCommands = [...deviceCommands, ...extCommands]

  return (
    <div className="bg-gradient-to-br from-orange-50 to-amber-50 dark:from-orange-950/30 dark:to-amber-950/30 rounded-xl p-4 border border-orange-200 dark:border-orange-800">
      <div className="flex items-center gap-2 mb-3">
        <Wrench className="h-4 w-4 text-orange-600 dark:text-orange-400" />
        <span className="text-sm font-medium text-orange-900 dark:text-orange-100">
          {t('preview.actions.title')}
        </span>
        {allCommands.length > 0 && (
          <Badge variant="secondary" className="ml-auto text-xs">
            {allCommands.length}
          </Badge>
        )}
      </div>

      {allCommands.length === 0 ? (
        <div className="text-xs text-muted-foreground text-center py-2">
          {t('preview.actions.noCommands')}
        </div>
      ) : (
        <div className="space-y-1.5">
          {allCommands.slice(0, 5).map((cmd, i) => (
            <div key={i} className="flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1">
              {cmd.type === 'device' ? (
                <Target className="h-3 w-3 text-orange-500" />
              ) : (
                <Puzzle className="h-3 w-3 text-purple-500" />
              )}
              <span className="text-xs font-medium truncate">{cmd.resource}</span>
              <ArrowRight className="h-3 w-3 text-muted-foreground" />
              <span className="text-xs text-muted-foreground truncate">{cmd.command}</span>
            </div>
          ))}
          {allCommands.length > 5 && (
            <div className="text-xs text-muted-foreground text-center">
              +{allCommands.length - 5} {t('preview.moreCommands')}
            </div>
          )}
        </div>
      )}

      {/* Alert capability */}
      {resources.some(r => r.type === 'device') && (
        <div className="mt-3 pt-3 border-t border-orange-200 dark:border-orange-800">
          <div className="flex items-center gap-2 text-xs text-orange-700 dark:text-orange-300">
            <Bell className="h-3 w-3" />
            <span>{t('preview.actions.canAlert')}</span>
          </div>
        </div>
      )}
    </div>
  )
}

// Issue Detection
function IssueDetection({ props, intent }: { props: AgentLogicPreviewProps; intent: ParsedIntent }) {
  const { t } = useTranslation('agents')

  const issues: { type: 'warning' | 'error' | 'info'; message: string }[] = []

  // Check for missing name
  if (!props.name.trim()) {
    issues.push({ type: 'error', message: t('preview.issues.noName') })
  }

  // Check for empty prompt
  if (!props.userPrompt.trim()) {
    issues.push({ type: 'error', message: t('preview.issues.noPrompt') })
  }

  // Check for no resources
  if (props.selectedResources.length === 0) {
    issues.push({ type: 'warning', message: t('preview.issues.noResources') })
  }

  // Check for no metrics selected
  const metricCount = props.selectedResources.reduce((sum, r) => sum + r.selectedMetrics.size, 0)
  if (props.selectedResources.length > 0 && metricCount === 0) {
    issues.push({ type: 'warning', message: t('preview.issues.noMetrics') })
  }

  // Check for unclear intent
  if (props.userPrompt.length > 0 && intent.confidence < 0.3) {
    issues.push({ type: 'info', message: t('preview.issues.unclearIntent') })
  }

  // Check for control type without commands
  if (intent.type === 'control') {
    const commandCount = props.selectedResources.reduce((sum, r) => sum + r.selectedCommands.size, 0)
    if (commandCount === 0) {
      issues.push({ type: 'warning', message: t('preview.issues.controlNoCommands') })
    }
  }

  if (issues.length === 0) {
    return (
      <div className="flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
        <CheckCircle2 className="h-4 w-4" />
        <span>{t('preview.issues.none')}</span>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      {issues.map((issue, i) => (
        <div key={i} className={cn(
          "flex items-start gap-2 text-xs",
          issue.type === 'error' && "text-red-600 dark:text-red-400",
          issue.type === 'warning' && "text-orange-600 dark:text-orange-400",
          issue.type === 'info' && "text-blue-600 dark:text-blue-400"
        )}>
          {issue.type === 'error' && <AlertTriangle className="h-3 w-3 mt-0.5 shrink-0" />}
          {issue.type === 'warning' && <AlertTriangle className="h-3 w-3 mt-0.5 shrink-0" />}
          {issue.type === 'info' && <Info className="h-3 w-3 mt-0.5 shrink-0" />}
          <span>{issue.message}</span>
        </div>
      ))}
    </div>
  )
}

// Main Component
export function AgentLogicPreview({ props }: { props: AgentLogicPreviewProps }) {
  const { t } = useTranslation('agents')

  // Parse intent from prompt
  const intent = useMemo(() => parseIntent(props.userPrompt), [props.userPrompt])

  return (
    <div className="h-full flex flex-col bg-muted/30 rounded-xl p-4">
      {/* Header */}
      <div className="flex items-center gap-2 mb-4 pb-3 border-b">
        <Sparkles className="h-5 w-5 text-primary" />
        <h3 className="font-semibold">{t('preview.title')}</h3>
      </div>

      {/* Flow Diagram */}
      <ScrollArea className="flex-1 pr-4">
        <div className="space-y-3">
          {/* Trigger */}
          <TriggerCard props={props} />

          {/* Arrow */}
          <div className="flex justify-center py-1">
            <ArrowDown className="h-5 w-5 text-muted-foreground" />
          </div>

          {/* Data Input */}
          <DataInputCard resources={props.selectedResources} />

          {/* Arrow */}
          <div className="flex justify-center py-1">
            <ArrowDown className="h-5 w-5 text-muted-foreground" />
          </div>

          {/* Processing */}
          <ProcessingCard intent={intent} prompt={props.userPrompt} />

          {/* Arrow */}
          <div className="flex justify-center py-1">
            <ArrowDown className="h-5 w-5 text-muted-foreground" />
          </div>

          {/* Actions */}
          <ActionsCard resources={props.selectedResources} />

          {/* Issues */}
          <div className="pt-3 border-t">
            <div className="flex items-center gap-2 mb-2">
              <Info className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium text-muted-foreground">
                {t('preview.issues.title')}
              </span>
            </div>
            <IssueDetection props={props} intent={intent} />
          </div>
        </div>
      </ScrollArea>
    </div>
  )
}
