/**
 * TransformBuilderSplit Component
 *
 * Full-screen dialog for creating/editing data transforms.
 * Following the same pattern as AgentEditorFullScreen.
 *
 * @module automation
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { createPortal } from 'react-dom'
import { api } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useBodyScrollLock } from '@/hooks/useBodyScrollLock'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { CodeEditor } from '@/components/ui/code-editor'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Code,
  Loader2,
  Play,
  Database,
  FlaskConical,
  Settings,
  ChevronLeft,
  ChevronRight,
  Check,
  Puzzle,
  FileCode,
  Info,
  ChevronDown,
  X,
  Eye,
  Lightbulb,
  Zap,
  Globe,
  Clock,
} from 'lucide-react'
import type {
  TransformAutomation,
  TransformScope,
  Extension,
  ExtensionV2DataSourceInfo,
  ExtensionDataType,
  ExtensionAggFunc,
} from '@/types'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs'

// ============================================================================
// Types
// ============================================================================

interface TransformBuilderProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  transform?: TransformAutomation | null
  devices: Array<{ id: string; name: string; device_type?: string }>
  onSave: (data: Partial<TransformAutomation>) => void
}

type Step = 'basic' | 'code' | 'test'
type ScopeType = 'global' | 'device_type' | 'device'

interface FormErrors {
  name?: string
  code?: string
  outputPrefix?: string
  scopeValue?: string
}

// Selected extension data source
interface SelectedExtensionSource {
  extension_id: string
  extension_name: string
  command: string
  command_display_name?: string
  field: string
  display_name: string
  data_type: ExtensionDataType
  unit?: string
}

// Extension data source with nested structure
interface ExtensionDataSourceGroup {
  extension_id: string
  extension_name: string
  commands: Array<{
    command: string
    command_display_name: string
    fields: Array<{
      field: string
      display_name: string
      data_type: ExtensionDataType
      unit?: string
    }>
  }>
}

// Extension command for invocation
interface ExtensionCommandInfo {
  extension_id: string
  extension_name: string
  command_id: string
  command_name: string
  display_name: string
  description: string
  // Parameters extracted from input_schema
  parameters: Array<{
    name: string
    display_name: string
    data_type: string
    required: boolean
    default_value?: unknown
    description?: string
  }>
}

// Code templates for common data transformations
// Note: names will be translated via i18n
const CODE_TEMPLATES = [
  {
    key: 'simple',
    nameKey: 'templates.simple',
    code: '// Device metric: input.temperature (Celsius)\nreturn {\n  temp_f: (input.temperature || 0) * 9 / 5 + 32,\n  status: (input.temperature || 0) > 30 ? \'hot\' : \'normal\'\n};',
  },
  {
    key: 'extensionOnly',
    nameKey: 'templates.extensionOnly',
    code: '// Use extension data from selected sources\n// Access via: input.extensions.{ext_id}.{command}.{field}\nconst ext = input.extensions || {};\n\n// Example: weather extension data\nconst weather = ext.weather?.get_current || {};\nreturn {\n  outdoor_temp: weather.temp_f || 0,\n  condition: weather.condition || \'unknown\',\n  has_data: Object.keys(weather).length > 0\n};',
  },
  {
    key: 'combined',
    nameKey: 'templates.combined',
    code: '// Combine device metrics and extension data\nconst deviceTemp = input.temperature || 0;\nconst ext = input.extensions || {};\n\n// Get extension data (check if exists first)\nconst extData = ext[Object.keys(ext)[0]] || {};\nconst firstCommand = extData[Object.keys(extData)[0]] || {};\nconst extTemp = firstCommand.temp_f || deviceTemp;\n\nreturn {\n  // Average of device and extension temperature\n  temp_avg: (deviceTemp + extTemp * 5/9) / 2,\n  // Status based on combined reading\n  is_hot: deviceTemp > 30 || extTemp > 90,\n  // Difference\n  temp_diff: Math.abs(deviceTemp - extTemp * 5 / 9)\n};',
  },
  {
    key: 'batteryStatus',
    nameKey: 'templates.batteryStatus',
    code: '// Battery level status calculation\nconst battery = input.battery || input.batt || input.level || 0;\nreturn {\n  battery_percent: Math.min(100, Math.max(0, battery)),\n  battery_status: battery > 80 ? \'good\' : battery > 20 ? \'medium\' : \'low\',\n  needs_charging: battery < 20\n};',
  },
  {
    key: 'statusCheck',
    nameKey: 'templates.statusCheck',
    code: '// Multi-level status check\nconst value = input.value || input.val || input.temperature || 0;\nreturn {\n  status: value > 100 ? \'critical\' : value > 80 ? \'warning\' : \'normal\',\n  is_alert: value > 100,\n  severity: value > 100 ? 3 : value > 80 ? 2 : 1,\n  normalized: Math.min(100, Math.max(0, value))\n};',
  },
  {
    key: 'passThrough',
    nameKey: 'templates.passThrough',
    code: '// Pass through all input data unchanged\nreturn input;',
  },
  {
    key: 'extensionInvoke',
    nameKey: 'templates.extensionInvoke',
    code: '// Call an extension action to process data\n// Replace {extension_id} and {command} with actual values\n// Click on an action in the Variables Panel to auto-generate code\nconst result = extensions_invoke(\'{extension_id}\', \'{command}\', {\n  data: input,\n})\n\nreturn result',
  },
]

// ============================================================================
// Variables Panel Component (with Tabs)
// ============================================================================

interface VariablesPanelProps {
  deviceTypeMetrics?: Array<{ name: string; display_name: string; data_type: string; unit?: string }>
  extensionSources?: SelectedExtensionSource[]
  allExtensionSources?: ExtensionDataSourceGroup[]
  onExtensionSourcesChange: (sources: SelectedExtensionSource[]) => void
  scopeType: ScopeType
  tBuilder: (key: string) => string
  t: (key: string, params?: Record<string, unknown>) => string
  onInsertVariable?: (variable: string) => void
  isMobile?: boolean
}

function VariablesPanel({
  deviceTypeMetrics,
  extensionSources,
  allExtensionSources,
  onExtensionSourcesChange,
  scopeType,
  tBuilder,
  t,
  onInsertVariable,
  isMobile = false,
}: VariablesPanelProps) {
  const [extensions, setExtensions] = useState<ExtensionDataSourceGroup[]>([])
  const [extensionCommands, setExtensionCommands] = useState<ExtensionCommandInfo[]>([])
  const [loadingExtensions, setLoadingExtensions] = useState(true)
  const [extensionPopoverOpen, setExtensionPopoverOpen] = useState(false)

  // Fetch extension data sources and commands when needed
  useEffect(() => {
    const fetchSources = async () => {
      setLoadingExtensions(true)
      try {
        const [allSources, extList] = await Promise.all([
          api.listAllDataSources(),
          api.listExtensions()
        ])

        // Filter only extension data sources (exclude transform data sources)
        const extensionSources = allSources.filter(
          (source): source is import('@/types').ExtensionDataSourceInfo =>
            'extension_id' in source
        )

        // Group data sources by extension
        const groups: Record<string, ExtensionDataSourceGroup> = {}
        for (const source of extensionSources) {
          const key = source.extension_id
          if (!groups[key]) {
            const ext = extList.find(e => e.id === source.extension_id)

            groups[key] = {
              extension_id: source.extension_id,
              extension_name: ext?.name || source.extension_id,
              commands: [], // For V2, we use a single "metrics" group
            }
          }

          // In V2, command is empty, use "metrics" as the display name
          const cmdKey = source.command || 'metrics'
          let cmdGroup = groups[key].commands.find(c => c.command === cmdKey)
          if (!cmdGroup) {
            cmdGroup = {
              command: cmdKey,
              command_display_name: 'Metrics',
              fields: [],
            }
            groups[key].commands.push(cmdGroup)
          }

          cmdGroup.fields.push({
            field: source.field,
            display_name: source.display_name,
            data_type: source.data_type as ExtensionDataType,
            unit: source.unit,
          })
        }

        setExtensions(Object.values(groups))

        // Collect extension commands (for invocation) with parameter info
        const commands: ExtensionCommandInfo[] = []
        for (const ext of extList) {
          for (const cmd of ext.commands || []) {
            // Skip metrics commands (data sources) - only show executable commands
            if (cmd.id === 'metrics' || cmd.id === 'get_current') {
              continue
            }

            // Extract parameters from input_schema
            const parameters = extractParametersFromSchema(cmd.input_schema)

            commands.push({
              extension_id: ext.id,
              extension_name: ext.name,
              command_id: cmd.id,
              command_name: cmd.id,
              display_name: cmd.display_name || cmd.id,
              description: cmd.description || '',
              parameters,
            })
          }
        }
        setExtensionCommands(commands)
      } catch (err) {
        console.error('Failed to load extension data sources:', err)
      } finally {
        setLoadingExtensions(false)
      }
    }

    if (extensionPopoverOpen && extensions.length === 0) {
      fetchSources()
    }
  }, [extensionPopoverOpen, extensions.length])

  // Extract parameters from JSON schema
  const extractParametersFromSchema = (schema: Record<string, unknown> | undefined): ExtensionCommandInfo['parameters'] => {
    if (!schema || typeof schema !== 'object') {
      return []
    }

    const params: ExtensionCommandInfo['parameters'] = []
    const properties = schema.properties as Record<string, unknown> | undefined
    const required = (schema.required as string[]) || []

    if (!properties) {
      return params
    }

    for (const [name, propSchema] of Object.entries(properties)) {
      if (typeof propSchema !== 'object' || propSchema === null) {
        continue
      }

      const prop = propSchema as Record<string, unknown>

      // Determine data type
      let dataType = 'string'
      const type = prop.type as string | undefined
      if (type === 'number' || type === 'integer') {
        dataType = 'number'
      } else if (type === 'boolean') {
        dataType = 'boolean'
      } else if (type === 'array') {
        dataType = 'array'
      } else if (type === 'object') {
        dataType = 'object'
      }

      params.push({
        name,
        display_name: (prop.title as string) || name,
        data_type: dataType,
        required: required.includes(name),
        default_value: prop.default,
        description: prop.description as string | undefined,
      })
    }

    return params
  }

  const isSourceSelected = (extId: string, field: string) => {
    return extensionSources?.some(s => s.extension_id === extId && s.field === field)
  }

  const toggleSource = (extId: string, extName: string, field: string, display: string, dataType: ExtensionDataType, unit: string | undefined) => {
    const currentSources = extensionSources || []
    const key = `${extId}/${field}`
    if (isSourceSelected(extId, field)) {
      onExtensionSourcesChange(currentSources.filter(s => `${s.extension_id}/${s.field}` !== key))
    } else {
      onExtensionSourcesChange([...currentSources, {
        extension_id: extId,
        extension_name: extName,
        command: 'metrics', // V2: fixed value
        command_display_name: 'Metrics',
        field,
        display_name: display,
        data_type: dataType,
        unit,
      }])
    }
  }

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'number': case 'integer': case 'float': return 'text-blue-500'
      case 'string': return 'text-green-500'
      case 'boolean': return 'text-purple-500'
      case 'object': return 'text-orange-500'
      case 'array': return 'text-cyan-500'
      case 'binary': return 'text-yellow-500'
      default: return 'text-gray-500'
    }
  }

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'number': case 'integer': case 'float': return '#'
      case 'string': return '"'
      case 'boolean': return 'TF'
      case 'object': return '{}'
      case 'array': return '[]'
      case 'binary': return 'BIN'
      default: return '?'
    }
  }

  // Generate code to call an extension command
  const handleInvokeCommand = (cmdInfo: ExtensionCommandInfo) => {
    // Build parameters object with all params
    let paramsCode = '  data: input'
    if (cmdInfo.parameters.length > 0) {
      const otherParams = cmdInfo.parameters
        .filter(p => p.name !== 'data')
        .map(p => {
          const comment = p.required ? '' : ` // optional${p.description ? ` - ${p.description}` : ''}`
          const value = p.default_value !== undefined ? JSON.stringify(p.default_value) : `/* ${p.display_name} */`
          return `\n  ${p.name}: ${value},${comment}`
        })
        .join('')
      if (otherParams) {
        paramsCode += ',' + otherParams
      }
    }

    const hasParams = cmdInfo.parameters.length > 0
    const resultParam = hasParams ? 'result' : '{ processed: true }'

    const codeTemplate = `// Call ${cmdInfo.extension_name}: ${cmdInfo.display_name}
// ${cmdInfo.description || 'Execute extension command'}
const ${resultParam} = extensions_invoke('${cmdInfo.extension_id}', '${cmdInfo.command_name}', {
${paramsCode}
})

return ${resultParam}`

    onInsertVariable?.(codeTemplate)
  }

  // Group extension sources by extension and command
  const groupedExtensions = useMemo(() => {
    const groups: Record<string, Record<string, SelectedExtensionSource[]>> = {}
    for (const source of extensionSources || []) {
      if (!groups[source.extension_id]) {
        groups[source.extension_id] = {}
      }
      if (!groups[source.extension_id][source.command]) {
        groups[source.extension_id][source.command] = []
      }
      groups[source.extension_id][source.command].push(source)
    }
    return groups
  }, [extensionSources])

  return (
    <div className={cn(
      "border rounded-lg overflow-hidden bg-background",
      isMobile ? "w-full" : "w-72"
    )}>
      {/* Header */}
      <div className={cn(
        "border-b bg-muted/30 flex items-center gap-2",
        isMobile ? "px-4 py-3" : "px-3 py-2"
      )}>
        <Database className={cn("text-blue-500", isMobile ? "h-5 w-5" : "h-4 w-4")} />
        <span className={cn("font-medium", isMobile ? "text-base" : "text-sm")}>{tBuilder('availableVars')}</span>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="device" className={cn(isMobile ? "h-[350px]" : "h-[400px]")}>
        <div className={cn(isMobile ? "px-4 pt-3" : "px-3 pt-2")}>
          <TabsList className={cn(
            "w-full gap-0.5",
            isMobile ? "h-10" : "h-8 px-0.5"
          )}>
            <TabsTrigger value="device" className={cn(
              "flex-1",
              isMobile ? "h-9 px-3 text-sm" : "h-7 px-2 text-xs"
            )}>
              <Database className={cn(isMobile ? "h-4 w-4" : "h-3 w-3", "mr-1")} />
              {tBuilder('device') || 'Device'}
            </TabsTrigger>
            <TabsTrigger value="extension" className={cn(
              "flex-1",
              isMobile ? "h-9 px-3 text-sm" : "h-7 px-2 text-xs"
            )}>
              <Puzzle className={cn(isMobile ? "h-4 w-4" : "h-3 w-3", "mr-1")} />
              {tBuilder('extension') || 'Extension'}
            </TabsTrigger>
          </TabsList>
        </div>

        {/* Device Metrics Tab */}
        <TabsContent value="device" className={cn(
          "overflow-y-auto overflow-x-hidden mt-0",
          isMobile ? "h-[290px] p-3" : "h-[340px] p-2"
        )}>
          {deviceTypeMetrics && deviceTypeMetrics.length > 0 ? (
            <div className={cn("space-y-1", isMobile ? "space-y-2" : "")}>
              {deviceTypeMetrics.map((metric, idx) => (
                <div
                  key={idx}
                  className={cn(
                    "flex items-center justify-between bg-background border rounded hover:bg-primary/5 hover:border-primary/30 transition-all cursor-pointer group",
                    isMobile ? "px-4 py-3" : "px-2 py-1.5"
                  )}
                  onClick={() => onInsertVariable?.(`input.${metric.name}`)}
                >
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    <code className={cn(
                      "font-mono text-blue-600 dark:text-blue-400 truncate",
                      isMobile ? "text-sm" : "text-xs"
                    )}>
                      {metric.name}
                    </code>
                    {metric.display_name && metric.display_name !== metric.name && (
                      <span className={cn("text-muted-foreground truncate", isMobile ? "text-sm" : "text-xs")}>{metric.display_name}</span>
                    )}
                  </div>
                  <div className="flex items-center gap-1 flex-shrink-0">
                    {metric.unit && (
                      <span className={cn("text-muted-foreground", isMobile ? "text-sm" : "text-xs")}>{metric.unit}</span>
                    )}
                    <Badge variant="outline" className={cn(
                      'py-0',
                      isMobile ? 'h-7 px-2 text-sm' : 'h-5 px-1.5 text-xs',
                      getTypeColor(metric.data_type)
                    )}>
                      {getTypeIcon(metric.data_type)}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className={cn(
              "text-center text-muted-foreground py-8 px-4",
              isMobile ? "text-sm" : "text-sm"
            )}>
              <Database className={cn("mx-auto mb-2 opacity-30", isMobile ? "h-10 w-10" : "h-8 w-8")} />
              <div>{tBuilder('noVariablesHint')}</div>
            </div>
          )}
        </TabsContent>

        {/* Extension Tab */}
        <TabsContent value="extension" className={cn(
          "overflow-y-auto overflow-x-hidden mt-0",
          isMobile ? "h-[290px] p-3" : "h-[340px] p-2"
        )}>
          {/* Extension Selector Button */}
          <Popover open={extensionPopoverOpen} onOpenChange={setExtensionPopoverOpen}>
            <PopoverTrigger asChild>
              <Button variant="outline" size={isMobile ? "default" : "sm"} className={cn(
                "w-full justify-start mb-3",
                isMobile ? "h-11 text-base" : "h-9 text-sm"
              )}>
                <Puzzle className={cn(isMobile ? "h-5 w-5" : "h-4 w-4", "mr-2")} />
                {tBuilder('selectSources')}
                {extensionSources && extensionSources.length > 0 && (
                  <Badge variant="secondary" className={cn(
                    "ml-auto",
                    isMobile ? "h-7 px-2 text-sm" : "h-5 px-1.5 text-xs"
                  )}>
                    {extensionSources.length}
                  </Badge>
                )}
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-72 max-h-80 overflow-auto p-3" align="start">
              {loadingExtensions ? (
                <div className="flex items-center justify-center py-4">
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground mr-2" />
                  <span className="text-sm text-muted-foreground">{t('loading')}</span>
                </div>
              ) : extensions.length === 0 ? (
                <div className="text-center py-4 text-sm text-muted-foreground">
                  {tBuilder('noExtensionSources')}
                </div>
              ) : (
                <div className="space-y-3">
                  {extensions.map((ext) => (
                    <div key={ext.extension_id} className="space-y-1.5">
                      <div className="font-medium text-xs flex items-center gap-2">
                        <Puzzle className="h-3 w-3 text-purple-500" />
                        {ext.extension_name}
                      </div>
                      {ext.commands.map((cmd) => (
                        <div key={cmd.command} className="ml-4 space-y-1">
                          {cmd.fields.map((field) => (
                            <div key={field.field} className="flex items-center gap-2 py-0.5">
                              <Checkbox
                                id={`field-${ext.extension_id}-${cmd.command}-${field.field}`}
                                checked={isSourceSelected(ext.extension_id, field.field)}
                                className="h-3 w-3"
                                onCheckedChange={() => toggleSource(
                                  ext.extension_id,
                                  ext.extension_name,
                                  field.field,
                                  field.display_name,
                                  field.data_type,
                                  field.unit
                                )}
                              />
                              <label
                                htmlFor={`field-${ext.extension_id}-${cmd.command}-${field.field}`}
                                className="text-xs text-muted-foreground cursor-pointer flex-1 truncate"
                                title={field.display_name}
                              >
                                {field.field}
                                {field.unit && <span className="ml-1 text-muted-foreground">({field.unit})</span>}
                              </label>
                            </div>
                          ))}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              )}
            </PopoverContent>
          </Popover>

          {/* Selected Extension Sources */}
          {extensionSources && extensionSources.length > 0 && (
            <div className={cn("space-y-2", isMobile ? "space-y-3" : "")}>
              {Object.entries(groupedExtensions).map(([extId, commands]) => {
                const extName = extensionSources.find(s => s.extension_id === extId)?.extension_name || extId
                return (
                  <div key={extId} className="border rounded bg-purple-50/50 dark:bg-purple-950/20 overflow-hidden">
                    <div className={cn(
                      "border-b font-medium text-purple-700 dark:text-purple-300",
                      isMobile ? "px-4 py-2.5 text-sm" : "px-2.5 py-1.5 text-xs"
                    )}>
                      {extName}
                    </div>
                    <div className={cn("space-y-1", isMobile ? "p-3" : "p-1.5")}>
                      {Object.entries(commands).map(([cmd, fields]) => (
                        <div key={cmd}>
                          <div className={cn("text-muted-foreground px-1 py-0.5", isMobile ? "text-sm" : "text-xs")}>{cmd}</div>
                          {fields.map((field, idx) => (
                            <div
                              key={idx}
                              className={cn(
                                "flex items-center justify-between bg-background border rounded hover:bg-primary/5 hover:border-primary/30 transition-all cursor-pointer group",
                                isMobile ? "px-4 py-3" : "px-2 py-1.5"
                              )}
                              onClick={() => onInsertVariable?.(`input.extensions.${extId}.${field.field}`)}
                            >
                              <div className="flex items-center gap-2 min-w-0 flex-1">
                                <code className={cn(
                                  "font-mono text-purple-600 dark:text-purple-400 truncate",
                                  isMobile ? "text-sm" : "text-xs"
                                )}>
                                  {field.field}
                                </code>
                                {field.display_name !== field.field && (
                                  <span className={cn("text-muted-foreground truncate", isMobile ? "text-sm" : "text-xs")}>{field.display_name}</span>
                                )}
                              </div>
                              <div className="flex items-center gap-1 flex-shrink-0">
                                {field.unit && (
                                  <span className={cn("text-muted-foreground", isMobile ? "text-sm" : "text-xs")}>{field.unit}</span>
                                )}
                                <Badge variant="outline" className={cn(
                                  'py-0',
                                  isMobile ? 'h-7 px-2 text-sm' : 'h-5 px-1.5 text-xs',
                                  getTypeColor(field.data_type)
                                )}>
                                  {getTypeIcon(field.data_type)}
                                </Badge>
                              </div>
                            </div>
                          ))}
                        </div>
                      ))}
                    </div>
                  </div>
                )
              })}
            </div>
          )}

          {/* Extension Commands Section */}
          {extensionCommands.length > 0 && (
            <div className="mt-4 pt-4 border-t">
              <div className="flex items-center gap-2 mb-3 px-1">
                <Zap className="h-4 w-4 text-amber-500" />
                <span className="text-sm font-medium">{tBuilder('extensionActions') || 'Actions'}</span>
              </div>
              <div className="space-y-2">
                {extensionCommands.map((cmd, idx) => (
                  <div
                    key={idx}
                    className="border border-amber-200 dark:border-amber-900/50 rounded-lg overflow-hidden bg-amber-50/30 dark:bg-amber-950/10 hover:bg-amber-100/30 dark:hover:bg-amber-900/20 transition-all cursor-pointer group"
                    onClick={() => handleInvokeCommand(cmd)}
                  >
                    {/* Header */}
                    <div className="flex items-center justify-between px-3 py-2 bg-amber-100/50 dark:bg-amber-900/30 border-b border-amber-200 dark:border-amber-900/50">
                      <div className="flex items-center gap-2 min-w-0 flex-1">
                        <Zap className="h-3.5 w-3.5 text-amber-500 shrink-0" />
                        <div className="min-w-0">
                          <div className="text-sm font-medium text-amber-700 dark:text-amber-300 truncate">
                            {cmd.display_name}
                          </div>
                          <div className="text-xs text-muted-foreground truncate">
                            {cmd.extension_name} · {cmd.command_id}
                          </div>
                        </div>
                      </div>
                      <Badge variant="outline" className="text-xs h-6 px-2 text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-700">
                        {tBuilder('call') || 'Call'}
                      </Badge>
                    </div>

                    {/* Description */}
                    {cmd.description && (
                      <div className="px-3 py-1.5 text-xs text-muted-foreground truncate" title={cmd.description}>
                        {cmd.description}
                      </div>
                    )}

                    {/* Parameters */}
                    {cmd.parameters.length > 0 && (
                      <div className="px-3 pb-2">
                        <div className="text-xs text-muted-foreground mb-1.5">Parameters:</div>
                        <div className="flex flex-wrap gap-1.5">
                          {cmd.parameters.map((param, pIdx) => (
                            <div
                              key={pIdx}
                              className="text-xs px-2 py-1 bg-background border rounded flex items-center gap-1.5"
                              title={`${param.display_name} (${param.data_type})${param.required ? '' : ' - optional'}`}
                            >
                              <span className={cn(
                                param.required ? 'text-foreground' : 'text-muted-foreground'
                              )}>
                                {param.name}
                              </span>
                              <span className="text-[10px] px-1.5 rounded bg-muted">
                                {param.data_type.slice(0, 3)}
                              </span>
                              {param.required && (
                                <span className="text-red-500">*</span>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Empty state */}
          {!extensionSources || extensionSources.length === 0 ? (
            <div className="text-center text-muted-foreground text-sm py-10 px-4">
              <Puzzle className="h-12 w-12 mx-auto mb-3 opacity-30" />
              <div>{tBuilder('noSourcesSelectedHint')}</div>
            </div>
          ) : null}
        </TabsContent>
      </Tabs>
    </div>
  )
}

// ============================================================================
// Transform Preview Panel Component
// ============================================================================

interface TransformPreviewPanelProps {
  name: string
  description: string
  enabled: boolean
  scopeType: ScopeType
  scopeValue: string
  jsCode: string
  outputPrefix: string
  deviceTypeMetrics?: Array<{ name: string; display_name: string; data_type: string; unit?: string }> | null
  extensionSources?: SelectedExtensionSource[]
  testOutput: string
  t: (key: string) => string
  tBuilder: (key: string) => string
}

function TransformPreviewPanel({
  name,
  description,
  enabled,
  scopeType,
  scopeValue,
  jsCode,
  outputPrefix,
  deviceTypeMetrics,
  extensionSources,
  testOutput,
  t,
  tBuilder
}: TransformPreviewPanelProps) {
  const [showDSL, setShowDSL] = React.useState(false)

  // Count code lines
  const codeLines = React.useMemo(() => {
    return jsCode.split('\n').filter(s => s.trim()).length
  }, [jsCode])

  // Parse output metrics from code (simple heuristic)
  const outputMetrics = React.useMemo(() => {
    try {
      // Try to execute code with sample data to get output keys
      const sampleInput = { temperature: 25, humidity: 60 }
      const fn = new Function('input', jsCode)
      const result = fn(sampleInput)
      if (typeof result === 'object' && result !== null && !Array.isArray(result)) {
        return Object.keys(result)
      }
    } catch {
      // If execution fails, return undefined
    }
    return []
  }, [jsCode])

  // Render compact source card
  const renderSourceCard = () => {
    const deviceMetricCount = deviceTypeMetrics?.length || 0
    const extensionSourceCount = extensionSources?.length || 0
    const hasSources = deviceMetricCount > 0 || extensionSourceCount > 0

    return (
      <div className="bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-950/30 dark:to-indigo-950/30 rounded-xl p-4 border border-blue-200 dark:border-blue-800">
        <div className="flex items-center gap-2 mb-3">
          <Database className="h-4 w-4 text-blue-600 dark:text-blue-400" />
          <span className="text-sm font-medium text-blue-900 dark:text-blue-100">
            {tBuilder('deviceMetrics') || '输入来源'}
          </span>
          <Badge variant="secondary" className="ml-auto text-xs">
            {deviceMetricCount + extensionSourceCount}
          </Badge>
        </div>
        {hasSources ? (
          <div className="space-y-2">
            {deviceMetricCount > 0 && (
              <div className="space-y-1">
                <div className="flex items-center gap-2 text-xs text-blue-700 dark:text-blue-300">
                  <Lightbulb className="h-3 w-3" />
                  <span>{deviceMetricCount} {tBuilder('deviceMetrics') || '设备指标'}</span>
                </div>
                {deviceTypeMetrics?.slice(0, 3).map((metric, idx) => (
                  <div key={idx} className="flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1">
                    <span className="text-xs truncate">{metric.display_name || metric.name}</span>
                    {metric.unit && <span className="text-[10px] text-muted-foreground">({metric.unit})</span>}
                  </div>
                ))}
                {deviceMetricCount > 3 && (
                  <div className="text-xs text-blue-700 dark:text-blue-300 pl-4">
                    +{deviceMetricCount - 3} {tBuilder('more') || '更多'}
                  </div>
                )}
              </div>
            )}
            {extensionSourceCount > 0 && (
              <div className="space-y-1">
                <div className="flex items-center gap-2 text-xs text-blue-700 dark:text-blue-300">
                  <Puzzle className="h-3 w-3" />
                  <span>{extensionSourceCount} {tBuilder('extensionDataSources') || '扩展数据源'}</span>
                </div>
                {extensionSources?.slice(0, 2).map((source, idx) => (
                  <div key={idx} className="flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1">
                    <Puzzle className="h-3 w-3 text-purple-500" />
                    <span className="text-xs truncate">{source.extension_name}</span>
                    <span className="text-[10px] text-muted-foreground">·</span>
                    <span className="text-xs truncate">{source.display_name || source.field}</span>
                  </div>
                ))}
                {extensionSourceCount > 2 && (
                  <div className="text-xs text-blue-700 dark:text-blue-300 pl-4">
                    +{extensionSourceCount - 2} {tBuilder('more') || '更多'}
                  </div>
                )}
              </div>
            )}
          </div>
        ) : (
          <p className="text-xs text-blue-700 dark:text-blue-300">
            {tBuilder('noVariablesHint') || '暂无输入变量'}
          </p>
        )}
      </div>
    )
  }

  // Render transform logic card
  const renderLogicCard = () => {
    return (
      <div className="bg-gradient-to-br from-purple-50 to-violet-50 dark:from-purple-950/30 dark:to-violet-950/30 rounded-xl p-4 border border-purple-200 dark:border-purple-800">
        <div className="flex items-center gap-2 mb-3">
          <Code className="h-4 w-4 text-purple-600 dark:text-purple-400" />
          <span className="text-sm font-medium text-purple-900 dark:text-purple-100">
            {tBuilder('transformCode') || '转换逻辑'}
          </span>
          <Badge variant="secondary" className="ml-auto text-xs">
            {codeLines} {tBuilder('lines') || '行'}
          </Badge>
        </div>
        <div className="text-xs text-purple-700 dark:text-purple-300 space-y-2">
          <div className="flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1">
            <Globe className="h-3 w-3" />
            <span className="font-medium">{tBuilder('scopeLabel') || '作用范围'}:</span>
            <span>
              {scopeType === 'global' ? (tBuilder('scope.global') || '全局') :
               scopeType === 'device_type' ? (tBuilder('scope.deviceType') || scopeValue) :
               scopeValue}
            </span>
          </div>
          <div className="flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1">
            <FileCode className="h-3 w-3" />
            <span className="font-mono">{outputPrefix || 'transform'}</span>
            <span className="text-[10px] text-muted-foreground">.key</span>
          </div>
        </div>
      </div>
    )
  }

  // Render output preview card
  const renderOutputCard = () => {
    return (
      <div className="bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-950/30 dark:to-emerald-950/30 rounded-xl p-4 border border-green-200 dark:border-green-800">
        <div className="flex items-center gap-2 mb-3">
          <Zap className="h-4 w-4 text-green-600 dark:text-green-400" />
          <span className="text-sm font-medium text-green-900 dark:text-green-100">
            {tBuilder('outputPrefix') || '输出指标'}
          </span>
          <Badge variant="secondary" className="ml-auto text-xs">
            {outputMetrics.length || 0}
          </Badge>
        </div>
        {outputMetrics.length > 0 ? (
          <div className="space-y-1.5">
            {outputMetrics.slice(0, 4).map((metric, idx) => (
              <div key={idx} className="flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1">
                <Zap className="h-3 w-3 text-green-500" />
                <span className="text-xs font-mono">{outputPrefix}.{metric}</span>
              </div>
            ))}
            {outputMetrics.length > 4 && (
              <div className="text-xs text-center text-green-700 dark:text-green-300">
                +{outputMetrics.length - 4} {tBuilder('more') || '更多'}
              </div>
            )}
          </div>
        ) : (
          <p className="text-xs text-green-700 dark:text-green-300">
            {tBuilder('outputPrefixHint') || '运行测试后显示输出指标'}
          </p>
        )}
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col bg-muted/30 rounded-xl p-4">
      {/* Header with toggle */}
      <div className="flex items-center justify-between mb-4 pb-3 border-b">
        <div className="flex items-center gap-2">
          <Eye className="h-5 w-5 text-primary" />
          <h3 className="font-semibold">{tBuilder('preview') || '实时预览'}</h3>
        </div>
        <button
          onClick={() => setShowDSL(!showDSL)}
          className={cn(
            "flex items-center gap-1.5 px-2 py-1 rounded-md text-xs font-medium transition-colors",
            showDSL
              ? "bg-primary text-primary-foreground"
              : "bg-muted hover:bg-muted/70"
          )}
        >
          <Code className="h-3 w-3" />
          {showDSL ? 'DSL' : (tBuilder('overview') || '预览')}
        </button>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto">
        {showDSL ? (
          <div className="p-3 bg-background rounded-lg border max-h-full overflow-y-auto">
            <pre className="text-[10px] font-mono bg-muted/50 p-3 rounded overflow-x-auto whitespace-pre-wrap break-all">
              {jsCode || '// No code'}
            </pre>
          </div>
        ) : (
          <div className="space-y-3">
            {/* Input Sources Card */}
            {renderSourceCard()}

            {/* Arrow */}
            <div className="flex justify-center py-1">
              <ChevronDown className="h-5 w-5 text-muted-foreground" />
            </div>

            {/* Transform Logic Card */}
            {renderLogicCard()}

            {/* Arrow */}
            <div className="flex justify-center py-1">
              <ChevronDown className="h-5 w-5 text-muted-foreground" />
            </div>

            {/* Output Preview Card */}
            {renderOutputCard()}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function TransformBuilder({
  open,
  onOpenChange,
  transform,
  devices,
  onSave,
}: TransformBuilderProps) {
  const { t } = useTranslation(['automation', 'common'])
  const tBuilder = (key: string) => t(`automation:transformBuilder.${key}`)
  const isEditMode = !!transform
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  // Step state
  const [currentStep, setCurrentStep] = useState<Step>('basic')
  const [completedSteps, setCompletedSteps] = useState<Set<Step>>(new Set())

  // Form data
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [scopeType, setScopeType] = useState<ScopeType>('global')
  const [scopeValue, setScopeValue] = useState('')
  const [outputPrefix, setOutputPrefix] = useState('transform')
  const [jsCode, setJsCode] = useState('')

  // Extension data sources (NEW: selected extension sources for this transform)
  const [selectedExtensionSources, setSelectedExtensionSources] = useState<SelectedExtensionSource[]>([])

  // Test state
  const [testInput, setTestInput] = useState('')
  const [testOutput, setTestOutput] = useState('')
  const [testError, setTestError] = useState('')
  const [testRunning, setTestRunning] = useState(false)

  // Device type metrics state
  const [deviceTypeMetrics, setDeviceTypeMetrics] = useState<Array<{ name: string; display_name: string; data_type: string; unit?: string }> | null>(null)

  // Validation state
  const [formErrors, setFormErrors] = useState<FormErrors>({})

  // Get all device types
  const deviceTypes = useMemo(() => {
    return Array.from(new Set(devices.map((d) => d.device_type).filter((dt): dt is string => Boolean(dt))))
  }, [devices])

  // Build scope options
  const scopeOptions: Array<{ value: string; label: string }> = useMemo(() => {
    if (scopeType === 'device_type') {
      return deviceTypes.map(dt => ({ value: dt, label: dt }))
    }
    if (scopeType === 'device') {
      return devices.map(d => ({ value: d.id, label: d.name }))
    }
    return []
  }, [scopeType, deviceTypes, devices])

  // Auto-select first scope option when scopeType changes
  useEffect(() => {
    if (scopeType !== 'global' && scopeOptions.length > 0) {
      setScopeValue(scopeOptions[0].value)
    } else if (scopeType === 'global') {
      setScopeValue('')
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scopeType])

  // Fetch device type metrics for the selected scope
  useEffect(() => {
    const fetchMetrics = async () => {
      if (scopeType === 'device_type' && scopeValue) {
        try {
          const deviceTypeData = await api.getDeviceType(scopeValue)
          setDeviceTypeMetrics(deviceTypeData.metrics || null)
        } catch {
          setDeviceTypeMetrics(null)
        }
      } else if (scopeType === 'device' && scopeValue) {
        try {
          const device = await api.getDevice(scopeValue)
          if (device.device_type) {
            try {
              const deviceTypeData = await api.getDeviceType(device.device_type)
              setDeviceTypeMetrics(deviceTypeData.metrics || null)
            } catch {
              setDeviceTypeMetrics(null)
            }
          } else {
            setDeviceTypeMetrics(null)
          }
        } catch {
          setDeviceTypeMetrics(null)
        }
      } else {
        setDeviceTypeMetrics(null)
      }
    }

    const timeoutId = setTimeout(fetchMetrics, 300)
    return () => clearTimeout(timeoutId)
  }, [scopeType, scopeValue])

  // Reset form helper
  const resetForm = useCallback(() => {
    setName('')
    setDescription('')
    setEnabled(true)
    setScopeType('global')
    setScopeValue('')
    setOutputPrefix('transform')
    setJsCode('')
    setSelectedExtensionSources([])
    setTestInput('')
    setTestOutput('')
    setTestError('')
    setDeviceTypeMetrics(null)
    setFormErrors({})
  }, [])

  // Reset when dialog opens or transform changes
  useEffect(() => {
    if (open) {
      setCurrentStep('basic')
      setCompletedSteps(new Set())

      if (transform) {
        setName(transform.name)
        setDescription(transform.description || '')
        setEnabled(transform.enabled)
        setOutputPrefix(transform.output_prefix ?? 'transform')
        setJsCode(transform.js_code || '')

        // Handle scope format
        if (transform.scope === 'global') {
          setScopeType('global')
          setScopeValue('')
        } else if (typeof transform.scope === 'object') {
          if ('device_type' in transform.scope) {
            setScopeType('device_type')
            setScopeValue(transform.scope.device_type || '')
          } else if ('device' in transform.scope) {
            setScopeType('device')
            setScopeValue(transform.scope.device || '')
          }
        }

        // Reset extension sources (will be reloaded if we add storage for them)
        setSelectedExtensionSources([])
      } else {
        resetForm()
      }
    }
  }, [open, transform, resetForm])

  // Apply template
  const handleApplyTemplate = useCallback((templateCode: string) => {
    setJsCode(templateCode)
  }, [])

  // Insert variable into code
  const handleInsertVariable = useCallback((variable: string) => {
    // Try CodeMirror first
    const cmEditor = document.querySelector('.cm-content')?.closest('.cm-editor')
    if (cmEditor) {
      const text = jsCode
      // For CodeMirror, just append to the end for now
      // A more sophisticated implementation would use the CM API
      const insertion = text.length > 0 && !text.endsWith(' ') && !text.endsWith('\n') ? ` ${variable}` : variable
      setJsCode(text + insertion)
    } else {
      // Fallback: append to code
      setJsCode(jsCode + (jsCode.length > 0 && !jsCode.endsWith(' ') && !jsCode.endsWith('\n') ? ' ' : '') + variable)
    }
  }, [jsCode])

  // Test code using backend API (supports extension invocation)
  const handleTestCode = useCallback(async () => {
    if (!jsCode.trim()) return

    setTestRunning(true)
    setTestOutput('')
    setTestError('')

    try {
      const inputData: Record<string, unknown> = testInput.trim()
        ? JSON.parse(testInput)
        : { temperature: 25, humidity: 60 }

      // Add mock extension data if sources are selected
      if (selectedExtensionSources.length > 0) {
        const extensions: Record<string, Record<string, Record<string, unknown>>> = {}
        inputData.extensions = extensions
        for (const source of selectedExtensionSources) {
          if (!extensions[source.extension_id]) {
            extensions[source.extension_id] = {}
          }
          if (!extensions[source.extension_id][source.command]) {
            extensions[source.extension_id][source.command] = {}
          }
          // Add mock value based on data type
          switch (source.data_type) {
            case 'integer':
              extensions[source.extension_id][source.command][source.field] = 42
              break
            case 'number':
              extensions[source.extension_id][source.command][source.field] = 42.5
              break
            case 'boolean':
              extensions[source.extension_id][source.command][source.field] = true
              break
            case 'string':
              extensions[source.extension_id][source.command][source.field] = 'sample'
              break
            default:
              extensions[source.extension_id][source.command][source.field] = null
          }
        }
      }

      // Call backend API to test the code
      const result = await api.testTransformCode({
        code: jsCode,
        input_data: inputData,
        output_prefix: outputPrefix,
      })

      if (result.success) {
        // Display output with prefix for clarity
        setTestOutput(JSON.stringify(result.output_with_prefix || result.output, null, 2))
      } else {
        setTestError(result.error || 'Unknown error')
      }
    } catch (err) {
      setTestError(err instanceof Error ? err.message : String(err))
    } finally {
      setTestRunning(false)
    }
  }, [jsCode, testInput, outputPrefix, selectedExtensionSources])

  // Validate current step
  const validateStep = (step: Step): boolean => {
    const errors: FormErrors = {}

    if (step === 'basic') {
      if (!name.trim()) {
        errors.name = tBuilder('validationErrors.name')
      }
      if (scopeType !== 'global' && !scopeValue) {
        errors.scopeValue = tBuilder('validationErrors.scopeValue')
      }
    }

    if (step === 'code') {
      if (!jsCode.trim()) {
        errors.code = tBuilder('validationErrors.code')
      }
      if (outputPrefix && !/^[a-z0-9_]+$/.test(outputPrefix)) {
        errors.outputPrefix = tBuilder('validationErrors.outputPrefix')
      }
    }

    setFormErrors(errors)
    return Object.keys(errors).length === 0
  }

  // Navigate to next step
  const handleNext = () => {
    if (!validateStep(currentStep)) return

    const newCompleted = new Set(completedSteps)
    newCompleted.add(currentStep)
    setCompletedSteps(newCompleted)

    const steps: Step[] = ['basic', 'code', 'test']
    const currentIndex = steps.indexOf(currentStep)
    if (currentIndex < steps.length - 1) {
      setCurrentStep(steps[currentIndex + 1])
    }
  }

  // Navigate to previous step
  const handlePrevious = () => {
    const steps: Step[] = ['basic', 'code', 'test']
    const currentIndex = steps.indexOf(currentStep)
    if (currentIndex > 0) {
      setCurrentStep(steps[currentIndex - 1])
    }
  }

  // Save
  const handleSave = useCallback(() => {
    if (!name.trim()) return

    const scope: TransformScope = (() => {
      switch (scopeType) {
        case 'global': return 'global' as const
        case 'device_type': return { device_type: scopeValue }
        case 'device': return { device: scopeValue }
      }
    })()

    // Save extension sources as a JSON string in description for now
    // (In future, we may add a dedicated field)
    let finalDescription = description
    if (selectedExtensionSources.length > 0) {
      const sourcesInfo = selectedExtensionSources.map(s => ({
        ext: s.extension_id,
        cmd: s.command,
        field: s.field,
      }))
      finalDescription = finalDescription
        ? `${finalDescription}\n\nExtension sources: ${JSON.stringify(sourcesInfo)}`
        : `Extension sources: ${JSON.stringify(sourcesInfo)}`
    }

    onSave({
      name,
      description: finalDescription,
      enabled,
      scope,
      js_code: jsCode,
      output_prefix: outputPrefix,
      complexity: jsCode.split('\n').length > 10 ? 3 : 2,
    })
  }, [name, description, enabled, scopeType, scopeValue, jsCode, outputPrefix, selectedExtensionSources, onSave])

  // Step config
  const steps: { key: Step; label: string; shortLabel: string; icon: React.ReactNode }[] = [
    { key: 'basic', label: tBuilder('steps.basic'), shortLabel: tBuilder('stepShort.basic') || '基本', icon: <Settings className="h-4 w-4" /> },
    { key: 'code', label: tBuilder('steps.code'), shortLabel: tBuilder('stepShort.code') || '代码', icon: <Code className="h-4 w-4" /> },
    { key: 'test', label: tBuilder('steps.test'), shortLabel: tBuilder('stepShort.test') || '测试', icon: <FlaskConical className="h-4 w-4" /> },
  ]

  const stepIndex = steps.findIndex(s => s.key === currentStep)
  const isFirstStep = currentStep === 'basic'

  // Lock body scroll when dialog is open (mobile only to prevent layout shift)
  useBodyScrollLock(open, { mobileOnly: true })

  // Get dialog root for portal rendering
  const dialogRoot = typeof document !== 'undefined'
    ? document.getElementById('dialog-root') || document.body
    : null

  if (!dialogRoot) return null

  return createPortal(
    <div
      className={cn(
        "fixed inset-0 z-[100] bg-background flex flex-col",
        !open && "hidden"
      )}
    >
        {/* Header - Simplified */}
        <header
          className="border-b shrink-0 bg-background"
          style={isMobile ? { paddingTop: `${insets.top}px` } : undefined}
        >
          <div className={cn(
            "flex items-center gap-3",
            isMobile ? "px-4 py-4" : "px-4 py-3"
          )}>
            <Button
              variant="ghost"
              size="icon"
              className={cn("shrink-0", isMobile ? "h-10 w-10" : "h-8 w-8")}
              onClick={() => onOpenChange(false)}
            >
              <X className={cn(isMobile ? "h-5 w-5" : "h-4 w-4")} />
            </Button>
            <div className="flex items-center gap-2 min-w-0 flex-1">
              <div className={cn(
                "rounded-lg bg-blue-500/10 flex items-center justify-center shrink-0",
                isMobile ? "w-8 h-8" : "w-7 h-7"
              )}>
                <Code className={cn(isMobile ? "h-4 w-4" : "h-3.5 w-3.5", "text-blue-500")} />
              </div>
              <h1 className={cn(
                "font-medium truncate",
                isMobile ? "text-base" : "text-sm"
              )}>
                {isEditMode ? tBuilder('editTitle') : tBuilder('title')}
              </h1>
            </div>
          </div>
        </header>

        {/* Content Area - Hide left sidebar on mobile */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left Sidebar - Vertical Steps (Compact) - Hide on mobile */}
          <aside className={cn(
            "border-r shrink-0 bg-muted/20",
            isMobile ? "hidden" : "w-[120px]"
          )}>
            <nav className="px-3 py-6 space-y-1">
              {steps.map((step, index) => {
                const isCompleted = completedSteps.has(step.key)
                const isCurrent = step.key === currentStep
                const isPast = index < stepIndex

                return (
                  <div key={step.key} className="relative">
                    {/* Step Item */}
                    <button
                      onClick={() => {
                        // Allow clicking on completed or current steps
                        if (isCompleted || isPast) {
                          setCurrentStep(step.key)
                        }
                      }}
                      className={cn(
                        "w-full text-left px-2 py-2 rounded-md transition-all flex flex-col items-center gap-1.5",
                        isCurrent && "bg-background shadow-sm",
                        !isCurrent && isPast && "hover:bg-background/50 cursor-pointer",
                        !isCurrent && !isPast && "opacity-50"
                      )}
                      title={step.label}
                    >
                      <div className="flex items-center justify-center">
                        {isCompleted ? (
                          <div className="w-5 h-5 rounded-full bg-green-500 flex items-center justify-center">
                            <Check className="h-3 w-3 text-white" />
                          </div>
                        ) : isCurrent ? (
                          <div className="w-5 h-5 rounded-full bg-primary flex items-center justify-center ring-4 ring-primary/20">
                            <span className="text-[10px] font-medium text-primary-foreground">{index + 1}</span>
                          </div>
                        ) : (
                          <div className="w-5 h-5 rounded-full bg-muted-foreground/20 flex items-center justify-center">
                            <span className="text-[10px] font-medium text-muted-foreground">{index + 1}</span>
                          </div>
                        )}
                      </div>
                      <div className="text-[10px] font-medium text-center leading-tight">
                        {step.shortLabel || step.label}
                      </div>
                    </button>

                    {/* Connector line to next step */}
                    {index < steps.length - 1 && (
                      <div className="absolute left-[23px] top-8 h-4 w-px">
                        <div className={cn(
                          "h-full w-px",
                          isPast ? "bg-primary" : "bg-border"
                        )} />
                      </div>
                    )}
                  </div>
                )
              })}
            </nav>
          </aside>

          {/* Main Content */}
          <main className="flex-1 overflow-y-auto">
            <div className={cn(
              "max-w-5xl mx-auto",
              isMobile ? "px-4 py-4" : "px-4 py-6"
            )}>
              {/* Step 1: Basic Info */}
          {currentStep === 'basic' && (
            <BasicInfoStep
              name={name}
              onNameChange={setName}
              description={description}
              onDescriptionChange={setDescription}
              enabled={enabled}
              onEnabledChange={setEnabled}
              scopeType={scopeType}
              onScopeTypeChange={setScopeType}
              scopeValue={scopeValue}
              onScopeValueChange={setScopeValue}
              scopeOptions={scopeOptions}
              errors={formErrors}
              t={t}
              tBuilder={tBuilder}
            />
          )}

          {/* Step 2: Code */}
          {currentStep === 'code' && (
            <CodeStep
              jsCode={jsCode}
              onCodeChange={setJsCode}
              templates={CODE_TEMPLATES}
              onApplyTemplate={handleApplyTemplate}
              deviceTypeMetrics={deviceTypeMetrics || undefined}
              extensionSources={selectedExtensionSources}
              onExtensionSourcesChange={setSelectedExtensionSources}
              errors={formErrors}
              outputPrefix={outputPrefix}
              onOutputPrefixChange={setOutputPrefix}
              onInsertVariable={handleInsertVariable}
              t={t}
              tBuilder={tBuilder}
              isMobile={isMobile}
            />
          )}

          {/* Step 3: Test */}
          {currentStep === 'test' && (
            <TestStep
              jsCode={jsCode}
              testInput={testInput}
              onTestInputChange={setTestInput}
              testOutput={testOutput}
              testError={testError}
              testRunning={testRunning}
              onTest={handleTestCode}
              onClearTest={() => { setTestOutput(''); setTestError('') }}
              deviceTypeMetrics={deviceTypeMetrics || undefined}
              extensionSources={selectedExtensionSources}
              scopeType={scopeType}
              t={t}
              tBuilder={tBuilder}
            />
          )}
            </div>
          </main>

          {/* Right Preview Panel - Hide on mobile */}
          <aside className={cn(
            "w-[360px] border-l shrink-0 bg-muted/10 overflow-y-auto",
            isMobile && "hidden"
          )}>
            <TransformPreviewPanel
              name={name}
              description={description}
              enabled={enabled}
              scopeType={scopeType}
              scopeValue={scopeValue}
              jsCode={jsCode}
              outputPrefix={outputPrefix}
              deviceTypeMetrics={deviceTypeMetrics}
              extensionSources={selectedExtensionSources}
              testOutput={testOutput}
              t={t}
              tBuilder={tBuilder}
            />
          </aside>
        </div>

        {/* Step Navigation Footer - Compact */}
        <footer
          className="border-t bg-background shrink-0"
          style={isMobile ? { paddingBottom: `${insets.bottom}px` } : undefined}
        >
          <div className={cn(
            "flex gap-2",
            isMobile ? "px-4 py-4" : "px-5 py-3"
          )}>
            {!isFirstStep && (
              <Button variant="outline" size={isMobile ? "default" : "sm"} onClick={handlePrevious} className={isMobile ? "h-12 min-w-[100px]" : ""}>
                <ChevronLeft className={cn(isMobile ? "h-4 w-4" : "h-3.5 w-3.5", "mr-1")} />
                {tBuilder('previous')}
              </Button>
            )}

            <div className="flex-1" />

            <Button
              size={isMobile ? "default" : "sm"}
              onClick={currentStep === 'test' ? handleSave : handleNext}
              disabled={!name.trim() && currentStep !== 'basic'}
              className={isMobile ? "h-12 min-w-[100px]" : ""}
            >
              {currentStep === 'test' ? tBuilder('save') : tBuilder('next')}
              <ChevronRight className={cn(isMobile ? "h-4 w-4" : "h-3.5 w-3.5", "ml-1")} />
            </Button>
          </div>
        </footer>
    </div>,
    dialogRoot
  )
}

// ============================================================================
// Step 1: Basic Info
// ============================================================================

interface BasicInfoStepProps {
  name: string
  onNameChange: (v: string) => void
  description: string
  onDescriptionChange: (v: string) => void
  enabled: boolean
  onEnabledChange: (v: boolean) => void
  scopeType: ScopeType
  onScopeTypeChange: (v: ScopeType) => void
  scopeValue: string
  onScopeValueChange: (v: string) => void
  scopeOptions: Array<{ value: string; label: string }>
  errors: FormErrors
  t: (key: string) => string
  tBuilder: (key: string) => string
}

function BasicInfoStep({
  name,
  onNameChange,
  description,
  onDescriptionChange,
  enabled,
  onEnabledChange,
  scopeType,
  onScopeTypeChange,
  scopeValue,
  onScopeValueChange,
  scopeOptions,
  errors,
  tBuilder,
}: BasicInfoStepProps) {
  return (
    <div className="space-y-6 py-4">
      <div className="text-center mb-6">
        <h3 className="text-lg font-semibold">{tBuilder('steps.basic')}</h3>
        <p className="text-sm text-muted-foreground">{tBuilder('steps.basicDesc')}</p>
      </div>

      {/* Transform Name */}
      <div className="space-y-2">
        <Label className="text-sm font-medium">
          {tBuilder('name')} <span className="text-destructive">*</span>
        </Label>
        <Input
          value={name}
          onChange={e => onNameChange(e.target.value)}
          placeholder={tBuilder('transformNamePlaceholder')}
          className={cn(errors.name && "border-destructive")}
        />
        {errors.name && (
          <p className="text-xs text-destructive">{errors.name}</p>
        )}
      </div>

      {/* Description */}
      <div className="space-y-2">
        <Label className="text-sm font-medium">{tBuilder('description')}</Label>
        <Input
          value={description}
          onChange={e => onDescriptionChange(e.target.value)}
          placeholder={tBuilder('descriptionPlaceholder')}
        />
      </div>

      {/* Enable Switch */}
      <div className="flex items-center gap-3">
        <input
          type="checkbox"
          id="transform-enabled"
          checked={enabled}
          onChange={e => onEnabledChange(e.target.checked)}
          className="h-4 w-4"
        />
        <Label htmlFor="transform-enabled" className="text-sm font-medium cursor-pointer">
          {tBuilder('enabled')}
        </Label>
      </div>

      {/* Scope Selection */}
      <div className="space-y-4">
        <div className="space-y-2">
          <Label className="text-sm font-medium">{tBuilder('scopeLabel')}</Label>
          <Select value={scopeType} onValueChange={(v: any) => onScopeTypeChange(v)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="global">{tBuilder('scope.global')}</SelectItem>
              <SelectItem value="device_type">{tBuilder('scope.deviceType')}</SelectItem>
              <SelectItem value="device">{tBuilder('scope.device')}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {scopeType !== 'global' && (
          <div className="space-y-2">
            <Label className="text-sm font-medium">
              {scopeType === 'device_type' ? tBuilder('scope.deviceType') : tBuilder('scope.device')}
            </Label>
            <Select value={scopeValue} onValueChange={onScopeValueChange}>
              <SelectTrigger className={cn(errors.scopeValue && "border-destructive")}>
                <SelectValue placeholder={tBuilder('selectScope')} />
              </SelectTrigger>
              <SelectContent>
                {scopeOptions.map(opt => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {errors.scopeValue && (
              <p className="text-xs text-destructive">{errors.scopeValue}</p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Step 2: Code
// ============================================================================

interface CodeStepProps {
  jsCode: string
  onCodeChange: (v: string) => void
  templates: Array<{ key: string; nameKey: string; code: string }>
  onApplyTemplate: (code: string) => void
  deviceTypeMetrics?: Array<{ name: string; display_name: string; data_type: string; unit?: string }>
  extensionSources?: SelectedExtensionSource[]
  onExtensionSourcesChange: (sources: SelectedExtensionSource[]) => void
  errors: FormErrors
  outputPrefix: string
  onOutputPrefixChange: (v: string) => void
  onInsertVariable: (variable: string) => void
  t: (key: string) => string
  tBuilder: (key: string) => string
  isMobile?: boolean
}

function CodeStep({
  jsCode,
  onCodeChange,
  templates,
  onApplyTemplate,
  deviceTypeMetrics,
  extensionSources,
  onExtensionSourcesChange,
  errors,
  outputPrefix,
  onOutputPrefixChange,
  onInsertVariable,
  tBuilder,
  t,
  isMobile = false,
}: CodeStepProps) {
  return (
    <div className="space-y-6 py-4">
      {/* Title */}
      <div className="text-center mb-6">
        <h3 className="text-lg font-semibold">{tBuilder('steps.code')}</h3>
        <p className="text-sm text-muted-foreground">{tBuilder('steps.codeDesc')}</p>
      </div>

      {/* Output Prefix */}
      <div className="space-y-2">
        <Label className="text-sm font-medium">{tBuilder('outputPrefix')}</Label>
        <Input
          value={outputPrefix}
          onChange={e => onOutputPrefixChange(e.target.value)}
          placeholder={tBuilder('outputPrefixPlaceholder')}
          className={cn(errors.outputPrefix && "border-destructive")}
        />
        {errors.outputPrefix && (
          <p className="text-xs text-destructive">{errors.outputPrefix}</p>
        )}
      </div>

      {/* Code Editor Section with Variables Panel */}
      <div className="space-y-3">
        {/* Section Header */}
        <Label className="text-sm font-medium">
          {tBuilder('codeLabel')}
        </Label>

        {/* Template Badges */}
        <div className="flex flex-wrap gap-1.5">
          {templates.map((tpl) => (
            <Button
              key={tpl.key}
              variant="outline"
              size="sm"
              onClick={() => onApplyTemplate(tpl.code)}
              className="h-7 text-xs px-2"
            >
              {tBuilder(tpl.nameKey)}
            </Button>
          ))}
        </div>

        {/* Main Code Editor Area */}
        <div className={cn(
          "min-h-[400px]",
          isMobile ? "flex flex-col gap-3" : "flex gap-3"
        )}>
          {/* Left - Variables Panel with integrated Extension selector */}
          <VariablesPanel
            deviceTypeMetrics={deviceTypeMetrics}
            extensionSources={extensionSources}
            onExtensionSourcesChange={onExtensionSourcesChange}
            scopeType="global"
            tBuilder={tBuilder}
            t={t}
            onInsertVariable={onInsertVariable}
            isMobile={isMobile}
          />

          {/* Right - Code Editor */}
          <div className={cn(
            "flex flex-col min-w-0 min-h-0",
            isMobile ? "w-full" : "flex-1"
          )}>
            <CodeEditor
              value={jsCode}
              onChange={onCodeChange}
              minHeight={isMobile ? "300px" : "400px"}
              maxHeight={isMobile ? "500px" : "600px"}
              className="flex-1"
            />
            {errors.code && (
              <p className="text-xs text-destructive mt-1">{errors.code}</p>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

// ============================================================================
// Step 3: Test
// ============================================================================

interface TestStepProps {
  jsCode: string
  testInput: string
  onTestInputChange: (v: string) => void
  testOutput: string
  testError: string
  testRunning: boolean
  onTest: () => void
  onClearTest: () => void
  deviceTypeMetrics?: Array<{ name: string; display_name: string; data_type: string; unit?: string }>
  extensionSources?: SelectedExtensionSource[]
  scopeType: ScopeType
  t: (key: string) => string
  tBuilder: (key: string) => string
}

function TestStep({
  jsCode,
  testInput,
  onTestInputChange,
  testOutput,
  testError,
  testRunning,
  onTest,
  onClearTest,
  deviceTypeMetrics,
  extensionSources,
  scopeType,
  tBuilder,
}: TestStepProps) {
  const generateMockData = useCallback(() => {
    const mockData: Record<string, unknown> & { extensions?: Record<string, unknown> } = {}

    // Generate device metrics mock data
    if (deviceTypeMetrics && deviceTypeMetrics.length > 0) {
      for (const metric of deviceTypeMetrics) {
        switch (metric.data_type) {
          case 'integer':
            mockData[metric.name] = Math.floor(Math.random() * 100)
            break
          case 'float':
            mockData[metric.name] = parseFloat((Math.random() * 100).toFixed(2))
            break
          case 'string':
            mockData[metric.name] = `sample_${metric.name}`
            break
          case 'boolean':
            mockData[metric.name] = Math.random() > 0.5
            break
          case 'array':
            mockData[metric.name] = [
              Math.floor(Math.random() * 100),
              parseFloat((Math.random() * 100).toFixed(2)),
              `sample_${metric.name}`
            ]
            break
          default:
            mockData[metric.name] = null
        }
      }
    } else {
      // Default mock data
      mockData.temperature = 25
      mockData.humidity = 60
    }

    // Add extension data mock (V2: flat structure without command layer)
    if (extensionSources && extensionSources.length > 0) {
      const extensions: Record<string, Record<string, unknown>> = {}
      mockData.extensions = extensions
      for (const source of extensionSources) {
        if (!extensions[source.extension_id]) {
          extensions[source.extension_id] = {}
        }
        switch (source.data_type) {
          case 'integer':
            extensions[source.extension_id][source.field] = Math.floor(Math.random() * 100)
            break
          case 'number':
            extensions[source.extension_id][source.field] = parseFloat((Math.random() * 100).toFixed(2))
            break
          case 'boolean':
            extensions[source.extension_id][source.field] = Math.random() > 0.5
            break
          case 'string':
            extensions[source.extension_id][source.field] = `sample_${source.field}`
            break
          default:
            extensions[source.extension_id][source.field] = null
        }
      }
    }

    onTestInputChange(JSON.stringify(mockData, null, 2))
  }, [deviceTypeMetrics, extensionSources, onTestInputChange])

  return (
    <div className="space-y-6 py-4">
      <div className="text-center mb-6">
        <h3 className="text-lg font-semibold">{tBuilder('test.title')}</h3>
        <p className="text-sm text-muted-foreground">{tBuilder('test.description')}</p>
      </div>

      {/* Summary */}
      <div className="grid grid-cols-3 gap-4">
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-xl font-bold text-blue-500">
            {jsCode.split('\n').filter(s => s.trim()).length}
          </div>
          <div className="text-xs text-muted-foreground">{tBuilder('test.codeLines')}</div>
        </div>
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-xl font-bold">
            {scopeType === 'global' ? tBuilder('scope.global') : scopeType}
          </div>
          <div className="text-xs text-muted-foreground">{tBuilder('test.scope')}</div>
        </div>
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-xl font-bold text-purple-500">
            {extensionSources?.length || 0}
          </div>
          <div className="text-xs text-muted-foreground">{tBuilder('test.extensionSources')}</div>
        </div>
      </div>

      {/* Code Preview */}
      <div className="rounded-lg border bg-card p-4">
        <h4 className="font-medium flex items-center gap-2 mb-3">
          <Code className="h-4 w-4" />
          {tBuilder('test.transformCode')}
        </h4>
        <pre className="text-xs font-mono bg-muted/30 p-3 rounded overflow-x-auto whitespace-pre-wrap max-h-48">
          {jsCode || tBuilder('noCode')}
        </pre>
      </div>

      {/* Test Panel */}
      <div className="rounded-lg border bg-card p-4">
        <h4 className="font-medium flex items-center gap-2 mb-3">
          <Play className="h-4 w-4" />
          {tBuilder('test.testPanel')}
        </h4>

        <div className="space-y-3">
          <div>
            <Label className="text-xs text-muted-foreground mb-2 block">{tBuilder('inputData')}</Label>
            <Textarea
              value={testInput}
              onChange={e => onTestInputChange(e.target.value)}
              placeholder='{"temperature": 25}'
              className="font-mono text-xs resize-none bg-muted/30 h-24"
            />
          </div>

          <div className="flex items-center gap-2">
            <Button
              size="sm"
              onClick={onTest}
              disabled={!jsCode || testRunning}
              className="h-8"
            >
              {testRunning ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3 mr-1" />}
              {tBuilder('run')}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={generateMockData}
              className="h-8"
            >
              <FlaskConical className="h-3 w-3 mr-1" />
              {tBuilder('generateMock')}
            </Button>
            {(testOutput || testError) && (
              <Button
                size="sm"
                variant="ghost"
                onClick={onClearTest}
                className="h-8"
              >
                {tBuilder('clear')}
              </Button>
            )}
          </div>

          {/* Output */}
          {(testOutput || testError) && (
            <div>
              <Label className="text-xs text-muted-foreground mb-2 block">{tBuilder('outputData')}</Label>
              <div className="rounded-md bg-muted/30 p-2 max-h-40 overflow-auto">
                {testError && (
                  <div className="p-1.5 bg-destructive/10 border border-destructive/20 rounded text-xs text-destructive font-mono">
                    {testError}
                  </div>
                )}
                {testOutput && !testError && (
                  <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap break-all">
                    {testOutput}
                  </pre>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
