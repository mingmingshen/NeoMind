/**
 * Agent Card - Grid item for displaying an AI Agent
 */

import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Bot,
  Edit,
  Play,
  Trash2,
  MoreVertical,
  Brain,
  Activity,
  Zap,
  CheckCircle2,
  XCircle,
  Loader2,
  TrendingUp,
  Clock,
  Plus,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { AiAgent } from "@/types"

interface CreateCardProps {
  onClick: () => void
}

export function CreateCard({ onClick }: CreateCardProps) {
  return (
    <button
      onClick={onClick}
      className="group flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed border-muted-foreground/20 hover:border-primary/50 hover:bg-muted/20 transition-all min-h-[180px]"
    >
      <div className="w-10 h-10 rounded-full bg-muted/50 flex items-center justify-center group-hover:bg-primary/20 transition-colors">
        <Plus className="h-5 w-5 text-muted-foreground group-hover:text-primary transition-colors" />
      </div>
      <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors">创建智能体</span>
    </button>
  )
}

interface AgentCardProps {
  agent: AiAgent
  onToggleStatus: (agent: AiAgent) => void
  onExecute: (agent: AiAgent) => void
  onEdit: (agent: AiAgent) => void
  onDelete: (agent: AiAgent) => void
  onViewMemory: (agentId: string, agentName: string) => void
  onClick: () => void
}

// Role configuration
const ROLE_CONFIG: Record<string, { label: string; icon: typeof Activity; color: string; bg: string }> = {
  Monitor: { label: '监控', icon: Activity, color: 'text-blue-500 dark:text-blue-400', bg: 'bg-blue-500/10' },
  Executor: { label: '执行', icon: Zap, color: 'text-orange-500 dark:text-orange-400', bg: 'bg-orange-500/10' },
  Analyst: { label: '分析', icon: Brain, color: 'text-purple-500 dark:text-purple-400', bg: 'bg-purple-500/10' },
}

// Status configuration
const STATUS_CONFIG: Record<string, { label: string; icon: typeof CheckCircle2; color: string; bg: string }> = {
  Active: { label: '运行中', icon: CheckCircle2, color: 'text-green-500', bg: 'bg-green-500/10' },
  Paused: { label: '已暂停', icon: XCircle, color: 'text-muted-foreground', bg: 'bg-muted/50' },
  Error: { label: '错误', icon: XCircle, color: 'text-red-500', bg: 'bg-red-500/10' },
  Executing: { label: '执行中', icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-500/10' },
}

export function AgentCard({
  agent,
  onToggleStatus,
  onExecute,
  onEdit,
  onDelete,
  onViewMemory,
  onClick,
}: AgentCardProps) {
  const { t } = useTranslation(['common', 'agents'])

  const roleConfig = ROLE_CONFIG[agent.role] || ROLE_CONFIG.Monitor
  const RoleIcon = roleConfig.icon
  const statusConfig = STATUS_CONFIG[agent.status] || STATUS_CONFIG.Paused
  const StatusIcon = statusConfig.icon

  const successRate = agent.execution_count > 0
    ? Math.round((agent.success_count / agent.execution_count) * 100)
    : 0

  const formatLastExecution = (dateStr: string | null) => {
    if (!dateStr) return '从未执行'
    try {
      const date = new Date(dateStr)
      const now = new Date()
      const diffMs = now.getTime() - date.getTime()
      const diffMins = Math.floor(diffMs / 60000)
      const diffHours = Math.floor(diffMs / 3600000)
      const diffDays = Math.floor(diffMs / 86400000)

      if (diffMins < 1) return '刚刚'
      if (diffMins < 60) return `${diffMins}分钟前`
      if (diffHours < 24) return `${diffHours}小时前`
      if (diffDays < 7) return `${diffDays}天前`
      return date.toLocaleDateString()
    } catch {
      return '-'
    }
  }

  return (
    <div
      className={cn(
        "group relative bg-card rounded-xl border transition-all cursor-pointer p-3",
        "hover:border-primary/50 hover:shadow-md"
      )}
      onClick={onClick}
    >
      {/* Header: Icon + Name + Status */}
      <div className="flex items-start gap-2 mb-3">
        {/* Icon with status glow */}
        <div className={cn(
          "w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors",
          agent.status === 'Active' && "bg-green-500/10 ring-1 ring-green-500/20",
          agent.status === 'Executing' && "bg-blue-500/10 ring-1 ring-blue-500/20",
          agent.status === 'Error' && "bg-red-500/10 ring-1 ring-red-500/20",
          agent.status === 'Paused' && "bg-muted/50"
        )}>
          <Bot className={cn(
            "h-4 w-4",
            agent.status === 'Active' && "text-green-500",
            agent.status === 'Executing' && "text-blue-500",
            agent.status === 'Error' && "text-red-500",
            agent.status === 'Paused' && "text-muted-foreground"
          )} />
        </div>

        {/* Name and Status */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1">
            <h3 className="text-sm font-medium truncate">{agent.name}</h3>
            <StatusIcon className={cn(
              "h-3 w-3 shrink-0",
              statusConfig.color,
              agent.status === 'Executing' && "animate-spin"
            )} />
          </div>
          <div className="flex items-center gap-1 mt-0.5">
            <RoleIcon className={cn("h-2.5 w-2.5", roleConfig.color)} />
            <span className="text-[11px] text-muted-foreground">{roleConfig.label}</span>
          </div>
        </div>

        {/* More menu */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
            <button className="h-7 w-7 flex items-center justify-center rounded-lg hover:bg-muted transition-colors">
              <MoreVertical className="h-3.5 w-3.5 text-muted-foreground" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-36">
            <DropdownMenuItem onClick={(e) => { e.stopPropagation(); onEdit(agent); }}>
              <Edit className="h-4 w-4 mr-2" />
              {t('common:edit')}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => { e.stopPropagation(); onExecute(agent); }}
              disabled={agent.status === 'Executing'}
            >
              <Play className="h-4 w-4 mr-2" />
              {t('agents:execute')}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={(e) => { e.stopPropagation(); onViewMemory(agent.id, agent.name); }}>
              <Brain className="h-4 w-4 mr-2" />
              记忆
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={(e) => { e.stopPropagation(); onDelete(agent); }}
              className="text-destructive"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              {t('common:delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Stats Grid - simplified without borders */}
      <div className="grid grid-cols-3 gap-2 mb-3">
        {/* Execution Count */}
        <div className="text-center">
          <div className="text-base font-semibold">{agent.execution_count}</div>
          <div className="text-[11px] text-muted-foreground/70">执行</div>
        </div>

        {/* Success Rate */}
        <div className="text-center">
          <div className={cn(
            "text-base font-semibold flex items-center justify-center gap-0.5",
            successRate >= 80 ? "text-green-500" : successRate >= 50 ? "text-yellow-500" : "text-red-500"
          )}>
            {successRate}%
          </div>
          <div className="text-[11px] text-muted-foreground/70">成功</div>
        </div>

        {/* Avg Duration */}
        <div className="text-center">
          <div className="text-base font-semibold">
            {agent.avg_duration_ms > 0 ? `${(agent.avg_duration_ms / 1000).toFixed(1)}s` : '-'}
          </div>
          <div className="text-[11px] text-muted-foreground/70">耗时</div>
        </div>
      </div>

      {/* Footer: Last Execution + Toggle */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1 text-[11px] text-muted-foreground">
          <Clock className="h-2.5 w-2.5" />
          <span className="truncate max-w-[100px]">{formatLastExecution(agent.last_execution_at)}</span>
        </div>

        <Switch
          checked={agent.status === 'Active' || agent.status === 'Executing'}
          onCheckedChange={() => onToggleStatus(agent)}
          disabled={agent.status === 'Executing'}
          onClick={(e) => e.stopPropagation()}
          className="scale-75"
        />
      </div>
    </div>
  )
}
