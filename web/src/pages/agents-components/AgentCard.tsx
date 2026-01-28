/**
 * Agent Card - Grid item for displaying an AI Agent
 */

import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { useConfirm } from "@/components/ui/use-confirm"
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
  Activity,
  Pause,
  AlertTriangle,
  Loader2,
  Clock,
  Plus,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils/format"
import type { AiAgent } from "@/types"

interface CreateCardProps {
  onClick: () => void
}

export function CreateCard({ onClick }: CreateCardProps) {
  return (
    <button
      onClick={onClick}
      className="group flex flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed border-muted-foreground/20 hover:border-primary/50 hover:bg-muted/20 transition-all min-h-[200px]"
    >
      <div className="w-12 h-12 rounded-full bg-muted/50 flex items-center justify-center group-hover:bg-primary/20 transition-colors">
        <Plus className="h-6 w-6 text-muted-foreground group-hover:text-primary transition-colors" />
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
  onClick: () => void
}

// Status configuration
const STATUS_CONFIG: Record<string, { label: string; icon: typeof Activity; color: string; bg: string }> = {
  Active: { label: '运行中', icon: Activity, color: 'text-green-500', bg: 'bg-green-500/10' },
  Paused: { label: '已暂停', icon: Pause, color: 'text-muted-foreground', bg: 'bg-muted/50' },
  Error: { label: '错误', icon: AlertTriangle, color: 'text-red-500', bg: 'bg-red-500/10' },
  Executing: { label: '执行中', icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-500/10' },
}

export function AgentCard({
  agent,
  onToggleStatus,
  onExecute,
  onEdit,
  onDelete,
  onClick,
}: AgentCardProps) {
  const { t } = useTranslation(['common', 'agents'])
  const { confirm } = useConfirm()

  const handleToggleStatus = async () => {
    const isCurrentlyActive = agent.status === 'Active' || agent.status === 'Executing'

    const confirmed = await confirm({
      title: isCurrentlyActive ? '确认暂停智能体？' : '确认启动智能体？',
      description: isCurrentlyActive
        ? `暂停后，智能体"${agent.name}"将停止自动执行任务。`
        : `启动后，智能体"${agent.name}"将开始自动执行任务。`,
      confirmText: isCurrentlyActive ? '确认暂停' : '确认启动',
      cancelText: t('common:cancel'),
      variant: 'default',
    })

    if (confirmed) {
      onToggleStatus(agent)
    }
  }

  const statusConfig = STATUS_CONFIG[agent.status] || STATUS_CONFIG.Paused
  const StatusIcon = statusConfig.icon

  const successRate = agent.execution_count > 0
    ? Math.round((agent.success_count / agent.execution_count) * 100)
    : 0

  return (
    <div
      className={cn(
        "group relative bg-card rounded-xl border transition-all cursor-pointer p-4",
        "hover:border-primary/50 hover:shadow-md"
      )}
      onClick={onClick}
    >
      {/* Header: Icon + Name + Status */}
      <div className="flex items-start gap-3 mb-4">
        {/* Icon with status glow */}
        <div className={cn(
          "w-12 h-12 rounded-xl flex items-center justify-center shrink-0 transition-colors",
          agent.status === 'Active' && "bg-green-500/10 ring-2 ring-green-500/30",
          agent.status === 'Executing' && "bg-blue-500/10 ring-2 ring-blue-500/30",
          agent.status === 'Error' && "bg-red-500/10 ring-2 ring-red-500/30",
          agent.status === 'Paused' && "bg-muted/50"
        )}>
          <Bot className={cn(
            "h-6 w-6",
            agent.status === 'Active' && "text-green-500",
            agent.status === 'Executing' && "text-blue-500",
            agent.status === 'Error' && "text-red-500",
            agent.status === 'Paused' && "text-muted-foreground"
          )} />
        </div>

        {/* Name and Status */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="text-base font-semibold truncate">{agent.name}</h3>
            <StatusIcon className={cn(
              "h-4 w-4 shrink-0",
              statusConfig.color,
              agent.status === 'Active' && "animate-pulse",
              agent.status === 'Executing' && "animate-spin",
              agent.status === 'Error' && "animate-pulse"
            )} />
          </div>
          <p className="text-sm text-muted-foreground mt-0.5 line-clamp-1">
            {agent.user_prompt || agent.description || '暂无描述'}
          </p>
        </div>

        {/* More menu */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
            <button className="h-8 w-8 flex items-center justify-center rounded-lg hover:bg-muted transition-colors">
              <MoreVertical className="h-4 w-4 text-muted-foreground" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
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
      <div className="grid grid-cols-3 gap-3 mb-4">
        {/* Execution Count */}
        <div className="text-center p-2 rounded-lg bg-muted/30">
          <div className="text-lg font-semibold">{agent.execution_count}</div>
          <div className="text-xs text-muted-foreground">执行次数</div>
        </div>

        {/* Success Rate */}
        <div className="text-center p-2 rounded-lg bg-muted/30">
          <div className={cn(
            "text-lg font-semibold",
            successRate >= 80 ? "text-green-500" : successRate >= 50 ? "text-yellow-500" : "text-red-500"
          )}>
            {successRate}%
          </div>
          <div className="text-xs text-muted-foreground">成功率</div>
        </div>

        {/* Avg Duration */}
        <div className="text-center p-2 rounded-lg bg-muted/30">
          <div className="text-lg font-semibold">
            {agent.avg_duration_ms > 0 ? `${(agent.avg_duration_ms / 1000).toFixed(1)}s` : '-'}
          </div>
          <div className="text-xs text-muted-foreground">平均耗时</div>
        </div>
      </div>

      {/* Footer: Last Execution + Toggle */}
      <div className="flex items-center justify-between pt-2 border-t border-border/50">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Clock className="h-4 w-4" />
          <span className="truncate max-w-[120px]">{agent.last_execution_at ? formatTimestamp(agent.last_execution_at, false) : '从未执行'}</span>
        </div>

        <Switch
          checked={agent.status === 'Active' || agent.status === 'Executing'}
          onCheckedChange={handleToggleStatus}
          disabled={agent.status === 'Executing'}
          onClick={(e) => e.stopPropagation()}
        />
      </div>
    </div>
  )
}
