/**
 * Rules List - Using ResponsiveTable for consistent styling
 */

import { useState } from "react"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import { ResponsiveTable } from "@/components/shared"
import { Edit, Play, Trash2, Bell, FileText, FlaskConical, AlertTriangle, Sparkles, Clock, CheckCircle2, Timer, Zap } from "lucide-react"
import { useTranslation } from "react-i18next"
import type { Rule, RuleAction } from "@/types"
import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils/format"

interface RulesListProps {
  rules: Rule[]
  loading: boolean
  paginatedRules?: Rule[]
  page?: number
  onPageChange?: (page: number) => void
  onEdit: (rule: Rule) => void
  onDelete: (rule: Rule) => void
  onToggleStatus: (rule: Rule) => void
  onExecute: (rule: Rule) => void
}

// Action configuration for display
const ACTION_CONFIG: Record<string, { icon: typeof Zap; label: string; color: string }> = {
  Execute: { icon: Zap, label: 'automation:ruleBuilder.actionType.execute', color: 'text-yellow-700 bg-yellow-50 border-yellow-200 dark:text-yellow-400 dark:bg-yellow-950/30 dark:border-yellow-800' },
  Notify: { icon: Bell, label: 'automation:ruleBuilder.actionType.notify', color: 'text-blue-700 bg-blue-50 border-blue-200 dark:text-blue-400 dark:bg-blue-950/30 dark:border-blue-800' },
  Log: { icon: FileText, label: 'automation:ruleBuilder.actionType.log', color: 'text-gray-700 bg-gray-50 border-gray-200 dark:text-gray-400 dark:bg-gray-800 dark:border-gray-700' },
  Set: { icon: FlaskConical, label: 'automation:ruleBuilder.actionType.set', color: 'text-purple-700 bg-purple-50 border-purple-200 dark:text-purple-400 dark:bg-purple-950/30 dark:border-purple-800' },
  Delay: { icon: Timer, label: 'automation:ruleBuilder.actionType.delay', color: 'text-orange-700 bg-orange-50 border-orange-200 dark:text-orange-400 dark:bg-orange-950/30 dark:border-orange-800' },
  CreateAlert: { icon: AlertTriangle, label: 'automation:ruleBuilder.actionType.createAlert', color: 'text-red-700 bg-red-50 border-red-200 dark:text-red-400 dark:bg-red-950/30 dark:border-red-800' },
  HttpRequest: { icon: FlaskConical, label: 'HTTP', color: 'text-green-700 bg-green-50 border-green-200 dark:text-green-400 dark:bg-green-950/30 dark:border-green-800' },
}

export const ITEMS_PER_PAGE = 10

// Format condition for display
function formatConditionDisplay(rule: Rule): { text: string; full: string } {
  if (!rule.dsl) return { text: '-', full: '-' }

  const whenMatch = rule.dsl.match(/WHEN\s+(.+?)(?:\nFOR|\nDO|$)/s)
  if (whenMatch) {
    let condition = whenMatch[1].trim()
    // Simplify common patterns for better readability
    condition = condition
      .replace(/\s+/g, ' ')  // Collapse multiple spaces
      .replace(/ > /g, '>')  // Clean up operators
      .replace(/ < /g, '<')
      .replace(/ >= /g, '>=')
      .replace(/ <= /g, '<=')
      .replace(/ == /g, '=')
      .replace(/ != /g, '≠')

    const full = condition
    // Truncate for display
    let text = condition
    if (text.length > 45) {
      text = text.substring(0, 42) + '...'
    }
    return { text, full }
  }
  return { text: '-', full: '-' }
}

// Parse FOR clause to get duration
function parseForClause(rule: Rule): { duration: number; unit: string } | null {
  if (!rule.dsl) return null
  const forMatch = rule.dsl.match(/FOR\s+(\d+)(ms|s|m|h)\b/)
  if (forMatch) {
    const duration = parseInt(forMatch[1], 10)
    const unit = forMatch[2]
    return { duration, unit }
  }
  return null
}

// Check if rule has FOR clause
function hasForClause(rule: Rule): boolean {
  return rule.dsl?.includes('\nFOR ') || false
}

// Parse actions from DSL
function parseActionsFromDSL(dsl?: string): RuleAction[] {
  if (!dsl) return []
  const actions: RuleAction[] = []
  const doMatch = dsl.match(/\nDO\n(.*?)\nEND/s)
  if (!doMatch) return actions

  const actionLines = doMatch[1].trim().split('\n').map(l => l.trim().replace(/^ {4}/, ''))

  for (const line of actionLines) {
    if (!line) continue

    const notifyMatch = line.match(/^NOTIFY\s+"(.+)"$/)
    if (notifyMatch) {
      actions.push({ type: 'Notify', message: notifyMatch[1] })
      continue
    }

    const execMatch = line.match(/^EXECUTE\s+([^.]+)\.(\w+)(?:\((.*)\))?$/)
    if (execMatch) {
      const [, deviceId, command, paramsStr] = execMatch
      const params: Record<string, string> = {}
      if (paramsStr) {
        paramsStr.split(', ').forEach(p => {
          const [k, v] = p.split('=')
          if (k && v) params[k] = v
        })
      }
      actions.push({ type: 'Execute', device_id: deviceId, command, params })
      continue
    }

    const logMatch = line.match(/^LOG\s+(\w+),\s+"(.+)"$/)
    if (logMatch) {
      actions.push({ type: 'Log', level: logMatch[1], message: logMatch[2] })
      continue
    }

    const setMatch = line.match(/^SET\s+([^.]+)\.([^=]+)\s*=\s*(.+)$/)
    if (setMatch) {
      actions.push({
        type: 'Set',
        device_id: setMatch[1],
        property: setMatch[2].trim(),
        value: setMatch[3].trim().replace(/^"|"$/g, '')
      })
      continue
    }

    const delayMatch = line.match(/^DELAY\s+(\d+)ms$/)
    if (delayMatch) {
      actions.push({ type: 'Delay', duration: parseInt(delayMatch[1], 10) })
      continue
    }

    const alertMatch = line.match(/^ALERT\s+"(.+)"\s+"(.+)"\s+(\w+)$/)
    if (alertMatch) {
      actions.push({
        type: 'CreateAlert',
        title: alertMatch[1],
        message: alertMatch[2],
        severity: alertMatch[3] as 'info' | 'warning' | 'error' | 'critical'
      })
      continue
    }

    const httpMatch = line.match(/^HTTP\s+(GET|POST|PUT|DELETE|PATCH)\s+(.+)$/)
    if (httpMatch) {
      actions.push({
        type: 'HttpRequest',
        method: httpMatch[1] as 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH',
        url: httpMatch[2]
      })
      continue
    }
  }

  return actions
}

export function RulesList({
  rules,
  loading,
  paginatedRules: propsPaginatedRules,
  page: propsPage,
  onPageChange,
  onEdit,
  onDelete,
  onToggleStatus,
  onExecute,
}: RulesListProps) {
  const { t } = useTranslation(['common', 'automation'])
  const [internalPage, setInternalPage] = useState(1)

  // Use props if provided, otherwise use internal state (backward compatibility)
  const page = propsPage ?? internalPage
  const setPage = onPageChange ?? setInternalPage

  const totalPages = Math.ceil(rules.length / ITEMS_PER_PAGE) || 1
  const startIndex = (page - 1) * ITEMS_PER_PAGE
  const endIndex = startIndex + ITEMS_PER_PAGE
  const paginatedRules = propsPaginatedRules ?? rules.slice(startIndex, endIndex)

  return (
    <ResponsiveTable
      columns={[
        {
          key: 'index',
          label: '#',
          width: 'w-10',
          align: 'center',
        },
        {
          key: 'name',
          label: t('automation:ruleName'),
        },
        {
          key: 'trigger',
          label: t('automation:trigger'),
        },
        {
          key: 'actions',
          label: t('automation:ruleBuilder.executeActions'),
          align: 'center',
        },
        {
          key: 'lastTriggered',
          label: t('automation:lastTriggered'),
          align: 'center',
        },
        {
          key: 'status',
          label: t('automation:status'),
          align: 'center',
        },
      ]}
      data={paginatedRules as unknown as Record<string, unknown>[]}
      rowKey={(rule) => (rule as unknown as Rule).id}
      loading={loading}
      getRowClassName={(rowData) => {
        const rule = rowData as unknown as Rule
        return cn(!rule.enabled && "opacity-50")
      }}
      renderCell={(columnKey, rowData) => {
        const rule = rowData as unknown as Rule
        const index = paginatedRules.indexOf(rule)

        switch (columnKey) {
          case 'index':
            return (
              <div className="flex items-center justify-center">
                <span className="text-xs text-muted-foreground font-medium">
                  {startIndex + index + 1}
                </span>
              </div>
            )

          case 'name':
            return (
              <div className="flex items-center gap-3">
                <div className={cn(
                  "w-9 h-9 rounded-lg flex items-center justify-center transition-colors shrink-0",
                  rule.enabled ? "bg-amber-500/10 text-amber-600" : "bg-muted text-muted-foreground"
                )}>
                  <Sparkles className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="font-medium text-sm truncate">{rule.name}</div>
                  <div className="text-xs text-muted-foreground line-clamp-1">
                    {rule.description || '-'}
                  </div>
                </div>
              </div>
            )

          case 'trigger': {
            const condition = formatConditionDisplay(rule)
            const forClause = parseForClause(rule)

            return (
              <div className="flex items-center gap-2">
                <div className="flex flex-col gap-1 min-w-0 flex-1">
                  <div className="text-xs font-mono bg-muted/60 px-2.5 py-1.5 rounded-md border border-border/50 truncate" title={condition.full}>
                    <span className={condition.text === '-' ? 'text-muted-foreground' : 'text-foreground'}>
                      {condition.text}
                    </span>
                  </div>
                  {forClause && (
                    <div className="flex items-center gap-1.5">
                      <Badge variant="outline" className="text-xs gap-1 px-2 py-0 text-blue-600 border-blue-200 dark:border-blue-800">
                        <Timer className="h-3 w-3" />
                        {forClause.duration}{forClause.unit}
                      </Badge>
                    </div>
                  )}
                </div>
              </div>
            )
          }

          case 'actions': {
            const actions = rule.actions && rule.actions.length > 0
              ? rule.actions
              : parseActionsFromDSL(rule.dsl)
            const actionsCount = actions.length
            const firstActions = actions.slice(0, 2)

            return actionsCount === 0 ? (
              <div className="flex items-center justify-center">
                <span className="text-muted-foreground text-sm">-</span>
              </div>
            ) : (
              <div className="flex items-center">
                <div className="flex flex-wrap gap-1">
                  {firstActions.map((action, i) => {
                    const config = ACTION_CONFIG[action.type] || ACTION_CONFIG.Execute
                    const Icon = config.icon
                    return (
                      <Badge
                        key={i}
                        variant="outline"
                        className={cn("text-xs gap-1", config.color)}
                      >
                        <Icon className="h-3 w-3" />
                        {t(config.label)}
                      </Badge>
                    )
                  })}
                  {actionsCount > 2 && (
                    <Badge variant="outline" className="text-xs bg-muted">
                      +{actionsCount - 2}
                    </Badge>
                  )}
                </div>
              </div>
            )
          }

          case 'lastTriggered': {
            const hasTriggered = rule.last_triggered && rule.last_triggered !== '-' && rule.last_triggered !== 0
            const triggerCount = rule.trigger_count || 0

            return (
              <div className="flex items-center justify-center">
                {!hasTriggered ? (
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span>{t('automation:never', 'Never')}</span>
                  </div>
                ) : (
                  <div className="flex flex-col items-center gap-0.5">
                    <div className="flex items-center gap-1.5 text-xs">
                      <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />
                      <span>{formatTimestamp(rule.last_triggered)}</span>
                    </div>
                    {triggerCount > 1 && (
                      <span className="text-xs text-muted-foreground">
                        {t('automation:triggeredTimes', '{{count}} times', { count: triggerCount })}
                      </span>
                    )}
                  </div>
                )}
              </div>
            )
          }

          case 'status':
            return (
              <div className="flex items-center justify-center gap-2">
                <Switch
                  checked={rule.enabled}
                  onCheckedChange={() => onToggleStatus(rule)}
                  className="scale-90"
                />
                <Badge variant="outline" className={cn(
                  "text-xs gap-1 hidden sm:flex",
                  rule.enabled
                    ? "text-green-700 bg-green-50 border-green-200 dark:text-green-400 dark:bg-green-950/30 dark:border-green-800"
                    : "text-gray-700 bg-gray-50 border-gray-200 dark:text-gray-400 dark:bg-gray-800 dark:border-gray-700"
                )}>
                  <CheckCircle2 className="h-3 w-3" />
                  {rule.enabled ? t('automation:statusEnabled') : t('automation:statusDisabled')}
                </Badge>
              </div>
            )

          default:
            return null
        }
      }}
      actions={[
        {
          label: t('common:edit'),
          icon: <Edit className="h-4 w-4" />,
          onClick: (rowData) => {
            const rule = rowData as unknown as Rule
            onEdit(rule)
          },
        },
        {
          label: t('automation:execute'),
          icon: <Play className="h-4 w-4" />,
          onClick: (rowData) => {
            const rule = rowData as unknown as Rule
            onExecute(rule)
          },
        },
        {
          label: t('common:delete'),
          icon: <Trash2 className="h-4 w-4" />,
          variant: 'destructive',
          onClick: (rowData) => {
            const rule = rowData as unknown as Rule
            onDelete(rule)
          },
        },
      ]}
    />
  )
}
