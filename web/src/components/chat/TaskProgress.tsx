// TaskProgress component - displays multi-turn task progress
import { CheckCircle2, Circle, Loader2, AlertCircle } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Progress } from "@/components/ui/progress"
import { cn } from "@/lib/utils"

export type TaskStepType = "gather_info" | "confirm" | "execute"
export type TaskStatus = "in_progress" | "completed" | "cancelled" | "failed"

export interface TaskStep {
  step_type: TaskStepType
  description: string
  prompt?: string
  details?: string
  action?: string
}

export interface TaskProgressData {
  task_id: string
  current_step: number
  total_steps: number
  steps: TaskStep[]
  status: TaskStatus
}

interface TaskProgressProps {
  task: TaskProgressData
  className?: string
}

// Step type configuration
const STEP_TYPE_CONFIG: Record<TaskStepType, {
  label: string
  icon: string
  color: string
}> = {
  gather_info: {
    label: "收集信息",
    icon: "📝",
    color: "text-blue-500"
  },
  confirm: {
    label: "确认",
    icon: "✓",
    color: "text-amber-500"
  },
  execute: {
    label: "执行",
    icon: "⚡",
    color: "text-green-500"
  }
}

export function TaskProgress({ task, className }: TaskProgressProps) {
  const progressPercent = task.total_steps > 0
    ? (task.current_step / task.total_steps) * 100
    : 0

  const getStepStatus = (index: number): "pending" | "active" | "completed" => {
    if (index < task.current_step) return "completed"
    if (index === task.current_step && task.status === "in_progress") return "active"
    return "pending"
  }

  const getStepIcon = (status: "pending" | "active" | "completed") => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />
      case "active":
        return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />
      case "pending":
        return <Circle className="h-4 w-4 text-gray-300" />
    }
  }

  return (
    <div className={cn("task-progress space-y-3", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">任务进度</span>
          <Badge variant="outline" className="text-xs">
            {task.current_step}/{task.total_steps}
          </Badge>
        </div>
        {task.status === "completed" && (
          <Badge className="bg-green-500 hover:bg-green-600 text-xs">
            已完成
          </Badge>
        )}
        {task.status === "failed" && (
          <Badge variant="destructive" className="text-xs">
            失败
          </Badge>
        )}
        {task.status === "cancelled" && (
          <Badge variant="secondary" className="text-xs">
            已取消
          </Badge>
        )}
      </div>

      {/* Progress Bar */}
      <div className="space-y-1">
        <Progress value={progressPercent} className="h-2" />
        <div className="text-xs text-muted-foreground text-right">
          {Math.round(progressPercent)}% 完成
        </div>
      </div>

      {/* Steps List */}
      <div className="space-y-2">
        {task.steps.map((step, index) => {
          const status = getStepStatus(index)
          const config = STEP_TYPE_CONFIG[step.step_type]

          return (
            <div
              key={index}
              className={cn(
                "flex items-start gap-3 p-3 rounded-md border transition-colors",
                status === "active" && "bg-blue-50 border-blue-200",
                status === "completed" && "bg-green-50/50 border-green-200/50",
                status === "pending" && "bg-muted/30 border-border/50"
              )}
            >
              {/* Step Icon */}
              <div className={cn(
                "flex-shrink-0 mt-0.5",
                config.color
              )}>
                {getStepIcon(status)}
              </div>

              {/* Step Content */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">{step.description}</span>
                  <Badge variant="outline" className="text-xs">
                    {config.label}
                  </Badge>
                </div>

                {step.details && (
                  <p className="text-xs text-muted-foreground mt-1">
                    {step.details}
                  </p>
                )}

                {status === "active" && step.prompt && (
                  <div className="mt-2 p-2 rounded bg-white border border-blue-200">
                    <p className="text-xs text-blue-700">{step.prompt}</p>
                  </div>
                )}
              </div>

              {/* Step Number Badge */}
              <div className={cn(
                "flex-shrink-0 w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium",
                status === "completed" && "bg-green-500 text-white",
                status === "active" && "bg-blue-500 text-white",
                status === "pending" && "bg-gray-200 text-gray-600"
              )}>
                {index + 1}
              </div>
            </div>
          )
        })}
      </div>

      {/* Status Messages */}
      {task.status === "in_progress" && (
        <div className="flex items-center gap-2 p-3 rounded-md bg-blue-50 border border-blue-200">
          <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />
          <p className="text-sm text-blue-700">
            正在处理第 {task.current_step + 1} 步...
          </p>
        </div>
      )}

      {task.status === "completed" && (
        <div className="flex items-center gap-2 p-3 rounded-md bg-green-50 border border-green-200">
          <CheckCircle2 className="h-4 w-4 text-green-500" />
          <p className="text-sm text-green-700">
            任务已成功完成！
          </p>
        </div>
      )}

      {task.status === "failed" && (
        <div className="flex items-center gap-2 p-3 rounded-md bg-red-50 border border-red-200">
          <AlertCircle className="h-4 w-4 text-red-500" />
          <p className="text-sm text-red-700">
            任务执行失败，请重试
          </p>
        </div>
      )}
    </div>
  )
}

// Compact horizontal progress indicator
export function CompactTaskProgress({ task, className }: TaskProgressProps) {
  const progressPercent = task.total_steps > 0
    ? (task.current_step / task.total_steps) * 100
    : 0

  return (
    <div className={cn("flex items-center gap-3", className)}>
      {/* Progress indicators */}
      <div className="flex items-center gap-1">
        {task.steps.map((_, index) => {
          const isCompleted = index < task.current_step
          const isCurrent = index === task.current_step && task.status === "in_progress"

          return (
            <div
              key={index}
              className={cn(
                "w-2 h-2 rounded-full transition-colors",
                isCompleted && "bg-green-500",
                isCurrent && "bg-blue-500 animate-pulse",
                !isCompleted && !isCurrent && "bg-gray-300"
              )}
            />
          )
        })}
      </div>

      {/* Text */}
      <span className="text-xs text-muted-foreground">
        {task.current_step}/{task.total_steps}
      </span>

      {/* Progress bar */}
      <div className="flex-1 h-1.5 bg-gray-200 rounded-full overflow-hidden">
        <div
          className={cn(
            "h-full transition-all duration-300",
            task.status === "completed" && "bg-green-500",
            task.status === "failed" && "bg-red-500",
            task.status !== "completed" && task.status !== "failed" && "bg-blue-500"
          )}
          style={{ width: `${progressPercent}%` }}
        />
      </div>
    </div>
  )
}

// Step-by-step wizard view
export function TaskWizard({ task, className }: TaskProgressProps) {
  const currentStepData = task.steps[task.current_step]

  return (
    <div className={cn("task-wizard space-y-4", className)}>
      {/* Progress Header */}
      <div className="flex items-center justify-between">
        <h3 className="font-medium">创建自动化</h3>
        <CompactTaskProgress task={task} />
      </div>

      {/* Current Step */}
      {task.status === "in_progress" && currentStepData && (
        <div className="p-4 rounded-md border bg-card">
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xs text-muted-foreground">第 {task.current_step + 1} 步</span>
            <Badge variant="outline" className="text-xs">
              {STEP_TYPE_CONFIG[currentStepData.step_type].label}
            </Badge>
          </div>

          <h4 className="font-medium mb-2">{currentStepData.description}</h4>

          {currentStepData.prompt && (
            <p className="text-sm text-muted-foreground mb-4">
              {currentStepData.prompt}
            </p>
          )}

          {currentStepData.details && (
            <div className="p-3 rounded-md bg-muted/50 text-sm">
              {currentStepData.details}
            </div>
          )}
        </div>
      )}

      {/* Completed state */}
      {task.status === "completed" && (
        <div className="p-6 rounded-md border bg-green-50 text-center">
          <CheckCircle2 className="h-12 w-12 text-green-500 mx-auto mb-3" />
          <h3 className="font-medium text-green-900 mb-1">任务完成</h3>
          <p className="text-sm text-green-700">
            自动化已成功创建！
          </p>
        </div>
      )}
    </div>
  )
}
