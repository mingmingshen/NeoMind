/**
 * EventsBar - Top notification bar for real-time events
 * Collapsible design with event summaries
 */

import { useState, useMemo } from "react"
import { useEvents } from "@/hooks/useEvents"
import type { NeoTalkEvent } from "@/lib/events"
import { cn } from "@/lib/utils"
import {
  Activity,
  ChevronDown,
  ChevronUp,
  Cpu,
  Bell,
  Sparkles,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"

interface EventsBarProps {
  className?: string
}

// Get icon for event type
function getEventIcon(type: string) {
  if (type.toLowerCase().includes("device")) return Cpu
  if (type.toLowerCase().includes("alert")) return Bell
  if (type.toLowerCase().includes("rule")) return Activity
  if (type.toLowerCase().includes("llm") || type.toLowerCase().includes("agent")) return Sparkles
  return Activity
}

// Format event to summary text
function getEventSummary(event: NeoTalkEvent): string {
  const data = event.data as Record<string, unknown> | undefined

  switch (event.type) {
    case "DeviceOnline":
      return `${data?.device_id || "设备"} 上线`
    case "DeviceOffline":
      return `${data?.device_id || "设备"} 离线`
    case "DeviceCommandResult":
      return `${data?.device_id || "设备"} 命令${data?.success ? "成功" : "失败"}`
    case "RuleTriggered":
      return `规则 ${data?.rule_id || ""} 触发`
    case "RuleExecuted":
      return `规则 ${data?.rule_id || ""} 执行完成`
    case "AlertCreated":
      return `新告警: ${data?.title || data?.message || "未知"}`
    case "AlertAcknowledged":
      return `告警已确认: ${data?.title || data?.alert_id || ""}`
    case "LlmResponse":
      return `AI 回复完成`
    default:
      return event.type
  }
}

// Format timestamp
function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  return date.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

export function EventsBar({ className }: EventsBarProps) {
  const [expanded, setExpanded] = useState(false)
  const { events, isConnected } = useEvents({
    category: "all",
    enabled: true,
  })

  // Get latest events (reverse order, newest first)
  const latestEvents = useMemo(() => {
    return [...events].reverse().slice(0, 20)
  }, [events])

  // Get display events for collapsed view (last 5)
  const displayEvents = useMemo(() => {
    return latestEvents.slice(0, 5)
  }, [latestEvents])

  // Don't render if no events and not connected
  if (!isConnected && events.length === 0) {
    return null
  }

  return (
    <div
      className={cn(
        "bg-muted/50 backdrop-blur transition-all z-40",
        expanded ? "h-64" : "h-9",
        className
      )}
      style={{ transitionDuration: "var(--duration-slow)" }}
    >
      {/* Collapsed Header */}
      <div className="h-9 px-4 flex items-center justify-between">
        <div className="flex items-center gap-2 flex-1 overflow-hidden">
          {/* Status indicator */}
          <div className="relative flex-shrink-0">
            <Activity className="h-4 w-4 text-muted-foreground" />
            {isConnected && (
              <span className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-foreground rounded-full animate-pulse" />
            )}
          </div>

          {/* Scrolling events */}
          <div className="flex-1 overflow-hidden">
            {displayEvents.length > 0 ? (
              <div className="flex gap-4 text-sm text-muted-foreground whitespace-nowrap">
                {displayEvents.map((event) => {
                  const Icon = getEventIcon(event.type)
                  return (
                    <span
                      key={event.id}
                      className="flex items-center gap-1.5 animate-fade-in-up"
                    >
                      <Icon className="h-3.5 w-3.5 flex-shrink-0" />
                      <span className="truncate max-w-[200px]">
                        {getEventSummary(event)}
                      </span>
                    </span>
                  )
                })}
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">
                等待事件...
              </span>
            )}
          </div>
        </div>

        {/* Expand/Collapse button */}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setExpanded(!expanded)}
          className="h-7 px-2 text-muted-foreground hover:text-foreground"
        >
          {expanded ? (
            <ChevronUp className="h-4 w-4" />
          ) : (
            <ChevronDown className="h-4 w-4" />
          )}
        </Button>
      </div>

      {/* Expanded Content */}
      {expanded && (
        <ScrollArea className="h-[calc(100%-36px)] pt-1">
          <div className="px-2">
            {latestEvents.length > 0 ? (
              latestEvents.map((event) => {
                const Icon = getEventIcon(event.type)
                return (
                  <div
                    key={event.id}
                    className="px-2 py-2 flex items-center gap-3 rounded-lg hover:bg-muted/50 text-sm transition-colors"
                  >
                    <span className="text-xs text-muted-foreground w-16 font-mono flex-shrink-0">
                      {formatTime(event.timestamp)}
                    </span>
                    <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center flex-shrink-0">
                      <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                    </div>
                    <span className="flex-1 truncate text-foreground">
                      {getEventSummary(event)}
                    </span>
                    <span className="text-xs text-muted-foreground px-2 py-0.5 rounded bg-muted">
                      {event.type}
                    </span>
                  </div>
                )
              })
            ) : (
              <div className="py-8 text-center text-muted-foreground text-sm">
                暂无事件
              </div>
            )}
          </div>
        </ScrollArea>
      )}
    </div>
  )
}
