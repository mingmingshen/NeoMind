import { Progress } from "@/components/ui/progress"
import { Activity, AlertTriangle } from "lucide-react"
import { cn } from "@/lib/utils"

interface StreamProgressProps {
  elapsed: number
  totalDuration: number
  stage: 'thinking' | 'generating' | 'tool_execution'
  warning?: string
  className?: string
}

export function StreamProgress({
  elapsed,
  totalDuration,
  stage,
  warning,
  className
}: StreamProgressProps) {
  const progress = Math.min((elapsed / totalDuration) * 100, 100)

  const stageLabels = {
    thinking: '思考中',
    generating: '生成中',
    tool_execution: '工具执行'
  }

  const isNearTimeout = progress > 80
  const barColor = isNearTimeout ? 'bg-yellow-500' : 'bg-blue-500'

  return (
    <div className={cn(
      "flex items-center gap-3 text-sm text-muted-foreground px-4 py-2 bg-muted/30 rounded-lg",
      className
    )}>
      <Activity className={cn(
        "h-4 w-4",
        stage === 'thinking' && "animate-pulse"
      )} />
      <div className="flex-1">
        <div className="flex items-center justify-between mb-1">
          <span>{stageLabels[stage]}</span>
          <span className="text-xs">{elapsed}s / {totalDuration}s</span>
        </div>
        <div className="h-1.5 bg-muted rounded-full overflow-hidden">
          <div
            className={cn("h-full transition-all duration-300", barColor)}
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>
      {warning && (
        <span className="text-yellow-600 text-xs flex items-center gap-1">
          <AlertTriangle className="h-3 w-3" />
          {warning}
        </span>
      )}
    </div>
  )
}
