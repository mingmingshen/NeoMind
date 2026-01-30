import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import {
  Clock,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Brain,
  Database,
  Play,
  FileText,
  ChevronRight,
  Sparkles,
} from "lucide-react"
import { api } from "@/lib/api"
import { formatTimestamp } from "@/lib/utils/format"
import type { AgentExecutionDetail, DataCollected, ReasoningStep, Decision } from "@/types"

interface ExecutionDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  agentId: string
  executionId: string
}

export function ExecutionDetailDialog({
  open,
  onOpenChange,
  agentId,
  executionId,
}: ExecutionDetailDialogProps) {
  const { t } = useTranslation(['common', 'agents'])
  const [execution, setExecution] = useState<AgentExecutionDetail | null>(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (open && agentId && executionId) {
      loadExecution()
    }
  }, [open, agentId, executionId])

  const loadExecution = async () => {
    setLoading(true)
    try {
      const data = await api.getExecution(agentId, executionId)
      setExecution(data)
    } catch (error) {
      console.error('Failed to load execution:', error)
    } finally {
      setLoading(false)
    }
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Completed':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />
      case 'Failed':
        return <XCircle className="h-4 w-4 text-red-500" />
      case 'Running':
        return <Clock className="h-4 w-4 text-blue-500" />
      default:
        return <AlertCircle className="h-4 w-4 text-gray-500" />
    }
  }

  if (!execution) {
    return null
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[85vh]">
        <DialogHeader className="pb-2">
          <DialogTitle className="flex items-center gap-2 text-sm">
            <Sparkles className="h-4 w-4 text-primary" />
            Execution #{executionId.slice(-6)}
          </DialogTitle>
        </DialogHeader>

        {loading ? (
          <div className="flex items-center justify-center py-12">
            <Clock className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <ScrollArea className="max-h-[70vh] pr-3">
            <div className="space-y-3 pr-1">
              {/* Status Bar - Compact */}
              <div className="flex items-center justify-between py-2 px-3 bg-muted/30 rounded-lg">
                <div className="flex items-center gap-2">
                  {getStatusIcon(execution.status)}
                  <span className="text-xs font-medium">{t(`agents:executionStatus.${execution.status.toLowerCase()}`)}</span>
                </div>
                <div className="flex items-center gap-3 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {formatTimestamp(execution.timestamp, false)}
                  </span>
                  <span>{execution.duration_ms}ms</span>
                </div>
              </div>

              {execution.error && (
                <Card className="p-2 border-destructive/50 bg-destructive/5">
                  <div className="flex items-start gap-1.5 text-destructive">
                    <AlertCircle className="h-3.5 w-3.5 mt-0.5 shrink-0" />
                    <div className="text-xs break-words">{execution.error}</div>
                  </div>
                </Card>
              )}

              {execution.decision_process && (
                <>
                  {/* Situation Analysis - Compact */}
                  <div className="p-2.5 bg-muted/20 rounded-lg border">
                    <div className="flex items-center gap-1.5 mb-1.5">
                      <Brain className="h-3.5 w-3.5 text-blue-500 shrink-0" />
                      <span className="text-xs font-semibold">分析</span>
                    </div>
                    <p className="text-xs leading-relaxed">{execution.decision_process.situation_analysis}</p>
                  </div>

                  {/* Data Collected - Compact */}
                  {execution.decision_process.data_collected.length > 0 && (
                    <div className="p-2.5 bg-muted/20 rounded-lg border">
                      <div className="flex items-center gap-1.5 mb-2">
                        <Database className="h-3.5 w-3.5 text-purple-500 shrink-0" />
                        <span className="text-xs font-semibold">数据源</span>
                        <span className="text-[10px] text-muted-foreground">({execution.decision_process.data_collected.length})</span>
                      </div>
                      <div className="space-y-1.5">
                        {execution.decision_process.data_collected.slice(0, 5).map((data, idx) => (
                          <div key={idx} className="flex items-center gap-2 text-[10px]">
                            <span className="font-medium min-w-0 truncate">{data.source}</span>
                            <Badge variant="outline" className="text-[9px] h-4 px-1 shrink-0">{data.data_type}</Badge>
                          </div>
                        ))}
                        {execution.decision_process.data_collected.length > 5 && (
                          <div className="text-[10px] text-muted-foreground">
                            +{execution.decision_process.data_collected.length - 5} more
                          </div>
                        )}
                      </div>
                    </div>
                  )}

                  {/* Reasoning Steps - Compact Timeline */}
                  {execution.decision_process.reasoning_steps.length > 0 && (
                    <div className="p-2.5 bg-muted/20 rounded-lg border">
                      <div className="flex items-center gap-1.5 mb-2">
                        <Sparkles className="h-3.5 w-3.5 text-amber-500 shrink-0" />
                        <span className="text-xs font-semibold">推理步骤</span>
                      </div>
                      <div className="space-y-2">
                        {execution.decision_process.reasoning_steps.map((step, idx, arr) => (
                          <div key={idx} className="flex gap-2">
                            <div className="flex flex-col items-center">
                              <div className="w-5 h-5 rounded-full bg-primary/10 text-primary text-[10px] flex items-center justify-center shrink-0">
                                {step.step_number}
                              </div>
                              {idx < arr.length - 1 && (
                                <div className="w-0.5 flex-1 bg-border my-0.5" />
                              )}
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="text-xs font-medium">{step.description}</div>
                              <div className="flex items-center gap-2 mt-1">
                                <Badge variant="outline" className="text-[9px] h-4 px-1">{step.step_type}</Badge>
                                <span className="text-[10px] text-muted-foreground">
                                  {Math.round(step.confidence * 100)}%
                                </span>
                              </div>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Decisions - Compact */}
                  {execution.decision_process.decisions.length > 0 && (
                    <div className="p-2.5 bg-muted/20 rounded-lg border">
                      <div className="flex items-center gap-1.5 mb-2">
                        <Play className="h-3.5 w-3.5 text-green-500 shrink-0" />
                        <span className="text-xs font-semibold">决策</span>
                      </div>
                      <div className="space-y-1.5">
                        {execution.decision_process.decisions.map((decision, idx) => (
                          <div key={idx} className="p-2 bg-background rounded border">
                            <div className="text-xs font-medium mb-1">{decision.description}</div>
                            <div className="flex items-center justify-between">
                              <span className="text-[10px] text-muted-foreground truncate flex-1 mr-2">{decision.rationale}</span>
                              <Badge variant="secondary" className="text-[9px] h-4 px-1 shrink-0">{decision.action}</Badge>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Conclusion - Compact */}
                  <Card className="p-2 bg-primary/5 border-primary/20">
                    <div className="text-xs">
                      <span className="font-semibold text-primary">结论:</span>
                      <span className="ml-1">{execution.decision_process.conclusion}</span>
                    </div>
                  </Card>

                  {/* Actions Executed - Compact */}
                  {execution.result?.actions_executed && execution.result.actions_executed.length > 0 && (
                    <div className="p-2.5 bg-muted/20 rounded-lg border">
                      <div className="flex items-center gap-1.5 mb-2">
                        <Play className="h-3.5 w-3.5 text-green-500 shrink-0" />
                        <span className="text-xs font-semibold">执行动作</span>
                      </div>
                      <div className="space-y-1">
                        {execution.result.actions_executed.map((action, idx) => (
                          <div key={idx} className="flex items-center justify-between p-1.5 bg-background rounded border">
                            <div className="flex-1 min-w-0 mr-2">
                              <div className="text-xs truncate">{action.description}</div>
                              <div className="text-[10px] text-muted-foreground truncate">{action.target}</div>
                            </div>
                            <Badge variant={action.success ? "default" : "destructive"} className="text-[9px] h-4 px-1 shrink-0">
                              {action.success ? '✓' : '✗'}
                            </Badge>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          </ScrollArea>
        )}

        <DialogFooter className="pt-2">
          <Button size="sm" onClick={() => onOpenChange(false)}>
            {t('common:close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
