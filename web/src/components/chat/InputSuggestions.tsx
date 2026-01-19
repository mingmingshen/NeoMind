/**
 * InputSuggestions - Context-aware suggestions while typing
 * Shows relevant prompts based on current input
 */

import { useMemo, useEffect, useState } from "react"
import {
  Lightbulb,
  Cpu,
  Zap,
  AlertTriangle,
  Settings,
  TrendingUp
} from "lucide-react"
import { cn } from "@/lib/utils"

interface Suggestion {
  id: string
  label: string
  prompt: string
  icon: React.ReactNode
  category: string
}

// Predefined suggestions organized by category
const SUGGESTIONS: Suggestion[] = [
  // Device queries
  {
    id: "device-list",
    label: "查看所有设备",
    prompt: "查看所有设备状态",
    icon: <Cpu className="h-4 w-4" />,
    category: "device"
  },
  {
    id: "device-online",
    label: "查看在线设备",
    prompt: "哪些设备当前在线",
    icon: <Cpu className="h-4 w-4" />,
    category: "device"
  },
  {
    id: "device-temp",
    label: "查看温度传感器",
    prompt: "查看所有温度传感器的读数",
    icon: <Cpu className="h-4 w-4" />,
    category: "device"
  },
  // Automation
  {
    id: "automation-list",
    label: "查看自动化规则",
    prompt: "查看所有自动化规则",
    icon: <Zap className="h-4 w-4" />,
    category: "automation"
  },
  {
    id: "automation-create",
    label: "创建自动化规则",
    prompt: "创建新的自动化规则",
    icon: <Zap className="h-4 w-4" />,
    category: "automation"
  },
  {
    id: "workflow-list",
    label: "查看工作流",
    prompt: "查看所有工作流",
    icon: <Zap className="h-4 w-4" />,
    category: "automation"
  },
  // Alerts
  {
    id: "alert-list",
    label: "查看告警",
    prompt: "查看当前告警",
    icon: <AlertTriangle className="h-4 w-4" />,
    category: "alert"
  },
  {
    id: "alert-create",
    label: "创建告警规则",
    prompt: "创建新的告警规则",
    icon: <AlertTriangle className="h-4 w-4" />,
    category: "alert"
  },
  // Analytics
  {
    id: "analytics-temp",
    label: "温度数据分析",
    prompt: "分析最近24小时的温度数据",
    icon: <TrendingUp className="h-4 w-4" />,
    category: "analytics"
  },
  {
    id: "analytics-trend",
    label: "查看数据趋势",
    prompt: "查看设备数据趋势",
    icon: <TrendingUp className="h-4 w-4" />,
    category: "analytics"
  },
  // Settings
  {
    id: "settings-llm",
    label: "LLM设置",
    prompt: "查看LLM后端配置",
    icon: <Settings className="h-4 w-4" />,
    category: "settings"
  },
  {
    id: "help",
    label: "帮助",
    prompt: "你能做什么",
    icon: <Lightbulb className="h-4 w-4" />,
    category: "general"
  }
]

interface InputSuggestionsProps {
  input: string
  onSelect: (prompt: string) => void
  visible: boolean
}

export function InputSuggestions({ input, onSelect, visible }: InputSuggestionsProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)

  // Filter suggestions based on input
  const filteredSuggestions = useMemo(() => {
    if (!input.trim()) {
      // Show a curated list of common actions when input is empty
      return SUGGESTIONS.filter(s =>
        ["device-list", "automation-list", "alert-list", "help"].includes(s.id)
      )
    }

    const inputLower = input.toLowerCase()
    return SUGGESTIONS.filter(s =>
      s.label.toLowerCase().includes(inputLower) ||
      s.prompt.toLowerCase().includes(inputLower)
    )
  }, [input])

  // Reset selected index when filtered suggestions change
  useEffect(() => {
    setSelectedIndex(0)
  }, [filteredSuggestions])

  // Handle keyboard navigation
  useEffect(() => {
    if (!visible) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault()
        setSelectedIndex(prev =>
          prev < filteredSuggestions.length - 1 ? prev + 1 : prev
        )
      } else if (e.key === "ArrowUp") {
        e.preventDefault()
        setSelectedIndex(prev => (prev > 0 ? prev - 1 : 0))
      } else if (e.key === "Enter" && filteredSuggestions.length > 0) {
        e.preventDefault()
        onSelect(filteredSuggestions[selectedIndex].prompt)
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [visible, filteredSuggestions, selectedIndex, onSelect])

  if (!visible || filteredSuggestions.length === 0) {
    return null
  }

  return (
    <div className="absolute bottom-full left-0 right-0 mb-2 z-10">
      <div className={cn(
        "bg-[var(--popover)] border border-[var(--border)] rounded-lg",
        "shadow-lg overflow-hidden",
        "animate-in slide-in-from-bottom-2 duration-200"
      )}>
        {/* Header */}
        {input.trim() && (
          <div className="px-3 py-2 border-b border-[var(--border)]">
            <p className="text-xs text-muted-foreground">
              按 <kbd className="px-1 py-0.5 rounded bg-[var(--muted)] text-[var(--muted-foreground)]">↑↓</kbd> 导航，
              <kbd className="px-1 py-0.5 rounded bg-[var(--muted)] text-[var(--muted-foreground)] ml-1">Enter</kbd> 选择
            </p>
          </div>
        )}

        {/* Suggestions list */}
        <div className="max-h-48 overflow-y-auto">
          {filteredSuggestions.map((suggestion, index) => (
            <button
              key={suggestion.id}
              onClick={() => onSelect(suggestion.prompt)}
              onMouseEnter={() => setSelectedIndex(index)}
              className={cn(
                "w-full flex items-center gap-3 px-3 py-2.5 text-left",
                "transition-colors duration-150",
                index === selectedIndex
                  ? "bg-[var(--accent)] text-[var(--accent-foreground)]"
                  : "hover:bg-[var(--accent)]/50"
              )}
            >
              <div className={cn(
                "flex-shrink-0 p-1.5 rounded-md",
                index === selectedIndex
                  ? "bg-[var(--accent)]/30"
                  : "bg-[var(--muted)]"
              )}>
                {suggestion.icon}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{suggestion.label}</p>
                <p className="text-xs text-muted-foreground truncate">{suggestion.prompt}</p>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
