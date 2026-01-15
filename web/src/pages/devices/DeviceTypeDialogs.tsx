import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { CodeEditor } from "@/components/ui/code-editor"
import { toast } from "@/components/ui/use-toast"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Plus, Trash2, ArrowDown, FileText, Sparkles, Code, ChevronDown, ChevronUp, Settings, AlertCircle, Wand2, Check, Zap } from "lucide-react"
import { cn } from "@/lib/utils"
import type { DeviceType, MetricDefinition, CommandDefinition, ParameterDefinition } from "@/types"

// Validation result type
interface ValidationResult {
  valid: boolean
  errors?: string[]
  warnings?: string[]
  message: string
}

// Form errors type
interface FormErrors {
  device_type?: string
  name?: string
  metrics?: Record<number, string>
  commands?: Record<number, string>
  [key: string]: string | Record<number, string> | undefined
}

// ============================================================================
// Add Device Type Dialog (Full Screen with JSON Editor + AI Generation)
// ============================================================================

interface AddDeviceTypeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onAdd: (definition: DeviceType) => Promise<boolean>
  onValidate: (definition: DeviceType) => Promise<ValidationResult>
  onGenerateMDL: (deviceName: string, description: string, metricsExample: string, commandsExample: string) => Promise<string>
  adding: boolean
  validating: boolean
  generating: boolean
}

export function AddDeviceTypeDialog({
  open,
  onOpenChange,
  onAdd,
  onValidate,
  onGenerateMDL,
  adding,
  validating,
  generating,
}: AddDeviceTypeDialogProps) {
  const { t } = useTranslation(['common', 'devices'])
  const [addTypeMode, setAddTypeMode] = useState<"form" | "json" | "ai">("form")

  // JSON mode state
  const [jsonDefinition, setJsonDefinition] = useState("")
  const [jsonError, setJsonError] = useState("")
  const [jsonValidation, setJsonValidation] = useState<ValidationResult | null>(null)

  // Form mode state
  const [formData, setFormData] = useState<DeviceType>({
    device_type: "",
    name: "",
    description: "",
    categories: [],
    mode: "simple",
    metrics: [],
    commands: [],
    uplink_samples: [],
  })
  const [formErrors, setFormErrors] = useState<FormErrors>({})
  const [touched, setTouched] = useState<Record<string, boolean>>({})

  // AI mode state
  const [aiDeviceName, setAiDeviceName] = useState("")
  const [aiDeviceDesc, setAiDeviceDesc] = useState("")
  const [aiMetricsExample, setAiMetricsExample] = useState("")
  const [aiCommandsExample, setAiCommandsExample] = useState("")
  const [aiGenerated, setAiGenerated] = useState(false)

  // Reset form when dialog opens/closes
  useEffect(() => {
    if (open) {
      setFormData({
        device_type: "",
        name: "",
        description: "",
        categories: [],
        mode: "simple",
        metrics: [],
        commands: [],
        uplink_samples: [],
      })
      setFormErrors({})
      setTouched({})
      setJsonDefinition("")
      setJsonError("")
      setJsonValidation(null)
      setAiDeviceName("")
      setAiDeviceDesc("")
      setAiMetricsExample("")
      setAiCommandsExample("")
      setAiGenerated(false)
    }
  }, [open])

  // Validate form in real-time
  useEffect(() => {
    const errors: FormErrors = {}

    if (touched.device_type || touched.name) {
      if (!formData.device_type?.trim()) {
        errors.device_type = t('devices:types.validation.deviceTypeRequired')
      }
      if (!formData.name?.trim()) {
        errors.name = t('devices:types.validation.nameRequired')
      }
    }

    const metricErrors: Record<number, string> = {}
    formData.metrics?.forEach((metric, i) => {
      if (!metric.name?.trim()) {
        metricErrors[i] = t('devices:types.validation.metricNameRequired')
      }
    })
    if (Object.keys(metricErrors).length > 0) {
      errors.metrics = metricErrors
    }

    const commandErrors: Record<number, string> = {}
    formData.commands?.forEach((cmd, i) => {
      if (!cmd.name?.trim()) {
        commandErrors[i] = t('devices:types.validation.commandNameRequired')
      }
    })
    if (Object.keys(commandErrors).length > 0) {
      errors.commands = commandErrors
    }

    setFormErrors(errors)
  }, [formData, touched, t])

  const isFormValid = () => {
    // Basic validation
    if (!formData.device_type?.trim() || !formData.name?.trim()) {
      return false
    }

    // For Simple mode, metrics are optional (using samples instead)
    const isSimpleMode = formData.mode === "simple"

    // For Full mode, validate metrics
    if (!isSimpleMode && formData.metrics?.some(m => !m.name?.trim())) {
      return false
    }

    // Commands are required in both modes
    if (formData.commands?.some(c => !c.name?.trim())) {
      return false
    }

    return true
  }

  const handleFieldChange = <K extends keyof DeviceType>(field: K, value: DeviceType[K]) => {
    setTouched(prev => ({ ...prev, [field]: true }))
    setFormData(prev => ({ ...prev, [field]: value }))
  }

  const generateDeviceTypeId = (name: string): string => {
    return name.toLowerCase()
      .replace(/\s+/g, "_")
      .replace(/[^a-z0-9_]/g, "")
      .replace(/_+/g, "_")
      .replace(/^_|_$/g, "")
  }

  const handleAdd = async () => {
    let definition: DeviceType

    if (addTypeMode === "json") {
      if (!jsonDefinition.trim()) {
        setJsonError(t('devices:types.add.definitionRequired'))
        return
      }
      try {
        definition = JSON.parse(jsonDefinition)
      } catch (e) {
        setJsonError(t('devices:types.add.jsonError', { error: (e as Error).message }))
        return
      }
      if (!definition.device_type || !definition.name) {
        setJsonError(t('devices:types.add.missingFields'))
        return
      }
    } else {
      if (!isFormValid()) {
        setTouched({ device_type: true, name: true })
        toast({
          title: t('devices:types.validationError'),
          description: t('devices:types.requiredFields'),
          variant: "destructive",
        })
        return
      }
      definition = formData
    }

    const success = await onAdd(definition)
    if (success) {
      onOpenChange(false)
      toast({
        title: t('devices:types.save.success'),
        description: t('devices:types.save.added', { name: definition.name }),
      })
    } else {
      toast({
        title: t('devices:types.save.error'),
        description: t('devices:types.save.retry'),
        variant: "destructive",
      })
    }
  }

  const handleValidate = async () => {
    let definition: DeviceType
    let errorSetter: (e: string) => void
    let validationSetter: (v: ValidationResult) => void

    if (addTypeMode === "json") {
      if (!jsonDefinition.trim()) {
        setJsonError(t('devices:types.add.definitionRequired'))
        return
      }
      try {
        definition = JSON.parse(jsonDefinition)
        errorSetter = setJsonError
        validationSetter = setJsonValidation
      } catch (e) {
        setJsonError(t('devices:types.add.jsonError', { error: (e as Error).message }))
        return
      }
    } else {
      definition = formData
      errorSetter = () => {}
      validationSetter = () => {}
    }

    const result = await onValidate(definition)
    validationSetter(result)
    if (!result.valid && result.errors) {
      errorSetter(result.errors.join("; "))
    }
  }

  const handleGenerateMDL = async () => {
    if (!aiDeviceName.trim()) {
      toast({
        title: t('devices:types.validationError'),
        description: t('devices:types.add.nameRequired'),
        variant: "destructive",
      })
      return
    }

    try {
      const mdlData = await onGenerateMDL(aiDeviceName, aiDeviceDesc, aiMetricsExample, aiCommandsExample)
      const parsed = JSON.parse(mdlData)

      // Convert from backend format (uplink/downlink) to frontend format (flat metrics/commands)
      const converted: DeviceType = {
        device_type: parsed.device_type || generateDeviceTypeId(aiDeviceName),
        name: parsed.name || aiDeviceName,
        description: parsed.description || aiDeviceDesc,
        categories: parsed.categories || [],
        metrics: parsed.uplink?.metrics || parsed.metrics || [],
        commands: parsed.downlink?.commands || parsed.commands || [],
        metric_count: parsed.uplink?.metrics?.length || parsed.metrics?.length || 0,
        command_count: parsed.downlink?.commands?.length || parsed.commands?.length || 0,
      }

      // Fill form with generated data and switch to form mode
      setFormData(converted)
      setTouched({ device_type: true, name: true })
      setAiGenerated(true)
      setAddTypeMode("form")

      toast({
        title: t('common:success'),
        description: (
          <div className="flex items-center gap-2">
            <Check className="h-4 w-4 text-green-600" />
            <span>{t('devices:types.generatedFilled')}</span>
          </div>
        ),
      })
    } catch (e) {
      toast({
        title: t('devices:types.generate.failed'),
        description: (e as Error).message,
        variant: "destructive",
      })
    }
  }

  const loadJsonExample = () => {
    const example = {
      device_type: "example_sensor",
      name: t('devices:types.example.sensor'),
      description: t('devices:types.example.description'),
      categories: ["sensor", "example"],
      metrics: [
        {
          name: "temperature",
          display_name: t('devices:types.example.temperature'),
          data_type: "float",
          unit: "°C",
          min: -40,
          max: 100,
        },
      ],
      commands: [
        {
          name: "set_interval",
          display_name: t('devices:types.example.setInterval'),
          payload_template: '{"action":"set_interval","interval":${interval}}',
          parameters: [
            {
              name: "interval",
              display_name: t('devices:types.example.interval'),
              data_type: "integer",
              default_value: { Integer: 60 },
              min: 10,
              max: 3600,
              unit: "s",
            },
          ],
        },
      ],
    }
    setJsonDefinition(JSON.stringify(example, null, 2))
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-6xl h-[95vh] max-h-[95vh] flex flex-col p-0">
        <DialogHeader className="px-6 pt-6 pb-4 border-b">
          <div className="flex items-center justify-between">
            <DialogTitle className="text-xl">{t('devices:types.add.title')}</DialogTitle>
            {aiGenerated && addTypeMode === "form" && (
              <Badge variant="default" className="gap-1">
                <Check className="h-3 w-3" />
                {t('devices:types.generatedFilled')}
              </Badge>
            )}
          </div>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6">
          <Tabs value={addTypeMode} onValueChange={(v) => setAddTypeMode(v as "form" | "json" | "ai")} className="w-full mt-4">
            <TabsList className="grid w-full grid-cols-3 h-12">
              <TabsTrigger value="form" className="flex items-center gap-2 text-base">
                <Settings className="h-4 w-4" />
                {t('devices:types.formMode')}
              </TabsTrigger>
              <TabsTrigger value="json" className="flex items-center gap-2 text-base">
                <Code className="h-4 w-4" />
                {t('devices:types.jsonMode')}
              </TabsTrigger>
              <TabsTrigger value="ai" className="flex items-center gap-2 text-base">
                <Sparkles className="h-4 w-4" />
                {t('devices:types.aiMode')}
              </TabsTrigger>
            </TabsList>

            {/* Form Mode */}
            <TabsContent value="form" className="mt-6 space-y-6">
              <DeviceTypeForm
                data={formData}
                onChange={setFormData}
                onFieldChange={handleFieldChange}
                errors={formErrors}
                touched={touched}
                setTouched={setTouched}
              />
            </TabsContent>

            {/* JSON Mode */}
            <TabsContent value="json" className="mt-6 space-y-4">
              <div className="rounded-lg bg-muted/50 p-4 text-sm">
                <p className="font-medium text-foreground mb-2">
                  {t('devices:types.advanced.format')}
                </p>
                <p className="text-muted-foreground mb-3">{t('devices:types.advanced.supported')}</p>
                <button
                  onClick={loadJsonExample}
                  className="text-primary hover:underline text-sm"
                >
                  {t('devices:types.loadExample')}
                </button>
              </div>
              <CodeEditor
                value={jsonDefinition}
                onChange={setJsonDefinition}
                language="json"
                placeholder={t('devices:types.advanced.placeholder')}
                className="min-h-[400px] max-h-[500px]"
                error={jsonError}
              />
              <div className="flex items-center gap-3">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleValidate}
                  disabled={validating || !jsonDefinition.trim()}
                >
                  {t('devices:types.validate.button')}
                </Button>
                {jsonValidation && (
                  <div className={`text-sm flex items-center gap-1 ${jsonValidation.valid ? "text-green-600" : "text-destructive"}`}>
                    {jsonValidation.valid ? <Check className="h-4 w-4" /> : <AlertCircle className="h-4 w-4" />}
                    {jsonValidation.message}
                  </div>
                )}
              </div>
              {jsonValidation?.errors && jsonValidation.errors.length > 0 && (
                <div className="rounded-lg bg-destructive/10 p-3 text-sm">
                  <p className="font-medium text-destructive mb-2">{t('devices:types.errors')}</p>
                  <ul className="list-disc list-inside space-y-1 text-destructive">
                    {jsonValidation.errors.map((err, i) => <li key={i}>{err}</li>)}
                  </ul>
                </div>
              )}
              {jsonValidation?.warnings && jsonValidation.warnings.length > 0 && (
                <div className="rounded-lg bg-muted p-3 text-sm">
                  <p className="font-medium mb-2">{t('devices:types.warnings')}</p>
                  <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                    {jsonValidation.warnings.map((warn, i) => <li key={i}>{warn}</li>)}
                  </ul>
                </div>
              )}
            </TabsContent>

            {/* AI Mode - Optimized Layout */}
            <TabsContent value="ai" className="mt-6 space-y-6">
              <div className="rounded-lg bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-950/30 dark:to-blue-950/30 p-6 border border-purple-100 dark:border-purple-900">
                <div className="flex items-start gap-3">
                  <div className="p-2 bg-gradient-to-br from-purple-500 to-blue-500 rounded-lg">
                    <Wand2 className="h-5 w-5 text-white" />
                  </div>
                  <div>
                    <p className="font-semibold text-foreground mb-1">{t('devices:types.smart.title')}</p>
                    <p className="text-sm text-muted-foreground">{t('devices:types.smart.description')}</p>
                  </div>
                </div>
              </div>

              {/* Device Name - Full Width */}
              <div className="space-y-2">
                <Label htmlFor="ai-device-name" className="text-base font-medium">
                  {t('devices:types.smart.nameRequired')} <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="ai-device-name"
                  value={aiDeviceName}
                  onChange={(e) => setAiDeviceName(e.target.value)}
                  placeholder={t('devices:types.smart.nameExample')}
                  className="h-11 text-base"
                />
                {aiDeviceName && (
                  <p className="text-sm text-muted-foreground">
                    <span className="font-medium">{t('devices:types.add.typeId')}:</span>{" "}
                    <code className="px-2 py-0.5 bg-muted rounded text-xs">{generateDeviceTypeId(aiDeviceName)}</code>
                  </p>
                )}
              </div>

              {/* Description */}
              <div className="space-y-2">
                <Label htmlFor="ai-device-desc" className="text-base font-medium">{t('devices:types.smart.description')}</Label>
                <Textarea
                  id="ai-device-desc"
                  value={aiDeviceDesc}
                  onChange={(e) => setAiDeviceDesc(e.target.value)}
                  placeholder={t('devices:types.smart.descExample')}
                  rows={2}
                  className="text-base resize-none"
                />
              </div>

              {/* Metrics and Commands - Side by Side */}
              <div className="grid grid-cols-2 gap-6">
                {/* Metrics Example */}
                <div className="space-y-3">
                  <Label className="flex items-center gap-2 text-base font-medium">
                    <div className="p-1.5 bg-green-100 dark:bg-green-900/30 rounded-md">
                      <ArrowDown className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    {t('devices:types.smart.metrics')}
                  </Label>
                  <Textarea
                    value={aiMetricsExample}
                    onChange={(e) => setAiMetricsExample(e.target.value)}
                    placeholder='{"temperature": 25.5, "humidity": 60, "battery": 85}'
                    rows={6}
                    className="font-mono text-sm bg-green-50/50 dark:bg-green-950/20 border-green-200 dark:border-green-900"
                  />
                  <p className="text-xs text-muted-foreground">{t('devices:types.smart.metricsHint')}</p>
                </div>

                {/* Commands Example */}
                <div className="space-y-3">
                  <Label className="flex items-center gap-2 text-base font-medium">
                    <div className="p-1.5 bg-blue-100 dark:bg-blue-900/30 rounded-md">
                      <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    {t('devices:types.smart.commands')}
                  </Label>
                  <Textarea
                    value={aiCommandsExample}
                    onChange={(e) => setAiCommandsExample(e.target.value)}
                    placeholder='{"action": "set_interval", "interval": 60}'
                    rows={6}
                    className="font-mono text-sm bg-blue-50/50 dark:bg-blue-950/20 border-blue-200 dark:border-blue-900"
                  />
                  <p className="text-xs text-muted-foreground">{t('devices:types.smart.commandsHint')}</p>
                </div>
              </div>

              {/* Generate Button */}
              <Button
                onClick={handleGenerateMDL}
                disabled={!aiDeviceName || generating}
                className="w-full h-12 text-base"
                size="lg"
              >
                {generating ? (
                  <>
                    <Sparkles className="mr-2 h-5 w-5 animate-spin" />
                    {t('devices:types.generating')}
                  </>
                ) : (
                  <>
                    <Wand2 className="mr-2 h-5 w-5" />
                    {t('devices:types.generate.title')}
                  </>
                )}
              </Button>
            </TabsContent>
          </Tabs>
        </div>

        <DialogFooter className="px-6 pb-6 pt-4 border-t">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={adding} className="h-10 px-6">
            {t('common:cancel')}
          </Button>
          <Button onClick={handleAdd} disabled={adding || (addTypeMode === "form" && !isFormValid())} className="h-10 px-6">
            {adding ? t('devices:types.adding') : t('common:add')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ============================================================================
// Device Type Form Component (reusable for Add/Edit)
// ============================================================================

interface DeviceTypeFormProps {
  data: DeviceType
  onChange: (data: DeviceType) => void
  onFieldChange?: <K extends keyof DeviceType>(field: K, value: DeviceType[K]) => void
  errors?: FormErrors
  touched?: Record<string, boolean>
  setTouched?: (touched: Record<string, boolean>) => void
  readonly?: boolean
}

function DeviceTypeForm({
  data,
  onChange,
  onFieldChange,
  errors = {},
  touched = {},
  setTouched,
  readonly = false
}: DeviceTypeFormProps) {
  const { t } = useTranslation(['common', 'devices'])

  // Local state for raw samples text (to allow typing incomplete JSON)
  const [rawSamplesText, setRawSamplesText] = useState<string>(
    (data.uplink_samples || []).map(s => JSON.stringify(s)).join('\n')
  )

  // Sync local state when data.uplink_samples changes from outside
  useEffect(() => {
    setRawSamplesText((data.uplink_samples || []).map(s => JSON.stringify(s)).join('\n'))
  }, [data.uplink_samples])

  const updateField = <K extends keyof DeviceType>(field: K, value: DeviceType[K]) => {
    if (onFieldChange) {
      onFieldChange(field, value)
    } else {
      onChange({ ...data, [field]: value })
    }
  }

  const markTouched = (field: string) => {
    if (setTouched && !readonly) {
      setTouched({ ...touched, [field]: true })
    }
  }

  const addMetric = () => {
    const metrics = data.metrics || []
    updateField('metrics', [
      ...metrics,
      {
        name: `metric_${metrics.length + 1}`,
        display_name: `Metric ${metrics.length + 1}`,
        data_type: "float",
      },
    ])
  }

  const updateMetric = (index: number, metric: MetricDefinition) => {
    const metrics = data.metrics || []
    const newMetrics = [...metrics]
    newMetrics[index] = metric
    updateField('metrics', newMetrics)
  }

  const removeMetric = (index: number) => {
    const metrics = data.metrics || []
    updateField('metrics', metrics.filter((_, i) => i !== index))
  }

  const addCommand = () => {
    const commands = data.commands || []
    updateField('commands', [
      ...commands,
      {
        name: `cmd_${commands.length + 1}`,
        display_name: `Command ${commands.length + 1}`,
        payload_template: '{"action": "${value}"}',
        parameters: [],
      },
    ])
  }

  const updateCommand = (index: number, command: CommandDefinition) => {
    const commands = data.commands || []
    const newCommands = [...commands]
    newCommands[index] = command
    updateField('commands', newCommands)
  }

  const removeCommand = (index: number) => {
    const commands = data.commands || []
    updateField('commands', commands.filter((_, i) => i !== index))
  }

  const hasFieldError = (field: string) => {
    return touched[field] && errors[field]
  }

  return (
    <div className="space-y-8">
      {/* Basic Info */}
      <div className="space-y-4">
        <h3 className="text-base font-semibold flex items-center gap-2">
          <span className="w-1 h-5 bg-primary rounded-full"></span>
          {t('devices:types.view.basicInfo')}
        </h3>

        <div className="grid grid-cols-2 gap-5">
          <div className="space-y-2">
            <Label htmlFor="type-id" className="text-sm font-medium">
              {t('devices:types.view.typeId')} <span className="text-destructive">*</span>
            </Label>
            <Input
              id="type-id"
              value={data.device_type}
              onChange={(e) => { markTouched('device_type'); updateField('device_type', e.target.value) }}
              placeholder="sensor_temp_humidity"
              disabled={readonly}
              className={cn(
                "h-10 font-mono",
                hasFieldError('device_type') && "border-destructive focus:border-destructive"
              )}
            />
            {hasFieldError('device_type') && (
              <p className="text-xs text-destructive flex items-center gap-1">
                <AlertCircle className="h-3 w-3" />
                {errors.device_type}
              </p>
            )}
          </div>
          <div className="space-y-2">
            <Label htmlFor="type-name" className="text-sm font-medium">
              {t('devices:types.view.name')} <span className="text-destructive">*</span>
            </Label>
            <Input
              id="type-name"
              value={data.name}
              onChange={(e) => { markTouched('name'); updateField('name', e.target.value) }}
              placeholder={t('devices:types.edit.typeNamePlaceholder')}
              disabled={readonly}
              className={cn(
                "h-10",
                hasFieldError('name') && "border-destructive focus:border-destructive"
              )}
            />
            {hasFieldError('name') && (
              <p className="text-xs text-destructive flex items-center gap-1">
                <AlertCircle className="h-3 w-3" />
                {errors.name}
              </p>
            )}
          </div>
        </div>

        <div className="space-y-2">
          <Label htmlFor="type-desc" className="text-sm font-medium">{t('devices:types.view.description')}</Label>
          <Textarea
            id="type-desc"
            value={data.description}
            onChange={(e) => updateField('description', e.target.value)}
            placeholder={t('devices:types.edit.descPlaceholder')}
            rows={2}
            disabled={readonly}
            className="resize-none"
          />
        </div>

        {/* Mode Selection */}
        {!readonly && (
          <div className="space-y-2">
            <Label className="text-sm font-medium">{t('devices:types.edit.definitionMode')}</Label>
            <div className="flex gap-4">
              <label className={cn(
                "flex items-center gap-2 px-4 py-2.5 rounded-lg border-2 cursor-pointer transition-all",
                data.mode !== 'simple'
                  ? "border-primary/50 bg-primary/5"
                  : "border-muted hover:border-muted-foreground/30"
              )}>
                <input
                  type="radio"
                  name="definition-mode"
                  checked={data.mode !== 'simple'}
                  onChange={() => updateField('mode', 'full')}
                  className="sr-only"
                />
                <Settings className={cn("h-4 w-4", data.mode !== 'simple' ? "text-primary" : "text-muted-foreground")} />
                <div className="text-sm">
                  <p className={cn("font-medium", data.mode !== 'simple' ? "text-foreground" : "text-muted-foreground")}>
                    {t('devices:types.mode.full')}
                  </p>
                  <p className="text-xs text-muted-foreground">{t('devices:types.mode.fullDescription')}</p>
                </div>
              </label>
              <label className={cn(
                "flex items-center gap-2 px-4 py-2.5 rounded-lg border-2 cursor-pointer transition-all",
                data.mode === 'simple'
                  ? "border-primary/50 bg-primary/5"
                  : "border-muted hover:border-muted-foreground/30"
              )}>
                <input
                  type="radio"
                  name="definition-mode"
                  checked={data.mode === 'simple'}
                  onChange={() => updateField('mode', 'simple')}
                  className="sr-only"
                />
                <Zap className={cn("h-4 w-4", data.mode === 'simple' ? "text-primary" : "text-muted-foreground")} />
                <div className="text-sm">
                  <p className={cn("font-medium", data.mode === 'simple' ? "text-foreground" : "text-muted-foreground")}>
                    {t('devices:types.mode.simple')}
                  </p>
                  <p className="text-xs text-muted-foreground">{t('devices:types.mode.simpleDescription')}</p>
                </div>
              </label>
            </div>
          </div>
        )}

        {/* Mode Display (readonly) */}
        {readonly && (
          <div className="space-y-2">
            <Label className="text-sm font-medium">{t('devices:types.edit.definitionMode')}</Label>
            <div className="flex items-center gap-2">
              {data.mode === 'simple' ? (
                <>
                  <Zap className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">{t('devices:types.mode.simple')}</span>
                  <span className="text-xs text-muted-foreground">- {t('devices:types.mode.simpleDescription')}</span>
                </>
              ) : (
                <>
                  <Settings className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">{t('devices:types.mode.full')}</span>
                  <span className="text-xs text-muted-foreground">- {t('devices:types.mode.fullDescription')}</span>
                </>
              )}
            </div>
          </div>
        )}

        <div className="space-y-2">
          <Label className="text-sm font-medium">{t('devices:types.view.categories')}</Label>
          <div className="flex gap-2 flex-wrap">
            {data.categories?.map((cat, i) => (
              <Badge key={i} variant="secondary" className="pl-2 pr-1 h-7">
                {cat}
                {!readonly && (
                  <button
                    onClick={() => {
                      updateField('categories', data.categories.filter((_, j) => j !== i))
                    }}
                    className="ml-1 hover:text-destructive"
                  >
                    ×
                  </button>
                )}
              </Badge>
            ))}
            {!readonly && (
              <Input
                placeholder="+ Add category"
                className="h-7 w-36 text-xs"
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && e.currentTarget.value.trim()) {
                    updateField('categories', [...data.categories, e.currentTarget.value.trim()])
                    e.currentTarget.value = ''
                  }
                }}
              />
            )}
          </div>
        </div>
      </div>

      {/* Metrics Section - different content based on mode */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-base font-semibold flex items-center gap-2">
            <span className="w-1 h-5 bg-green-500 rounded-full"></span>
            {data.mode === 'simple'
              ? t('devices:types.edit.uplinkSamples')
              : `${t('devices:types.view.metrics')} (${(data.metrics || []).length})`
            }
          </h3>
          {!readonly && data.mode !== 'simple' && (
            <Button onClick={addMetric} size="sm" variant="outline" className="h-9">
              <Plus className="mr-2 h-4 w-4" />
              {t('devices:types.edit.addMetric')}
            </Button>
          )}
        </div>

        {/* Simple Mode: Show samples textarea */}
        {data.mode === 'simple' && (
          <div className="rounded-lg border p-4 space-y-3 bg-muted/30">
            <p className="text-sm text-muted-foreground">
              {t('devices:types.edit.samplesDescription')}
            </p>
            <Textarea
              value={rawSamplesText}
              onChange={(e) => setRawSamplesText(e.target.value)}
              onBlur={() => {
                const lines = rawSamplesText.trim().split('\n').filter(l => l.trim())
                const samples: Record<string, unknown>[] = []
                for (const line of lines) {
                  try {
                    samples.push(JSON.parse(line))
                  } catch {
                    // Skip invalid JSON lines
                  }
                }
                updateField('uplink_samples', samples)
              }}
              placeholder='{"temperature": 25.5, "humidity": 60}\n{"battery": 85}'
              rows={6}
              className="font-mono text-sm resize-y"
              disabled={readonly}
            />
            <p className="text-xs text-muted-foreground">
              {t('devices:types.edit.samplesHint')}
            </p>
          </div>
        )}

        {/* Full Mode: Show metrics list */}
        {data.mode !== 'simple' && (data.metrics || []).length === 0 ? (
          <div className="text-center py-8 border-2 border-dashed rounded-lg">
            <p className="text-sm text-muted-foreground">{t('devices:types.edit.noMetrics')}</p>
            <p className="text-xs text-muted-foreground mt-1">Click "Add Metric" to create your first metric</p>
          </div>
        ) : data.mode !== 'simple' ? (
          <div className="space-y-3">
            {(data.metrics || []).map((metric, i) => (
              <MetricEditor
                key={i}
                metric={metric}
                onChange={(m) => updateMetric(i, m)}
                onRemove={() => removeMetric(i)}
                readonly={readonly}
                error={errors.metrics?.[i]}
              />
            ))}
          </div>
        ) : null}
      </div>

      {/* Commands */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-base font-semibold flex items-center gap-2">
            <span className="w-1 h-5 bg-blue-500 rounded-full"></span>
            {t('devices:types.view.commands')} ({(data.commands || []).length})
          </h3>
          {!readonly && (
            <Button onClick={addCommand} size="sm" variant="outline" className="h-9">
              <Plus className="mr-2 h-4 w-4" />
              {t('devices:types.edit.addCommand')}
            </Button>
          )}
        </div>

        {(data.commands || []).length === 0 ? (
          <div className="text-center py-8 border-2 border-dashed rounded-lg">
            <p className="text-sm text-muted-foreground">{t('devices:types.edit.noCommands')}</p>
            <p className="text-xs text-muted-foreground mt-1">Click "Add Command" to create your first command</p>
          </div>
        ) : (
          <div className="space-y-3">
            {(data.commands || []).map((cmd, i) => (
              <CommandEditor
                key={i}
                command={cmd}
                onChange={(c) => updateCommand(i, c)}
                onRemove={() => removeCommand(i)}
                readonly={readonly}
                error={errors.commands?.[i]}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Metric Editor Component
// ============================================================================

interface MetricEditorProps {
  metric: MetricDefinition
  onChange: (metric: MetricDefinition) => void
  onRemove: () => void
  readonly?: boolean
  error?: string
}

function MetricEditor({ metric, onChange, onRemove, readonly = false, error }: MetricEditorProps) {
  const { t } = useTranslation(['common', 'devices'])
  const [expanded, setExpanded] = useState(true)

  const updateField = <K extends keyof MetricDefinition>(field: K, value: MetricDefinition[K]) => {
    onChange({ ...metric, [field]: value })
  }

  return (
    <div className={cn(
      "rounded-lg border p-4 space-y-3 transition-all",
      error && "border-destructive bg-destructive/5",
      !error && "bg-card"
    )}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          </Button>
          <span className="font-medium text-sm">{metric.display_name || metric.name}</span>
          <Badge variant="outline" className="text-xs">{metric.data_type}</Badge>
          {metric.unit && <Badge variant="secondary" className="text-xs">{metric.unit}</Badge>}
        </div>
        {!readonly && (
          <Button variant="ghost" size="icon" onClick={onRemove} className="h-8 w-8">
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        )}
      </div>

      {error && (
        <p className="text-xs text-destructive flex items-center gap-1">
          <AlertCircle className="h-3 w-3" />
          {error}
        </p>
      )}

      {expanded && (
        <div className="space-y-3">
          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-1">
              <Label className="text-xs font-medium">
                {t('devices:types.edit.fieldName')} <span className="text-destructive">*</span>
              </Label>
              <Input
                value={metric.name}
                onChange={(e) => updateField('name', e.target.value)}
                placeholder="temperature"
                className={cn(
                  "h-9 text-sm",
                  !metric.name?.trim() && "border-destructive"
                )}
                disabled={readonly}
              />
              <p className="text-xs text-muted-foreground">
                {t('devices:types.edit.fieldNameHint')}
              </p>
            </div>
            <div className="space-y-1">
              <Label className="text-xs font-medium">{t('devices:types.edit.displayName')}</Label>
              <Input
                value={metric.display_name}
                onChange={(e) => updateField('display_name', e.target.value)}
                placeholder={t('devices:types.edit.displayNamePlaceholder')}
                className="h-9 text-sm"
                disabled={readonly}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs font-medium">{t('devices:types.edit.dataType')}</Label>
              <Select
                value={metric.data_type}
                onValueChange={(v) => updateField('data_type', v as any)}
                disabled={readonly}
              >
                <SelectTrigger className="h-9 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="string">String</SelectItem>
                  <SelectItem value="integer">Integer</SelectItem>
                  <SelectItem value="float">Float</SelectItem>
                  <SelectItem value="boolean">Boolean</SelectItem>
                  <SelectItem value="binary">Binary</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-1">
              <Label className="text-xs font-medium">{t('devices:types.edit.unit')}</Label>
              <Input
                value={metric.unit || ""}
                onChange={(e) => updateField('unit', e.target.value)}
                placeholder="°C"
                className="h-9 text-sm"
                disabled={readonly}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs font-medium">Min</Label>
              <Input
                type="number"
                value={metric.min ?? ""}
                onChange={(e) => updateField('min', e.target.value ? Number(e.target.value) : undefined)}
                placeholder="-40"
                className="h-9 text-sm"
                disabled={readonly}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs font-medium">Max</Label>
              <Input
                type="number"
                value={metric.max ?? ""}
                onChange={(e) => updateField('max', e.target.value ? Number(e.target.value) : undefined)}
                placeholder="100"
                className="h-9 text-sm"
                disabled={readonly}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

// ============================================================================
// Command Editor Component
// ============================================================================

interface CommandEditorProps {
  command: CommandDefinition
  onChange: (command: CommandDefinition) => void
  onRemove: () => void
  readonly?: boolean
  error?: string
}

function CommandEditor({ command, onChange, onRemove, readonly = false, error }: CommandEditorProps) {
  const { t } = useTranslation(['common', 'devices'])
  const [expanded, setExpanded] = useState(true)

  // Local state for raw command samples text (to allow typing incomplete JSON)
  const [rawSamplesText, setRawSamplesText] = useState<string>(
    (command.samples || []).map(s => JSON.stringify(s)).join('\n')
  )

  // Sync local state when command.samples changes from outside
  useEffect(() => {
    setRawSamplesText((command.samples || []).map(s => JSON.stringify(s)).join('\n'))
  }, [command.samples])

  const updateField = <K extends keyof CommandDefinition>(field: K, value: CommandDefinition[K]) => {
    onChange({ ...command, [field]: value })
  }

  const addParameter = () => {
    updateField('parameters', [
      ...command.parameters,
      {
        name: `param_${command.parameters.length + 1}`,
        display_name: `Parameter ${command.parameters.length + 1}`,
        data_type: "string",
      },
    ])
  }

  const updateParameter = (index: number, param: ParameterDefinition) => {
    const newParams = [...command.parameters]
    newParams[index] = param
    updateField('parameters', newParams)
  }

  const removeParameter = (index: number) => {
    updateField('parameters', command.parameters.filter((_, i) => i !== index))
  }

  return (
    <div className={cn(
      "rounded-lg border p-4 space-y-3 transition-all",
      error && "border-destructive bg-destructive/5",
      !error && "bg-card"
    )}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          </Button>
          <span className="font-medium text-sm">{command.display_name || command.name}</span>
          <Badge variant="secondary" className="text-xs">
            {command.parameters.length} {t('devices:types.view.params')}
          </Badge>
        </div>
        {!readonly && (
          <Button variant="ghost" size="icon" onClick={onRemove} className="h-8 w-8">
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        )}
      </div>

      {error && (
        <p className="text-xs text-destructive flex items-center gap-1">
          <AlertCircle className="h-3 w-3" />
          {error}
        </p>
      )}

      {expanded && (
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label className="text-xs font-medium">
                {t('devices:types.edit.cmdName')} <span className="text-destructive">*</span>
              </Label>
              <Input
                value={command.name}
                onChange={(e) => updateField('name', e.target.value)}
                placeholder="set_interval"
                className={cn(
                  "h-9 text-sm",
                  !command.name?.trim() && "border-destructive"
                )}
                disabled={readonly}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs font-medium">{t('devices:types.edit.displayName')}</Label>
              <Input
                value={command.display_name}
                onChange={(e) => updateField('display_name', e.target.value)}
                placeholder={t('devices:types.edit.cmdDisplayPlaceholder')}
                className="h-9 text-sm"
                disabled={readonly}
              />
            </div>
          </div>

          <div className="space-y-1">
            <Label className="text-xs font-medium">{t('devices:types.edit.payloadTemplate')}</Label>
            <Input
              value={command.payload_template}
              onChange={(e) => updateField('payload_template', e.target.value)}
              placeholder='{"action": "${param}"}'
              className="h-9 font-mono text-sm"
              disabled={readonly}
            />
            <p className="text-xs text-muted-foreground">{t('devices:types.edit.payloadTemplateHint')}</p>
          </div>

          {/* LLM Hints */}
          <div className="space-y-1">
            <Label className="text-xs font-medium">{t('devices:types.edit.llmHints')}</Label>
            <Textarea
              value={command.llm_hints || ""}
              onChange={(e) => updateField('llm_hints', e.target.value)}
              placeholder={t('devices:types.edit.llmHintsPlaceholder')}
              rows={2}
              className="text-sm resize-none"
              disabled={readonly}
            />
            <p className="text-xs text-muted-foreground">{t('devices:types.edit.llmHintsDescription')}</p>
          </div>

          {/* Command Samples */}
          <div className="space-y-1">
            <Label className="text-xs font-medium">{t('devices:types.edit.commandSamples')}</Label>
            <Textarea
              value={rawSamplesText}
              onChange={(e) => setRawSamplesText(e.target.value)}
              onBlur={() => {
                const lines = rawSamplesText.trim().split('\n').filter(l => l.trim())
                const samples: Record<string, unknown>[] = []
                for (const line of lines) {
                  try {
                    samples.push(JSON.parse(line))
                  } catch {
                    // Skip invalid JSON lines
                  }
                }
                updateField('samples', samples)
              }}
              placeholder='{"action": "on"}\n{"action": "off", "interval": 60}'
              rows={3}
              className="font-mono text-sm resize-y"
              disabled={readonly}
            />
            <p className="text-xs text-muted-foreground">{t('devices:types.edit.commandSamplesHint')}</p>
          </div>

          {/* Parameters */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium">{t('devices:types.view.parameters')}</Label>
              {!readonly && (
                <Button onClick={addParameter} size="sm" variant="ghost" className="h-7">
                  <Plus className="mr-1 h-3 w-3" />
                  {t('common:add')}
                </Button>
              )}
            </div>

            {command.parameters.length === 0 ? (
              <p className="text-xs text-muted-foreground text-center py-3 border border-dashed rounded-md bg-muted/30">
                {t('devices:types.edit.noParameters')}
              </p>
            ) : (
              <div className="space-y-2 pl-2 border-l-2 border-muted">
                {command.parameters.map((param, i) => (
                  <ParameterEditor
                    key={i}
                    parameter={param}
                    onChange={(p) => updateParameter(i, p)}
                    onRemove={() => removeParameter(i)}
                    readonly={readonly}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}

// ============================================================================
// Parameter Editor Component
// ============================================================================

interface ParameterEditorProps {
  parameter: ParameterDefinition
  onChange: (parameter: ParameterDefinition) => void
  onRemove: () => void
  readonly?: boolean
}

function ParameterEditor({ parameter, onChange, onRemove, readonly = false }: ParameterEditorProps) {
  const { t } = useTranslation(['common', 'devices'])

  const updateField = <K extends keyof ParameterDefinition>(field: K, value: ParameterDefinition[K]) => {
    onChange({ ...parameter, [field]: value })
  }

  const getDisplayValue = () => {
    if (!parameter.default_value) return ''
    const dv = parameter.default_value as any
    if (dv.String !== undefined) return dv.String
    if (dv.Integer !== undefined) return dv.Integer.toString()
    if (dv.Float !== undefined) return dv.Float.toString()
    if (dv.Boolean !== undefined) return dv.Boolean.toString()
    return ''
  }

  return (
    <div className="flex items-start gap-2">
      {!readonly && (
        <Button variant="ghost" size="icon" onClick={onRemove} className="h-7 w-7 mt-0.5">
          <Trash2 className="h-3 w-3 text-destructive" />
        </Button>
      )}
      <div className="flex-1 grid grid-cols-4 gap-2">
        <div className="space-y-1">
          <Input
            value={parameter.name}
            onChange={(e) => updateField('name', e.target.value)}
            placeholder="name"
            className="h-8 text-xs"
            disabled={readonly}
          />
        </div>
        <div className="space-y-1">
          <Input
            value={parameter.display_name || ""}
            onChange={(e) => updateField('display_name', e.target.value)}
            placeholder={t('devices:types.edit.displayName')}
            className="h-8 text-xs"
            disabled={readonly}
          />
        </div>
        <Select
          value={parameter.data_type}
          onValueChange={(v) => updateField('data_type', v)}
          disabled={readonly}
        >
          <SelectTrigger className="h-8 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="string">String</SelectItem>
            <SelectItem value="integer">Integer</SelectItem>
            <SelectItem value="float">Float</SelectItem>
            <SelectItem value="boolean">Boolean</SelectItem>
          </SelectContent>
        </Select>
        <Input
          value={getDisplayValue()}
          onChange={(e) => {
            const val = e.target.value
            let newValue: any = undefined
            switch (parameter.data_type) {
              case 'integer':
                const num = parseInt(val, 10)
                if (!isNaN(num)) newValue = { Integer: num }
                break
              case 'float':
                const float = parseFloat(val)
                if (!isNaN(float)) newValue = { Float: float }
                break
              case 'boolean':
                if (val === 'true') newValue = { Boolean: true }
                else if (val === 'false') newValue = { Boolean: false }
                else newValue = { String: val }
                break
              default:
                newValue = { String: val }
            }
            updateField('default_value', newValue)
          }}
          placeholder={t('devices:types.edit.defaultValue')}
          className="h-8 text-xs"
          disabled={readonly}
        />
      </div>
    </div>
  )
}

// ============================================================================
// View Device Type Dialog (readonly, full screen)
// ============================================================================

interface ViewDeviceTypeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  deviceType: DeviceType | null
}

export function ViewDeviceTypeDialog({ open, onOpenChange, deviceType }: ViewDeviceTypeDialogProps) {
  const { t } = useTranslation(['common', 'devices'])

  if (!deviceType) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl h-[90vh] max-h-[90vh] flex flex-col">
        <DialogHeader className="border-b pb-4">
          <DialogTitle className="text-xl">{t('devices:types.view.title')}</DialogTitle>
        </DialogHeader>
        <div className="flex-1 overflow-y-auto -mx-6 px-6">
          <DeviceTypeForm key={deviceType.device_type} data={deviceType} onChange={() => {}} readonly />
        </div>
        <DialogFooter className="border-t pt-4">
          <Button onClick={() => onOpenChange(false)} className="h-10 px-6">{t('common:close')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ============================================================================
// Edit Device Type Dialog (uses DeviceTypeForm, full screen)
// ============================================================================

interface EditDeviceTypeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  deviceType: DeviceType | null
  onEdit: (data: DeviceType) => Promise<boolean>
  editing: boolean
}

export function EditDeviceTypeDialog({ open, onOpenChange, deviceType, onEdit, editing }: EditDeviceTypeDialogProps) {
  const { t } = useTranslation(['common', 'devices'])

  const [formData, setFormData] = useState<DeviceType>({
    device_type: "",
    name: "",
    description: "",
    categories: [],
    mode: "simple",
    metrics: [],
    commands: [],
    uplink_samples: [],
  })
  const [formErrors, setFormErrors] = useState<FormErrors>({})
  const [touched, setTouched] = useState<Record<string, boolean>>({})

  // Sync with props when dialog opens or deviceType changes
  useEffect(() => {
    if (deviceType) {
      setFormData(deviceType)
      setFormErrors({})
      setTouched({})
    }
  }, [deviceType, open])

  // Validate form in real-time
  useEffect(() => {
    const errors: FormErrors = {}

    if (touched.device_type || touched.name) {
      if (!formData.device_type?.trim()) {
        errors.device_type = t('devices:types.validation.deviceTypeRequired')
      }
      if (!formData.name?.trim()) {
        errors.name = t('devices:types.validation.nameRequired')
      }
    }

    const metricErrors: Record<number, string> = {}
    formData.metrics?.forEach((metric, i) => {
      if (!metric.name?.trim()) {
        metricErrors[i] = t('devices:types.validation.metricNameRequired')
      }
    })
    if (Object.keys(metricErrors).length > 0) {
      errors.metrics = metricErrors
    }

    const commandErrors: Record<number, string> = {}
    formData.commands?.forEach((cmd, i) => {
      if (!cmd.name?.trim()) {
        commandErrors[i] = t('devices:types.validation.commandNameRequired')
      }
    })
    if (Object.keys(commandErrors).length > 0) {
      errors.commands = commandErrors
    }

    setFormErrors(errors)
  }, [formData, touched, t])

  const isFormValid = () => {
    // Basic validation
    if (!formData.device_type?.trim() || !formData.name?.trim()) {
      return false
    }

    // For Simple mode, metrics are optional (using samples instead)
    const isSimpleMode = formData.mode === "simple"

    // For Full mode, validate metrics
    if (!isSimpleMode && formData.metrics?.some(m => !m.name?.trim())) {
      return false
    }

    // Commands are required in both modes
    if (formData.commands?.some(c => !c.name?.trim())) {
      return false
    }

    return true
  }

  const handleFieldChange = <K extends keyof DeviceType>(field: K, value: DeviceType[K]) => {
    setTouched(prev => ({ ...prev, [field]: true }))
    setFormData(prev => ({ ...prev, [field]: value }))
  }

  const handleEdit = async () => {
    if (!isFormValid()) {
      setTouched({ device_type: true, name: true })
      toast({
        title: t('devices:types.validationError'),
        description: t('devices:types.requiredFields'),
        variant: "destructive",
      })
      return
    }

    const success = await onEdit(formData)
    if (success) {
      onOpenChange(false)
      toast({
        title: t('devices:types.save.success'),
        description: t('devices:types.save.updated', { name: formData.name }),
      })
    } else {
      toast({
        title: t('devices:types.save.error'),
        description: t('devices:types.save.retry'),
        variant: "destructive",
      })
    }
  }

  if (!deviceType) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-5xl h-[90vh] max-h-[90vh] flex flex-col p-0">
        <DialogHeader className="px-6 pt-6 pb-4 border-b">
          <DialogTitle className="text-xl">{t('devices:types.edit.title')}</DialogTitle>
        </DialogHeader>
        <div className="flex-1 overflow-y-auto px-6">
          <DeviceTypeForm
            data={formData}
            onChange={setFormData}
            onFieldChange={handleFieldChange}
            errors={formErrors}
            touched={touched}
            setTouched={setTouched}
          />
        </div>
        <DialogFooter className="px-6 pb-6 pt-4 border-t">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={editing} className="h-10 px-6">
            {t('common:cancel')}
          </Button>
          <Button onClick={handleEdit} disabled={!isFormValid() || editing} className="h-10 px-6">
            {editing ? t('devices:types.edit.saving') : t('common:save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
