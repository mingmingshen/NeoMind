// IntentIndicator component - displays AI intent classification
import { Badge } from "@/components/ui/badge"
import {
  Target,
  Zap,
  Brain,
  Settings,
  Send,
  FileText,
  HelpCircle,
  AlertTriangle,
  CheckCircle2
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next"

// Intent types matching Rust backend
export type IntentCategory =
  | "query_data"
  | "analyze_data"
  | "control_device"
  | "create_automation"
  | "send_message"
  | "summarize_info"
  | "clarify"
  | "out_of_scope"

export type ProcessingStrategy =
  | "fast_path"
  | "standard"
  | "quality"
  | "multi_turn"
  | "fallback"

export type EntityType =
  | "device"
  | "location"
  | "value"
  | "time_range"
  | "action"
  | "unknown"

export interface Entity {
  entity_type: EntityType
  value: string
  confidence: number
}

export interface IntentClassification {
  intent: IntentCategory
  sub_type: string
  confidence: number
  entities: Entity[]
  strategy: ProcessingStrategy
  needs_followup: boolean
  followup_prompt?: string
  capability_statement?: string
}

interface IntentIndicatorProps {
  classification: IntentClassification
  isStreaming?: boolean
  showDetails?: boolean
}

// Helper function to get intent config
function getIntentConfig(intent: IntentCategory, t: (key: string) => string) {
  const configs: Record<IntentCategory, {
    key: string
    icon: typeof Target
    color: string
  }> = {
    query_data: { key: "intent.queryData", icon: Target, color: "text-blue-500" },
    analyze_data: { key: "intent.analyzeData", icon: Brain, color: "text-purple-500" },
    control_device: { key: "intent.controlDevice", icon: Settings, color: "text-green-500" },
    create_automation: { key: "intent.createAutomation", icon: Zap, color: "text-amber-500" },
    send_message: { key: "intent.sendMessage", icon: Send, color: "text-cyan-500" },
    summarize_info: { key: "intent.summarize", icon: FileText, color: "text-indigo-500" },
    clarify: { key: "intent.clarify", icon: HelpCircle, color: "text-orange-500" },
    out_of_scope: { key: "intent.outOfScope", icon: AlertTriangle, color: "text-red-500" }
  }
  const config = configs[intent]
  return {
    label: t(`${config.key}.label`),
    icon: config.icon,
    color: config.color,
    description: t(`${config.key}.description`)
  }
}

// Helper function to get strategy config
function getStrategyConfig(strategy: ProcessingStrategy, t: (key: string) => string) {
  const configs: Record<ProcessingStrategy, { key: string; color: string }> = {
    fast_path: { key: "intent.path.fast", color: "text-green-600 bg-green-50" },
    standard: { key: "intent.path.standard", color: "text-blue-600 bg-blue-50" },
    quality: { key: "intent.path.quality", color: "text-purple-600 bg-purple-50" },
    multi_turn: { key: "intent.path.multiTurn", color: "text-amber-600 bg-amber-50" },
    fallback: { key: "intent.path.fallback", color: "text-gray-600 bg-gray-50" }
  }
  const config = configs[strategy]
  return {
    label: t(config.key),
    color: config.color
  }
}

// Helper function to get entity config
function getEntityConfig(entityType: EntityType, t: (key: string) => string) {
  const configs: Record<EntityType, { key: string; icon: string; color: string }> = {
    device: { key: "intent.entity.device", icon: "🔌", color: "bg-blue-100 text-blue-700" },
    location: { key: "intent.entity.location", icon: "📍", color: "bg-green-100 text-green-700" },
    value: { key: "intent.entity.value", icon: "🔢", color: "bg-purple-100 text-purple-700" },
    time_range: { key: "intent.entity.time", icon: "⏰", color: "bg-orange-100 text-orange-700" },
    action: { key: "intent.entity.action", icon: "▶️", color: "bg-red-100 text-red-700" },
    unknown: { key: "intent.entity.unknown", icon: "❓", color: "bg-gray-100 text-gray-700" }
  }
  const config = configs[entityType]
  return {
    label: t(config.key),
    icon: config.icon,
    color: config.color
  }
}

export function IntentIndicator({
  classification,
  isStreaming = false,
  showDetails = false
}: IntentIndicatorProps) {
  const { t } = useTranslation("chat")
  const config = getIntentConfig(classification.intent, t)
  const Icon = config.icon
  const strategyConfig = getStrategyConfig(classification.strategy, t)

  // Confidence percentage
  const confidencePercent = Math.round(classification.confidence * 100)

  // Confidence color based on level
  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return "bg-green-500"
    if (confidence >= 0.6) return "bg-blue-500"
    if (confidence >= 0.4) return "bg-yellow-500"
    return "bg-red-500"
  }

  return (
    <div className="intent-indicator space-y-2">
      {/* Main Intent Badge */}
      <div className="flex items-center gap-2 flex-wrap">
        <div
          title={config.description}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-muted border border-border/50 cursor-help"
        >
          <Icon className={cn("h-3.5 w-3.5", config.color)} />
          <span className="text-sm font-medium">{config.label}</span>
          {isStreaming && (
            <span className="flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-2 w-2 rounded-full bg-blue-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-500"></span>
            </span>
          )}
        </div>

        {/* Strategy Badge */}
        <Badge variant="outline" className={cn("text-xs", strategyConfig.color)}>
          {strategyConfig.label}
        </Badge>

        {/* Confidence */}
        <div
          title={`${t("intent.confidence")}: ${confidencePercent}%`}
          className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-secondary"
        >
          <div className={cn(
            "h-1.5 rounded-full transition-all",
            getConfidenceColor(classification.confidence)
          )} style={{ width: `${confidencePercent}%`, minWidth: '6px' }} />
          <span className="text-xs text-muted-foreground w-8 text-right">
            {confidencePercent}%
          </span>
        </div>
      </div>

      {/* Follow-up Prompt */}
      {classification.needs_followup && classification.followup_prompt && (
        <div className="flex items-start gap-2 p-2 rounded-md bg-orange-50 border border-orange-200">
          <HelpCircle className="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
          <p className="text-sm text-orange-800">{classification.followup_prompt}</p>
        </div>
      )}

      {/* Capability Statement */}
      {classification.intent === "out_of_scope" && classification.capability_statement && (
        <div className="flex items-start gap-2 p-2 rounded-md bg-red-50 border border-red-200">
          <AlertTriangle className="h-4 w-4 text-red-500 mt-0.5 flex-shrink-0" />
          <p className="text-sm text-red-800">{classification.capability_statement}</p>
        </div>
      )}

      {/* Extracted Entities */}
      {showDetails && classification.entities.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {classification.entities.map((entity, index) => {
            const entityConfig = getEntityConfig(entity.entity_type, t)
            return (
              <Badge
                key={index}
                variant="secondary"
                className={cn("text-xs", entityConfig.color)}
                title={`${entityConfig.label}: ${entity.value} (${Math.round(entity.confidence * 100)}%)`}
              >
                <span className="mr-1">{entityConfig.icon}</span>
                {entity.value}
              </Badge>
            )
          })}
        </div>
      )}

      {/* Sub-type indicator */}
      {showDetails && classification.sub_type && classification.sub_type !== "Unknown" && (
        <div className="text-xs text-muted-foreground">
          {t("intent.type")}: {classification.sub_type}
        </div>
      )}
    </div>
  )
}

// Compact version for inline display
export function CompactIntentBadge({
  classification
}: {
  classification: IntentClassification
}) {
  const { t } = useTranslation("chat")
  const config = getIntentConfig(classification.intent, t)

  return (
    <div
      title={config.description}
      className={cn(
        "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium",
        "bg-muted hover:bg-muted/70 transition-colors cursor-help"
      )}
    >
      <span className={config.color}>●</span>
      <span>{config.label}</span>
    </div>
  )
}

// Intent flow indicator for showing multiple intents in sequence
interface IntentFlowProps {
  steps: Array<{
    intent: IntentCategory
    label?: string
    status?: "pending" | "active" | "completed" | "error"
  }>
  currentStep?: number
}

export function IntentFlow({ steps, currentStep = 0 }: IntentFlowProps) {
  const { t } = useTranslation("chat")

  return (
    <div className="intent-flow flex items-center gap-1">
      {steps.map((step, index) => {
        const config = getIntentConfig(step.intent, t)
        const isCompleted = index < currentStep
        const isCurrent = index === currentStep

        const getStatusColor = () => {
          if (step.status === "error") return "text-red-500"
          if (step.status === "completed") return "text-green-500"
          if (isCurrent) return "text-blue-500"
          if (isCompleted) return "text-green-500"
          return "text-gray-400"
        }

        return (
          <div key={index} className="flex items-center">
            <div
              title={step.label || config.label}
              className={cn(
                "flex items-center justify-center w-6 h-6 rounded-full text-xs transition-colors cursor-help",
                getStatusColor(),
                isCurrent && "bg-blue-50 ring-2 ring-blue-200"
              )}
            >
              {step.status === "completed" || isCompleted ? (
                <CheckCircle2 className="h-3.5 w-3.5" />
              ) : step.status === "error" ? (
                <AlertTriangle className="h-3.5 w-3.5" />
              ) : (
                <config.icon className="h-3.5 w-3.5" />
              )}
            </div>
            {index < steps.length - 1 && (
              <div className={cn(
                "w-8 h-0.5 mx-0.5 transition-colors",
                index < currentStep ? "bg-green-300" : "bg-gray-200"
              )} />
            )}
          </div>
        )
      })}
    </div>
  )
}

// Intent confidence bar component
interface IntentConfidenceBarProps {
  confidence: number
  label?: string
  showLabel?: boolean
  size?: "sm" | "md" | "lg"
}

export function IntentConfidenceBar({
  confidence,
  label,
  showLabel = false,
  size = "md"
}: IntentConfidenceBarProps) {
  const { t } = useTranslation("chat")
  const percent = Math.round(confidence * 100)

  const height = {
    sm: "h-1",
    md: "h-2",
    lg: "h-3"
  }[size]

  const getColor = () => {
    if (confidence >= 0.8) return "bg-green-500"
    if (confidence >= 0.6) return "bg-blue-500"
    if (confidence >= 0.4) return "bg-yellow-500"
    return "bg-red-500"
  }

  return (
    <div className="flex items-center gap-2">
      {showLabel && (
        <span className="text-xs text-muted-foreground">
          {label || t("intent.confidence")}
        </span>
      )}
      <div className="flex-1 flex items-center gap-2">
        <div className={cn("flex-1 bg-secondary rounded-full overflow-hidden", height)}>
          <div
            className={cn("h-full rounded-full transition-all duration-500", getColor())}
            style={{ width: `${percent}%` }}
          />
        </div>
        <span className={cn(
          "text-xs font-medium tabular-nums w-10 text-right",
          percent >= 80 ? "text-green-600" :
          percent >= 60 ? "text-blue-600" :
          percent >= 40 ? "text-yellow-600" :
          "text-red-600"
        )}>
          {percent}%
        </span>
      </div>
    </div>
  )
}
