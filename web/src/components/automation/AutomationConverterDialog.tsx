import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/lib/api'
import type { AutomationType } from '@/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Loader2, ArrowRight, AlertCircle, CheckCircle2 } from 'lucide-react'

export interface AutomationConverterDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  automationId: string
  automationName: string
  currentType: AutomationType
  onConversionComplete?: (newId: string, newType: AutomationType) => void
}

interface ConversionInfo {
  automation_id: string
  current_type: string
  can_convert: boolean
  target_type?: string
  reason: string
  estimated_complexity?: number
}

interface ConversionResult {
  automation: {
    id: string
    name: string
    type: AutomationType
  }
  message: string
  original_id: string
  new_id: string
}

export function AutomationConverterDialog({
  open,
  onOpenChange,
  automationId,
  automationName,
  currentType,
  onConversionComplete,
}: AutomationConverterDialogProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [loading, setLoading] = useState(false)
  const [converting, setConverting] = useState(false)
  const [conversionInfo, setConversionInfo] = useState<ConversionInfo | null>(null)
  const [selectedTargetType, setSelectedTargetType] = useState<AutomationType>('workflow')
  const [error, setError] = useState<string | null>(null)

  // Determine target type based on current type
  const targetType: AutomationType = currentType === 'rule' ? 'workflow' : 'rule'

  useEffect(() => {
    if (open) {
      setSelectedTargetType(targetType)
      loadConversionInfo()
    }
  }, [open, targetType])

  const loadConversionInfo = async () => {
    setLoading(true)
    setError(null)
    try {
      const info = await api.getConversionInfo(automationId)
      setConversionInfo(info)
    } catch (err) {
      setError((err as Error).message || t('automation:intentAnalysisFailed'))
    } finally {
      setLoading(false)
    }
  }

  const handleConvert = async () => {
    setConverting(true)
    setError(null)
    try {
      const result: ConversionResult = await api.convertAutomation(automationId, selectedTargetType)
      onConversionComplete?.(result.new_id, result.automation.type)
      onOpenChange(false)
    } catch (err) {
      setError((err as Error).message || t('automation:intentAnalysisFailed'))
    } finally {
      setConverting(false)
    }
  }

  const getTypeLabel = (type: AutomationType) =>
    type === 'transform' ? t('automation:transforms', { defaultValue: 'Transforms' })
    : type === 'rule' ? t('automation:rules', { defaultValue: 'Rules' })
    : t('automation:workflows', { defaultValue: 'Workflows' })

  const getTypeColor = (type: AutomationType) =>
    type === 'transform' ? 'bg-green-100 text-green-700 border-green-200'
    : type === 'rule' ? 'bg-blue-100 text-blue-700 border-blue-200'
    : 'bg-purple-100 text-purple-700 border-purple-200'

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t('automation:convertAutomation', { defaultValue: 'Convert Automation' })}</DialogTitle>
          <DialogDescription>
            {t('automation:convertAutomationDesc', { defaultValue: 'Convert this automation between Rule and Workflow types' })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Current Type Display */}
          <div className="flex items-center justify-center gap-4 p-4 bg-muted rounded-lg">
            <div className="text-center">
              <Badge variant="outline" className={`${getTypeColor(currentType)} text-sm px-3 py-1`}>
                {getTypeLabel(currentType)}
              </Badge>
              <p className="text-sm text-muted-foreground mt-1">{automationName}</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground" />
            <div className="text-center">
              <Badge variant="outline" className={`${getTypeColor(targetType)} text-sm px-3 py-1`}>
                {getTypeLabel(targetType)}
              </Badge>
              <p className="text-sm text-muted-foreground mt-1">{t('automation:converted', { defaultValue: 'Converted' })}</p>
            </div>
          </div>

          {loading ? (
            <div className="flex items-center justify-center p-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              <span className="ml-2 text-muted-foreground">{t('common:loading')}</span>
            </div>
          ) : error ? (
            <Card className="p-4 border-destructive">
              <div className="flex items-center gap-2">
                <AlertCircle className="h-4 w-4 text-destructive" />
                <p className="text-sm text-destructive">{error}</p>
              </div>
            </Card>
          ) : conversionInfo ? (
            <Tabs defaultValue="info" className="w-full">
              <TabsList className="grid w-full grid-cols-2">
                <TabsTrigger value="info">
                  {t('automation:conversionInfo', { defaultValue: 'Conversion Info' })}
                </TabsTrigger>
                <TabsTrigger value="preview">
                  {t('automation:preview', { defaultValue: 'Preview' })}
                </TabsTrigger>
              </TabsList>

              <TabsContent value="info" className="space-y-4">
                {conversionInfo.can_convert ? (
                  <>
                    <Card className="p-4">
                      <div className="flex items-start gap-2">
                        <CheckCircle2 className="h-4 w-4 text-green-600 mt-0.5" />
                        <div>
                          <p className="font-medium">{conversionInfo.reason}</p>
                          {conversionInfo.estimated_complexity !== undefined && (
                            <p className="text-sm text-muted-foreground mt-1">
                              {t('automation:estimatedComplexity')}: {conversionInfo.estimated_complexity}/5
                            </p>
                          )}
                        </div>
                      </div>
                    </Card>

                    <div className="text-sm space-y-2">
                      <h4 className="font-medium">{t('automation:whatHappens', { defaultValue: 'What happens' })}:</h4>
                      <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                        <li>
                          {currentType === 'rule'
                            ? t('automation:ruleToWorkflowDesc', { defaultValue: 'Your rule will be converted to a workflow with a single step' })
                            : t('automation:workflowToRuleDesc', { defaultValue: 'Simple workflow steps will be converted to a rule condition' })}
                        </li>
                        <li>{t('automation:originalPreserved', { defaultValue: 'The original automation will be preserved' })}</li>
                        <li>{t('automation:newCreated', { defaultValue: 'A new automation will be created' })}</li>
                      </ul>
                    </div>
                  </>
                ) : (
                  <Card className="p-4 border-destructive">
                    <div className="flex items-start gap-2">
                      <AlertCircle className="h-4 w-4 text-destructive mt-0.5" />
                      <div>
                        <p className="font-medium text-destructive">{t('automation:cannotConvert', { defaultValue: 'Cannot convert' })}</p>
                        <p className="text-sm">{conversionInfo.reason}</p>
                      </div>
                    </div>
                  </Card>
                )}
              </TabsContent>

              <TabsContent value="preview" className="space-y-4">
                <div className="text-sm">
                  <h4 className="font-medium mb-2">{t('automation:targetStructure', { defaultValue: 'Target structure' })}:</h4>
                  <pre className="p-3 bg-muted rounded text-xs overflow-x-auto max-h-48">
                    {currentType === 'rule'
                      ? `{
  "type": "workflow",
  "name": "${automationName} (converted)",
  "triggers": [/* from rule trigger */],
  "steps": [
    /* rule actions as step */
  ]
}`
                      : `{
  "type": "rule",
  "name": "${automationName} (converted)",
  "trigger": /* from workflow trigger */,
  "condition": /* from workflow condition */,
  "actions": /* from workflow steps */
}`}
                  </pre>
                </div>
              </TabsContent>
            </Tabs>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={converting}>
            {t('common:cancel')}
          </Button>
          <Button
            onClick={handleConvert}
            disabled={converting || loading || !conversionInfo?.can_convert}
          >
            {converting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            {converting
              ? t('automation:converting', { defaultValue: 'Converting...' })
              : t('automation:convert', { defaultValue: 'Convert' })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
