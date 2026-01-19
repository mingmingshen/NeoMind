/**
 * QuickActions - AI-generated action suggestions based on conversation context
 * Appears after assistant messages with suggested follow-up actions
 */

import { useMemo } from "react"
import {
  Sparkles,
  RefreshCw,
  ChevronRight,
  Settings,
  Cpu,
  AlertTriangle,
  TrendingUp,
  FileText,
  Zap
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { Message } from "@/types"

interface QuickAction {
  id: string
  label: string
  prompt: string
  icon: React.ReactNode
  category: "device" | "automation" | "analytics" | "general"
}

// Analyze message content to generate relevant quick actions
function generateQuickActions(message: Message): QuickAction[] {
  const content = (message.content || "").toLowerCase()
  const actions: QuickAction[] = []

  // Device-related suggestions
  if (content.includes("设备") || content.includes("sensor") || content.includes("温度")) {
    actions.push({
      id: "refresh-devices",
      label: "刷新设备状态",
      prompt: "刷新所有设备的状态",
      icon: <RefreshCw className="h-3.5 w-3.5" />,
      category: "device"
    })
    actions.push({
      id: "device-details",
      label: "查看设备详情",
      prompt: "显示设备的详细信息和配置",
      icon: <Cpu className="h-3.5 w-3.5" />,
      category: "device"
    })
  }

  // Automation-related suggestions
  if (content.includes("自动化") || content.includes("规则") || content.includes("工作流")) {
    actions.push({
      id: "create-automation",
      label: "创建自动化",
      prompt: "帮我创建一个新的自动化规则",
      icon: <Zap className="h-3.5 w-3.5" />,
      category: "automation"
    })
    actions.push({
      id: "list-automations",
      label: "查看所有自动化",
      prompt: "列出所有已配置的自动化规则",
      icon: <FileText className="h-3.5 w-3.5" />,
      category: "automation"
    })
  }

  // Analytics-related suggestions
  if (content.includes("数据") || content.includes("统计") || content.includes("趋势")) {
    actions.push({
      id: "show-analytics",
      label: "查看数据分析",
      prompt: "显示最近的数据分析和趋势",
      icon: <TrendingUp className="h-3.5 w-3.5" />,
      category: "analytics"
    })
  }

  // Alert-related suggestions
  if (content.includes("告警") || content.includes("异常") || content.includes("错误")) {
    actions.push({
      id: "check-alerts",
      label: "检查告警",
      prompt: "检查当前是否有未处理的告警",
      icon: <AlertTriangle className="h-3.5 w-3.5" />,
      category: "general"
    })
  }

  // Settings-related suggestions
  if (content.includes("配置") || content.includes("设置")) {
    actions.push({
      id: "open-settings",
      label: "打开设置",
      prompt: "打开系统设置页面",
      icon: <Settings className="h-3.5 w-3.5" />,
      category: "general"
    })
  }

  // Always add some generic actions if none were generated
  if (actions.length === 0) {
    actions.push(
      {
        id: "continue",
        label: "继续对话",
        prompt: "继续",
        icon: <ChevronRight className="h-3.5 w-3.5" />,
        category: "general"
      },
      {
        id: "explain-more",
        label: "详细说明",
        prompt: "请详细说明一下",
        icon: <Sparkles className="h-3.5 w-3.5" />,
        category: "general"
      }
    )
  }

  // Limit to 4 actions
  return actions.slice(0, 4)
}

interface QuickActionsProps {
  message: Message
  onActionClick: (prompt: string) => void
}

export function QuickActions({ message, onActionClick }: QuickActionsProps) {
  const actions = useMemo(() => generateQuickActions(message), [message])

  if (actions.length === 0) return null

  return (
    <div className="flex flex-wrap gap-2 mt-2">
      {actions.map((action) => (
        <button
          key={action.id}
          onClick={() => onActionClick(action.prompt)}
          className={cn(
            "flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium",
            "transition-all duration-200",
            "bg-[var(--quick-action-bg)]",
            "hover:bg-[var(--quick-action-hover)]",
            "text-[var(--quick-action-text)]",
            "border border-[var(--quick-action-border)]",
            "hover:shadow-sm hover:scale-105"
          )}
        >
          {action.icon}
          <span>{action.label}</span>
          <ChevronRight className="h-3 w-3 opacity-50" />
        </button>
      ))}
    </div>
  )
}
