/**
 * Agent Thinking Panel - Real-time display of agent reasoning steps
 *
 * Shows thinking steps and decisions as they happen during execution.
 */

import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Brain,
  ChevronRight,
  Loader2,
  Play,
  Zap,
  CheckCircle2,
  Clock,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useAgentEvents, type AgentThinkingStep } from "@/hooks/useAgentEvents"
import { formatTimestamp } from "@/lib/utils/format"

interface AgentThinkingPanelProps {
  agentId: string
  isExecuting: boolean
}

export function AgentThinkingPanel({ agentId, isExecuting }: AgentThinkingPanelProps) {
  const { t } = useTranslation(['common', 'agents'])
  const { currentExecution, thinkingSteps, decisions } = useAgentEvents(agentId, {
    enabled: true,
    eventTypes: ['AgentExecutionStarted', 'AgentThinking', 'AgentDecision', 'AgentExecutionCompleted'],
  })

  const [autoScroll, setAutoScroll] = useState(true)

  // Track when we should show the panel
  const [showPanel, setShowPanel] = useState(false)
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    if (isExecuting && currentExecution && !dismissed) {
      setShowPanel(true)
    }
    // Auto-hide after completion with a delay
    if (!isExecuting && currentExecution?.completed_at) {
      const timer = setTimeout(() => {
        setShowPanel(false)
      }, 5000)
      return () => clearTimeout(timer)
    }
  }, [isExecuting, currentExecution, dismissed])

  // Reset dismissed state when new execution starts
  useEffect(() => {
    if (isExecuting && currentExecution?.status === 'running') {
      setDismissed(false)
    }
  }, [isExecuting, currentExecution?.status])

  if (!showPanel || !currentExecution) return null

  const hasContent = thinkingSteps.length > 0 || decisions.length > 0

  return (
    <div className="border-t bg-muted/30">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b">
        <div className="flex items-center gap-2">
          {currentExecution.status === 'running' ? (
            <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
          ) : (
            <CheckCircle2 className={cn(
              "h-4 w-4",
              currentExecution.status === 'completed' ? "text-green-500" : "text-red-500"
            )} />
          )}
          <span className="text-sm font-medium">
            {currentExecution.status === 'running'
              ? t('agents:thinking.executing')
              : currentExecution.status === 'completed'
                ? t('agents:thinking.completed')
                : t('agents:thinking.failed')
            }
          </span>
          {currentExecution.duration_ms !== undefined && (
            <span className="text-xs text-muted-foreground flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {currentExecution.duration_ms < 1000
                ? `${currentExecution.duration_ms}ms`
                : `${(currentExecution.duration_ms / 1000).toFixed(1)}s`
              }
            </span>
          )}
          {thinkingSteps.length > 0 && (
            <Badge variant="outline" className="text-xs h-5">
              {thinkingSteps.length} {t('agents:thinking.steps')}
            </Badge>
          )}
        </div>
        <button
          onClick={() => {
            setShowPanel(false)
            setDismissed(true)
          }}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          {t('common:close')}
        </button>
      </div>

      {/* Content */}
      <ScrollArea className="max-h-[300px]">
        <div className="p-3 space-y-3">
          {!hasContent ? (
            <div className="flex items-center justify-center py-8 text-muted-foreground text-sm">
              <Loader2 className="h-4 w-4 animate-spin mr-2" />
              {t('agents:thinking.waiting')}
            </div>
          ) : (
            <>
              {/* Thinking Steps */}
              {thinkingSteps.length > 0 && (
                <div>
                  <h4 className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1.5">
                    <Brain className="h-3.5 w-3.5 text-purple-500" />
                    {t('agents:thinking.reasoningSteps')}
                  </h4>
                  <div className="space-y-2">
                    {thinkingSteps.map((step) => (
                      <ThinkingStep key={step.step_number} step={step} />
                    ))}
                  </div>
                </div>
              )}

              {/* Decisions */}
              {decisions.length > 0 && (
                <div>
                  <h4 className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1.5">
                    <Play className="h-3.5 w-3.5 text-green-500" />
                    {t('agents:thinking.decisions')}
                  </h4>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                    {decisions.map((decision, idx) => (
                      <Decision key={idx} decision={decision} />
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}

// ============================================================================
// Sub Components
// ============================================================================

interface ThinkingStepProps {
  step: AgentThinkingStep
}

function ThinkingStep({ step }: ThinkingStepProps) {
  const { t } = useTranslation(['common', 'agents'])

  const getStepTypeColor = (stepType: string) => {
    switch (stepType.toLowerCase()) {
      case 'analysis':
      case 'analyze':
        return 'text-blue-500 bg-blue-500/10 border-blue-500/20'
      case 'evaluation':
      case 'evaluate':
        return 'text-orange-500 bg-orange-500/10 border-orange-500/20'
      case 'planning':
      case 'plan':
        return 'text-purple-500 bg-purple-500/10 border-purple-500/20'
      case 'execution':
      case 'execute':
        return 'text-green-500 bg-green-500/10 border-green-500/20'
      default:
        return 'text-muted-foreground bg-muted/50'
    }
  }

  return (
    <div className="flex gap-2">
      <div className="flex flex-col items-center shrink-0">
        <div className="w-6 h-6 rounded-full bg-primary/10 text-primary text-xs flex items-center justify-center font-medium">
          {step.step_number}
        </div>
        {step.step_number < 10 && <div className="w-0.5 flex-1 bg-border min-h-[16px]" />}
      </div>
      <Card className="flex-1 p-2.5">
        <div className="flex items-start justify-between gap-2 mb-1">
          <p className="text-sm flex-1">{step.description}</p>
          <Badge
            variant="outline"
            className={cn("text-xs h-5 shrink-0", getStepTypeColor(step.step_type))}
          >
            {step.step_type}
          </Badge>
        </div>
        {step.details != null && (
          <div className="mt-2 pt-2 border-t">
            <pre className="text-xs bg-muted p-2 rounded overflow-x-auto max-h-20">
              {String(
                typeof step.details === 'string'
                  ? step.details
                  : JSON.stringify(step.details, null, 2)
              )}
            </pre>
          </div>
        )}
        <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatTimestamp(step.timestamp, false)}
          </span>
        </div>
      </Card>
    </div>
  )
}

interface DecisionProps {
  decision: {
    description: string
    rationale: string
    action: string
    confidence: number
    timestamp: number
  }
}

function Decision({ decision }: DecisionProps) {
  const { t } = useTranslation(['common', 'agents'])

  return (
    <Card className="p-2.5">
      <div className="text-sm font-medium mb-1">{decision.description}</div>
      {decision.rationale && (
        <div className="text-xs text-muted-foreground mb-2">{decision.rationale}</div>
      )}
      <div className="flex items-center justify-between text-xs">
        <div className="flex items-center gap-1.5">
          <Zap className="h-3 w-3 text-green-500" />
          <Badge variant="secondary" className="h-5">
            {decision.action}
          </Badge>
        </div>
        <Badge
          variant={decision.confidence > 0.7 ? "default" : "secondary"}
          className="h-5"
        >
          {(decision.confidence * 100).toFixed(0)}%
        </Badge>
      </div>
    </Card>
  )
}
