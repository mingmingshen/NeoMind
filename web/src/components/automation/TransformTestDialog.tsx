import { getPortalRoot } from '@/lib/portal'
import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Loader2, Play, CheckCircle2, AlertTriangle, X, FlaskConical } from 'lucide-react'
import { FormField } from '@/components/ui/field'
import { FormSection, FormSectionGroup } from '@/components/ui/form-section'
import { formatTimestamp } from '@/lib/utils/format'
import { api } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'

interface TransformTestDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  transformId: string
  devices: Array<{ id: string; name: string; device_type?: string }>
}

export function TransformTestDialog({ open, onOpenChange, transformId, devices }: TransformTestDialogProps) {
  const { t } = useTranslation(['automation', 'common'])
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [selectedDevice, setSelectedDevice] = useState('')
  const [testData, setTestData] = useState('{\n  "temperature": 23.5,\n  "humidity": 65,\n  "sensors": [\n    { "id": 1, "value": 25.3 },\n    { "id": 2, "value": 22.1 }\n  ]\n}')
  const [testing, setTesting] = useState(false)
  const [result, setResult] = useState<{
    metrics: Array<{
      device_id: string
      metric: string
      value: number
      timestamp: number
      quality: number | null
    }>
    count: number
    warnings: string[]
  } | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    if (open && devices.length > 0 && !selectedDevice) {
      setSelectedDevice(devices[0].id)
    }
  }, [open, devices, selectedDevice])

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setResult(null)
      setError(null)
    }
  }, [open])

  const handleTest = async () => {
    if (!selectedDevice) return

    setTesting(true)
    setResult(null)
    setError(null)

    try {
      let parsedData
      try {
        parsedData = JSON.parse(testData)
      } catch {
        setError(t('automation:invalidJson', { defaultValue: 'Invalid JSON data' }))
        setTesting(false)
        return
      }

      const device = devices.find((d) => d.id === selectedDevice)
      const response = await api.testTransform(transformId, {
        device_id: selectedDevice,
        device_type: device?.device_type,
        data: parsedData,
        timestamp: Math.floor(Date.now() / 1000),
      })

      setResult(response)
    } catch (err) {
      setError(err instanceof Error ? err.message : t('automation:testFailed', { defaultValue: 'Test failed' }))
    } finally {
      setTesting(false)
    }
  }

  const loadSampleData = () => {
    if (!selectedDevice) return

    const device = devices.find((d) => d.id === selectedDevice)
    const deviceType = device?.device_type || 'sensor'

    const samples: Record<string, unknown> = {
      sensor: {
        temperature: 23.5,
        humidity: 65,
        pressure: 1013.25,
        sensors: [
          { id: 1, type: 'temp', value: 25.3 },
          { id: 2, type: 'temp', value: 22.1 },
          { id: 3, type: 'humidity', value: 68 },
        ],
      },
      switch: {
        state: 'on',
        power: 150,
        voltage: 220,
        current: 0.68,
      },
      thermostat: {
        current_temperature: 21.5,
        target_temperature: 23,
        mode: 'heating',
        humidity: 55,
      },
    }

    const sample = samples[deviceType] || samples.sensor
    setTestData(JSON.stringify(sample, null, 2))
  }

  const handleClose = () => {
    if (!testing) {
      onOpenChange(false)
    }
  }

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-50 bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <Play className="h-5 w-5 text-muted-foreground shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('automation:testTransform', { defaultValue: 'Test Transform' })}</h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {t('automation:testTransformDesc', { defaultValue: 'Test with sample data' })}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={testing} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <FormSectionGroup>
      {/* Device Selection */}
      <FormSection title={t('automation:testDevice', { defaultValue: 'Test Device' })}>
        <FormField label={t('automation:selectDevice', { defaultValue: 'Select device' })}>
          <Select value={selectedDevice} onValueChange={setSelectedDevice}>
            <SelectTrigger>
              <SelectValue placeholder={t('automation:selectDevice', { defaultValue: 'Select device' })} />
            </SelectTrigger>
            <SelectContent>
              {devices.map((d) => (
                <SelectItem key={d.id} value={d.id}>
                  {d.name} {d.device_type && `(${d.device_type})`}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>
      </FormSection>

      {/* Test Data */}
      <FormSection title={t('automation:testData', { defaultValue: 'Test Data (JSON)' })}>
        <div className="flex justify-end mb-2">
          <Button variant="ghost" size="sm" onClick={loadSampleData}>
            {t('automation:loadSample', { defaultValue: 'Load Sample' })}
          </Button>
        </div>
        <Textarea
          value={testData}
          onChange={(e) => setTestData(e.target.value)}
          placeholder='{"temperature": 23.5, "humidity": 65}'
          rows={8}
          className="font-mono text-sm"
        />
      </FormSection>

      {/* Error Display */}
      {error && (
        <Card className="p-4 bg-muted border-destructive">
          <div className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-4 w-4" />
            <span>{error}</span>
          </div>
        </Card>
      )}

      {/* Results */}
      {result && (
        <FormSection title={t('automation:testResults', { defaultValue: 'Test Results' })}>
          <div className="flex items-center gap-2 mb-4">
            <CheckCircle2 className="h-5 w-5 text-success" />
            <span className="font-medium">
              {t('automation:testSuccess', { defaultValue: 'Test completed successfully' })}
            </span>
            <Badge variant="outline">{result.count} {t('automation:metrics', { defaultValue: 'metrics' })}</Badge>
          </div>
          {result.metrics.length > 0 ? (
            <div className="space-y-2">
              <h4 className="font-medium">{t('automation:generatedMetrics', { defaultValue: 'Generated Metrics' })}</h4>
              {result.metrics.map((metric, idx) => (
                <div key={idx} className="pl-4 border-l-2 border-success">
                  <div className="flex items-center gap-2">
                    <Badge variant="outline">{metric.metric}</Badge>
                    <span className="font-mono text-sm">{metric.value}</span>
                    {metric.quality !== null && (
                      <span className="text-xs text-muted-foreground">(q: {(metric.quality * 100).toFixed(0)}%)</span>
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {metric.device_id} @ {formatTimestamp(metric.timestamp)}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              {t('automation:noMetricsGenerated', { defaultValue: 'No metrics were generated. Check your transform configuration.' })}
            </p>
          )}
          {result.warnings.length > 0 && (
            <div className="mt-4 space-y-1">
              <h4 className="font-medium text-warning">{t('automation:warnings', { defaultValue: 'Warnings' })}</h4>
              {result.warnings.map((warning, idx) => (
                <div key={idx} className="text-sm text-warning flex items-start gap-2">
                  <AlertTriangle className="h-4 w-4 mt-0.5 flex-shrink-0" />
                  <span>{warning}</span>
                </div>
              ))}
            </div>
          )}
        </FormSection>
      )}
    </FormSectionGroup>
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={testing} className="min-w-[80px]">
                {t('common:close', { defaultValue: 'Close' })}
              </Button>
              <Button onClick={handleTest} disabled={!selectedDevice || testing} className="min-w-[80px]">
                {testing ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    {t('automation:testing', { defaultValue: 'Testing...' })}
                  </>
                ) : (
                  <>
                    <Play className="h-4 w-4 mr-2" />
                    {t('automation:runTest', { defaultValue: 'Run Test' })}
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>
      ) : null, getPortalRoot()
    )
  }

  // Desktop: Traditional dialog
  return createPortal(
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
            'max-w-3xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <Play className="h-5 w-5 text-muted-foreground" />
                <h2 className="text-lg font-semibold leading-none truncate">
                  {t('automation:testTransform', { defaultValue: 'Test Transform' })}
                </h2>
              </div>
              <p className="text-sm text-muted-foreground">
                {t('automation:testTransformDesc', { defaultValue: 'Test your transform with sample data to verify it produces the expected virtual metrics.' })}
              </p>
            </div>
            <button
              onClick={handleClose}
              disabled={testing}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <FormSectionGroup>
      {/* Device Selection */}
      <FormSection title={t('automation:testDevice', { defaultValue: 'Test Device' })}>
        <FormField label={t('automation:selectDevice', { defaultValue: 'Select device' })}>
          <Select value={selectedDevice} onValueChange={setSelectedDevice}>
            <SelectTrigger>
              <SelectValue placeholder={t('automation:selectDevice', { defaultValue: 'Select device' })} />
            </SelectTrigger>
            <SelectContent>
              {devices.map((d) => (
                <SelectItem key={d.id} value={d.id}>
                  {d.name} {d.device_type && `(${d.device_type})`}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>
      </FormSection>

      {/* Test Data */}
      <FormSection title={t('automation:testData', { defaultValue: 'Test Data (JSON)' })}>
        <div className="flex justify-end mb-2">
          <Button variant="ghost" size="sm" onClick={loadSampleData}>
            {t('automation:loadSample', { defaultValue: 'Load Sample' })}
          </Button>
        </div>
        <Textarea
          value={testData}
          onChange={(e) => setTestData(e.target.value)}
          placeholder='{"temperature": 23.5, "humidity": 65}'
          rows={10}
          className="font-mono text-sm"
        />
      </FormSection>

      {/* Error Display */}
      {error && (
        <Card className="p-4 bg-muted border-destructive">
          <div className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-4 w-4" />
            <span>{error}</span>
          </div>
        </Card>
      )}

      {/* Results */}
      {result && (
        <FormSection title={t('automation:testResults', { defaultValue: 'Test Results' })}>
          <div className="flex items-center gap-2 mb-4">
            <CheckCircle2 className="h-5 w-5 text-success" />
            <span className="font-medium">
              {t('automation:testSuccess', { defaultValue: 'Test completed successfully' })}
            </span>
            <Badge variant="outline">{result.count} {t('automation:metrics', { defaultValue: 'metrics' })}</Badge>
          </div>
          {result.metrics.length > 0 ? (
            <div className="space-y-2">
              <h4 className="font-medium">{t('automation:generatedMetrics', { defaultValue: 'Generated Metrics' })}</h4>
              {result.metrics.map((metric, idx) => (
                <div key={idx} className="pl-4 border-l-2 border-success">
                  <div className="flex items-center gap-2">
                    <Badge variant="outline">{metric.metric}</Badge>
                    <span className="font-mono text-sm">{metric.value}</span>
                    {metric.quality !== null && (
                      <span className="text-xs text-muted-foreground">(q: {(metric.quality * 100).toFixed(0)}%)</span>
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {metric.device_id} @ {formatTimestamp(metric.timestamp)}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              {t('automation:noMetricsGenerated', { defaultValue: 'No metrics were generated. Check your transform configuration.' })}
            </p>
          )}
          {result.warnings.length > 0 && (
            <div className="mt-4 space-y-1">
              <h4 className="font-medium text-warning">{t('automation:warnings', { defaultValue: 'Warnings' })}</h4>
              {result.warnings.map((warning, idx) => (
                <div key={idx} className="text-sm text-warning flex items-start gap-2">
                  <AlertTriangle className="h-4 w-4 mt-0.5 flex-shrink-0" />
                  <span>{warning}</span>
                </div>
              ))}
            </div>
          )}
        </FormSection>
      )}
    </FormSectionGroup>
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t shrink-0 bg-muted-30">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={testing}>
              {t('common:close', { defaultValue: 'Close' })}
            </Button>
            <Button size="sm" onClick={handleTest} disabled={!selectedDevice || testing}>
              {testing ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  {t('automation:testing', { defaultValue: 'Testing...' })}
                </>
              ) : (
                <>
                  <Play className="h-4 w-4 mr-2" />
                  {t('automation:runTest', { defaultValue: 'Run Test' })}
                </>
              )}
            </Button>
          </div>
        </div>
      )}
    </>,
    getPortalRoot()
  )
}
