/**
 * Agent List Panel - Left sidebar of Agents page
 *
 * Shows all agents in a compact list format with selection capability.
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
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { AiAgent, AiAgentDetail } from "@/types"

interface AgentListPanelProps {
  agents: AiAgent[]
  loading: boolean
  selectedAgent: AiAgentDetail | null
  onSelectAgent: (agent: AiAgent) => void
  onCreate: () => void
  onEdit: (agent: AiAgent) => void
  onDelete: (agent: AiAgent) => void
  onToggleStatus: (agent: AiAgent) => void
  onExecute: (agent: AiAgent) => void
  onViewMemory: (agentId: string, agentName: string) => void
}

// Status configuration
const STATUS_CONFIG: Record<string, { label: string; icon: typeof CheckCircle2; color: string }> = {
  Active: { label: '运行中', icon: CheckCircle2, color: 'text-green-500' },
  Paused: { label: '已暂停', icon: XCircle, color: 'text-muted-foreground' },
  Error: { label: '错误', icon: XCircle, color: 'text-red-500' },
  Executing: { label: '执行中', icon: Loader2, color: 'text-blue-500' },
}

export function AgentListPanel({
  agents,
  loading,
  selectedAgent,
  onSelectAgent,
  onCreate,
  onEdit,
  onDelete,
  onToggleStatus,
  onExecute,
  onViewMemory,
}: AgentListPanelProps) {
  const { t } = useTranslation(['common', 'agents'])

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (agents.length === 0) {
    return (
      <div className="text-center py-12 px-4">
        <Bot className="h-10 w-10 mx-auto mb-3 text-muted-foreground/30" />
        <p className="text-sm text-muted-foreground mb-4">{t('agents:noAgents')}</p>
        <Button onClick={onCreate} variant="outline" size="sm">
          创建第一个智能体
        </Button>
      </div>
    )
  }

  return (
    <div className="py-2">
      {agents.map((agent) => {
        const isSelected = selectedAgent?.id === agent.id
        const statusConfig = STATUS_CONFIG[agent.status] || STATUS_CONFIG.Paused

        return (
          <div
            key={agent.id}
            onClick={() => onSelectAgent(agent)}
            className={cn(
              "group relative px-3 py-2.5 cursor-pointer transition-colors",
              isSelected ? "bg-accent" : "hover:bg-muted/30"
            )}
          >
            {/* Selection indicator bar */}
            {isSelected && (
              <div className="absolute left-0 top-2 bottom-2 w-0.5 bg-primary rounded-r-full" />
            )}

            <div className="flex items-center gap-3">
              {/* Icon with executing indicator */}
              <div className="relative shrink-0">
                <div className={cn(
                  "w-7 h-7 rounded flex items-center justify-center",
                  isSelected || agent.status === 'Active' || agent.status === 'Executing'
                    ? "bg-purple-500/20 text-purple-500"
                    : "bg-muted text-muted-foreground"
                )}>
                  <Bot className="h-3.5 w-3.5" />
                </div>
                {agent.status === 'Executing' && (
                  <div className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-blue-500 rounded-full border-2 border-card" />
                )}
              </div>

              {/* Content */}
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium truncate">
                  {agent.name}
                </div>
                <div className="flex items-center gap-2 mt-0.5">
                  <statusConfig.icon className={cn(
                    "h-3 w-3",
                    statusConfig.color,
                    agent.status === 'Executing' && "animate-spin"
                  )} />
                  <span className="text-xs text-muted-foreground">
                    {agent.execution_count}
                  </span>
                </div>
              </div>

              {/* Actions - only show on hover */}
              <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <Switch
                  checked={agent.status === 'Active' || agent.status === 'Executing'}
                  onCheckedChange={() => onToggleStatus(agent)}
                  disabled={agent.status === 'Executing'}
                  className="scale-75"
                  onClick={(e) => e.stopPropagation()}
                />

                <DropdownMenu>
                  <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                    <button className="h-7 w-7 flex items-center justify-center rounded hover:bg-muted/80 transition-colors">
                      <MoreVertical className="h-3.5 w-3.5 text-muted-foreground" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-32">
                    <DropdownMenuItem onClick={() => onEdit(agent)}>
                      <Edit className="h-4 w-4 mr-2" />
                      {t('common:edit')}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => onExecute(agent)}
                      disabled={agent.status === 'Executing'}
                    >
                      <Play className="h-4 w-4 mr-2" />
                      {t('agents:execute')}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => onViewMemory(agent.id, agent.name)}
                    >
                      <Brain className="h-4 w-4 mr-2" />
                      记忆
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => onDelete(agent)}
                      className="text-destructive"
                    >
                      <Trash2 className="h-4 w-4 mr-2" />
                      {t('common:delete')}
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          </div>
        )
      })}
    </div>
  )
}
