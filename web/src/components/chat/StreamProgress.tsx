import { Progress } from "@/components/ui/progress"
import { Activity, AlertTriangle, Loader2 } from "lucide-react"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next"

interface StreamProgressProps {
  elapsed: number
  totalDuration: number
  stage: 'thinking' | 'generating' | 'tool_execution'
  warning?: string
  currentStep?: string  // Current execution plan step from Plan events
  className?: string
}

export function StreamProgress({
  elapsed,
  totalDuration,
  stage,
  warning,
  currentStep,
  className
}: StreamProgressProps) {
  const { t } = useTranslation("chat")
  const progress = Math.min((elapsed / totalDuration) * 100, 100)

  const stageConfig = {
    thinking: {
      label: t("progress.thinking"),
      icon: <Activity className="h-4 w-4 animate-pulse" />
    },
    generating: {
      label: t("progress.generating"),
      icon: <Loader2 className="h-4 w-4 animate-spin" />
    },
    tool_execution: {
      label: t("progress.toolExecution"),
      icon: <Loader2 className="h-4 w-4 animate-spin" />
    }
  }

  const currentStage = stageConfig[stage]
  const isNearTimeout = progress > 80
  const barColor = isNearTimeout ? 'bg-yellow-500' : 'bg-blue-500'

  return (
    <div className={cn(
      "flex items-center gap-3 text-sm text-muted-foreground px-4 py-2 bg-muted/30 rounded-lg",
      className
    )}>
      {currentStage.icon}
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-1">
          <span className="font-medium text-foreground">{currentStage.label}</span>
          <span className="text-xs">{elapsed}s / {totalDuration}s</span>
        </div>
        {currentStep && (
          <p className="text-xs text-muted-foreground truncate mb-1">
            {currentStep}
          </p>
        )}
        <div className="h-1.5 bg-muted rounded-full overflow-hidden">
          <div
            className={cn("h-full transition-all duration-300", barColor)}
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>
      {warning && (
        <span className="text-yellow-600 text-xs flex items-center gap-1 shrink-0">
          <AlertTriangle className="h-3 w-3" />
          {warning}
        </span>
      )}
    </div>
  )
}
