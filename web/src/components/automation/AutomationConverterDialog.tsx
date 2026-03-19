import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { api } from '@/lib/api'
import type { AutomationType } from '@/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Loader2, ArrowRight, AlertCircle, CheckCircle2, X, RefreshCw, ArrowLeftRight } from 'lucide-react'
import { FormSection, FormSectionGroup } from '@/components/ui/form-section'
import { cn } from '@/lib/utils'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'

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
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [loading, setLoading] = useState(false)
  const [converting, setConverting] = useState(false)
  const [conversionInfo, setConversionInfo] = useState<ConversionInfo | null>(null)
  const [selectedTargetType, setSelectedTargetType] = useState<AutomationType>('transform')
  const [error, setError] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState('info')

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  // Determine target type based on current type
  const targetType: AutomationType = currentType === 'rule' ? 'transform' : currentType === 'transform' ? 'rule' : 'transform'

  useEffect(() => {
    if (open) {
      setSelectedTargetType(targetType)
      setActiveTab('info')
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

  const handleClose = useCallback(() => {
    if (!converting) {
      onOpenChange(false)
    }
  }, [converting, onOpenChange])

  const getTypeLabel = (type: AutomationType) =>
    type === 'transform' ? t('automation:transforms', { defaultValue: 'Transforms' })
    : t('automation:rules', { defaultValue: 'Rules' })

  const getTypeColor = (type: AutomationType) =>
    type === 'transform' ? 'bg-green-100 text-green-700 border-green-200 dark:bg-green-900/30 dark:text-green-400 dark:border-green-800'
    : 'bg-blue-100 text-blue-700 border-blue-200 dark:bg-blue-900/30 dark:text-blue-400 dark:border-blue-800'

  const ConverterContent = () => (
    <FormSectionGroup>
      {/* Current Type Display */}
      <div className="flex items-center justify-center gap-4 p-4 bg-muted/50 rounded-xl">
        <div className="text-center">
          <Badge variant="outline" className={cn('text-sm px-3 py-1', getTypeColor(currentType))}>
            {getTypeLabel(currentType)}
          </Badge>
          <p className="text-sm text-muted-foreground mt-1 truncate max-w-[120px]">{automationName}</p>
        </div>
        <ArrowRight className="h-5 w-5 text-muted-foreground" />
        <div className="text-center">
          <Badge variant="outline" className={cn('text-sm px-3 py-1', getTypeColor(targetType))}>
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
        <div className="space-y-4">
          {/* Mobile tabs */}
          {isMobile ? (
            <div className="space-y-4">
              <div className="flex gap-2 p-1 bg-muted/50 rounded-xl">
                <button
                  onClick={() => setActiveTab('info')}
                  className={cn(
                    'flex-1 py-2.5 px-3 text-sm font-medium rounded-lg transition-all',
                    activeTab === 'info'
                      ? 'bg-background text-foreground shadow-sm'
                      : 'text-muted-foreground'
                  )}
                >
                  {t('automation:conversionInfo', { defaultValue: 'Conversion Info' })}
                </button>
                <button
                  onClick={() => setActiveTab('preview')}
                  className={cn(
                    'flex-1 py-2.5 px-3 text-sm font-medium rounded-lg transition-all',
                    activeTab === 'preview'
                      ? 'bg-background text-foreground shadow-sm'
                      : 'text-muted-foreground'
                  )}
                >
                  {t('automation:preview', { defaultValue: 'Preview' })}
                </button>
              </div>

              {activeTab === 'info' && (
                <ConversionInfoSection
                  conversionInfo={conversionInfo}
                  currentType={currentType}
                  t={t}
                />
              )}

              {activeTab === 'preview' && (
                <PreviewSection
                  automationName={automationName}
                  currentType={currentType}
                  t={t}
                />
              )}
            </div>
          ) : (
            /* Desktop tabs */
            <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
              <TabsList className="grid w-full grid-cols-2">
                <TabsTrigger value="info">
                  {t('automation:conversionInfo', { defaultValue: 'Conversion Info' })}
                </TabsTrigger>
                <TabsTrigger value="preview">
                  {t('automation:preview', { defaultValue: 'Preview' })}
                </TabsTrigger>
              </TabsList>

              <TabsContent value="info" className="space-y-4 mt-4">
                <ConversionInfoSection
                  conversionInfo={conversionInfo}
                  currentType={currentType}
                  t={t}
                />
              </TabsContent>

              <TabsContent value="preview" className="space-y-4 mt-4">
                <PreviewSection
                  automationName={automationName}
                  currentType={currentType}
                  t={t}
                />
              </TabsContent>
            </Tabs>
          )}
        </div>
      ) : null}
    </FormSectionGroup>
  )

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-[100] bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <ArrowLeftRight className="h-5 w-5 text-muted-foreground shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('automation:convertAutomation', { defaultValue: 'Convert Automation' })}</h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {t('automation:convertAutomationDesc', { defaultValue: 'Convert between types' })}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={converting} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <ConverterContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={converting} className="min-w-[80px]">
                {t('common:cancel')}
              </Button>
              <Button
                onClick={handleConvert}
                disabled={converting || loading || !conversionInfo?.can_convert}
                className="min-w-[80px]"
              >
                {converting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                {converting
                  ? t('automation:converting', { defaultValue: 'Converting...' })
                  : t('automation:convert', { defaultValue: 'Convert' })}
              </Button>
            </div>
          </div>
        </div>
      ) : null,
      document.body
    )
  }

  // Desktop: Traditional dialog
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={handleClose}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-50',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[calc(100vh-2rem)] sm:max-h-[90vh]',
            'flex flex-col',
            'max-w-2xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <ArrowLeftRight className="h-5 w-5 text-muted-foreground" />
                <h2 className="text-lg font-semibold leading-none truncate">
                  {t('automation:convertAutomation', { defaultValue: 'Convert Automation' })}
                </h2>
              </div>
              <p className="text-sm text-muted-foreground">
                {t('automation:convertAutomationDesc', { defaultValue: 'Convert this automation between Rule and Workflow types' })}
              </p>
            </div>
            <button
              onClick={handleClose}
              disabled={converting}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <ConverterContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t shrink-0 bg-muted/30">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={converting}>
              {t('common:cancel')}
            </Button>
            <Button
              size="sm"
              onClick={handleConvert}
              disabled={converting || loading || !conversionInfo?.can_convert}
            >
              {converting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              {converting
                ? t('automation:converting', { defaultValue: 'Converting...' })
                : t('automation:convert', { defaultValue: 'Convert' })}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}

// Helper components
function ConversionInfoSection({
  conversionInfo,
  currentType,
  t,
}: {
  conversionInfo: ConversionInfo
  currentType: AutomationType
  t: (key: string, options?: { defaultValue?: string }) => string
}) {
  return conversionInfo.can_convert ? (
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
  )
}

function PreviewSection({
  automationName,
  currentType,
  t,
}: {
  automationName: string
  currentType: AutomationType
  t: (key: string, options?: { defaultValue?: string }) => string
}) {
  return (
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
  )
}
