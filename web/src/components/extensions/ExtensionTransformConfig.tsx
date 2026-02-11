/**
 * Extension Transform Configuration Component
 * Allows users to configure extension-based data transformations
 * Uses the unified Extension system
 */

import React, { useState, useEffect, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Puzzle,
  Loader2,
  Zap,
  Info,
  Plus,
  X,
  Code,
} from "lucide-react"
import type {
  Extension,
  ExtensionTransformOperation,
  ExtensionCommandDescriptor,
} from "@/types"

interface ExtensionTransformConfigProps {
  /** Current transform operation */
  operation: ExtensionTransformOperation | null
  /** Callback when operation changes */
  onChange: (operation: ExtensionTransformOperation | null) => void
  /** CSS class name */
  className?: string
}

interface ParameterValue {
  name: string
  value: string | number | boolean
  type: 'string' | 'number' | 'boolean' | 'object'
}

export function ExtensionTransformConfig({
  operation,
  onChange,
  className,
}: ExtensionTransformConfigProps) {
  const { t } = useTranslation('extensions')
  const [extensions, setExtensions] = useState<Extension[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // UI state
  const [selectedExtensionId, setSelectedExtensionId] = useState<string>("")
  const [selectedCommand, setSelectedCommand] = useState<string>("")
  const [parameterValues, setParameterValues] = useState<ParameterValue[]>([])
  const [outputMetrics, setOutputMetrics] = useState<string>("")

  // Get available commands for selected extension
  const availableCommands = useCallback((): ExtensionCommandDescriptor[] => {
    if (!selectedExtensionId) return []

    const ext = extensions.find(e => e.id === selectedExtensionId)
    return ext?.commands || []
  }, [extensions, selectedExtensionId])

  // Get selected command descriptor
  const selectedCommandDescriptor = useCallback((): ExtensionCommandDescriptor | null => {
    if (!selectedCommand) return null
    return availableCommands().find(c => c.id === selectedCommand) || null
  }, [availableCommands, selectedCommand])

  // Fetch extensions
  useEffect(() => {
    const fetchData = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = await api.listExtensions()
        setExtensions(data)

        // If we have an existing operation, load its values
        if (operation) {
          setSelectedExtensionId(operation.extension_id)
          setSelectedCommand(operation.command)
          setOutputMetrics(operation.output_metrics.join(', '))

          // Parse parameters if available
          if (operation.parameters) {
            const params: ParameterValue[] = Object.entries(operation.parameters).map(
              ([name, value]) => ({
                name,
                value: value as string | number | boolean,
                type: typeof value as 'string' | 'number' | 'boolean' | 'object',
              })
            )
            setParameterValues(params)
          }
        }
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [])

  // Get extensions that support transforms (all extensions with commands)
  const transformExtensions = extensions.filter(ext => ext.commands && ext.commands.length > 0)

  // Get parameter schema for selected command
  const parameterSchema = useCallback(() => {
    const cmd = selectedCommandDescriptor()
    if (!cmd?.input_schema) return null

    const schema = cmd.input_schema as any
    return schema.properties || schema
  }, [selectedCommandDescriptor])

  // Update operation when values change
  useEffect(() => {
    if (!selectedExtensionId || !selectedCommand) {
      onChange(null)
      return
    }

    const params: Record<string, unknown> = {}
    parameterValues.forEach(p => {
      params[p.name] = p.value
    })

    const newOperation: ExtensionTransformOperation = {
      extension_id: selectedExtensionId,
      command: selectedCommand,
      parameters: params,
      output_metrics: outputMetrics.split(',').map(s => s.trim()).filter(Boolean),
    }

    onChange(newOperation)
  }, [selectedExtensionId, selectedCommand, parameterValues, outputMetrics, onChange])

  // Handle extension selection
  const handleExtensionChange = (extensionId: string) => {
    setSelectedExtensionId(extensionId)
    setSelectedCommand("")
    setParameterValues([])
  }

  // Handle command selection
  const handleCommandChange = (commandId: string) => {
    setSelectedCommand(commandId)

    // Load parameter defaults from schema
    const cmd = availableCommands().find(c => c.id === commandId)
    if (!cmd?.input_schema) return

    const schema = cmd.input_schema as any
    if (schema?.properties) {
      const params: ParameterValue[] = Object.entries(schema.properties).map(
        ([name, prop]: [string, any]) => ({
          name,
          value: prop.default || '',
          type: prop.type || 'string',
        })
      )
      setParameterValues(params)
    }
  }

  // Update parameter value
  const updateParameter = (name: string, value: string | number | boolean) => {
    setParameterValues(prev =>
      prev.map(p => (p.name === name ? { ...p, value } : p))
    )
  }

  // Add a new parameter
  const addParameter = () => {
    const name = `param_${parameterValues.length + 1}`
    setParameterValues(prev => [...prev, { name, value: '', type: 'string' }])
  }

  // Remove a parameter
  const removeParameter = (name: string) => {
    setParameterValues(prev => prev.filter(p => p.name !== name))
  }

  if (loading) {
    return (
      <div className={cn("flex items-center justify-center py-8", className)}>
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground mr-2" />
        <span className="text-sm text-muted-foreground">{t('loadingCapabilities')}</span>
      </div>
    )
  }

  if (error) {
    return (
      <div className={cn("text-center py-8", className)}>
        <Info className="h-8 w-8 mx-auto mb-2 text-destructive/50" />
        <p className="text-sm text-destructive">{t('errorLoadingCapabilities', { error })}</p>
      </div>
    )
  }

  const currentCommands = availableCommands()
  const schema = parameterSchema()

  return (
    <div className={cn("space-y-4", className)}>
      {/* Extension Selection */}
      <div className="space-y-2">
        <Label className="text-sm">{t('selectExtension')}</Label>
        <Select value={selectedExtensionId} onValueChange={handleExtensionChange}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder={t('selectExtensionPlaceholder')} />
          </SelectTrigger>
          <SelectContent>
            {transformExtensions.length === 0 ? (
              <div className="p-2 text-sm text-muted-foreground">
                {t('noTransformExtensions')}
              </div>
            ) : (
              transformExtensions.map(ext => (
                <SelectItem key={ext.id} value={ext.id}>
                  <div className="flex items-center gap-2">
                    <Puzzle className="h-4 w-4" />
                    <span>{ext.name}</span>
                  </div>
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
      </div>

      {/* Command Selection */}
      {selectedExtensionId && (
        <div className="space-y-2">
          <Label className="text-sm">{t('selectCommand')}</Label>
          <Select value={selectedCommand} onValueChange={handleCommandChange}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder={t('selectCommandPlaceholder')} />
            </SelectTrigger>
            <SelectContent>
              {currentCommands.length === 0 ? (
                <div className="p-2 text-sm text-muted-foreground">
                  {t('noCommands')}
                </div>
              ) : (
                currentCommands.map(cmd => (
                  <SelectItem key={cmd.id} value={cmd.id}>
                    <div className="flex items-center gap-2">
                      <Zap className="h-4 w-4" />
                      <div className="flex-1">
                        <div className="font-medium">{cmd.display_name}</div>
                        <div className="text-xs text-muted-foreground">
                          {cmd.description}
                        </div>
                      </div>
                    </div>
                  </SelectItem>
                ))
              )}
            </SelectContent>
          </Select>
        </div>
      )}

      {/* Parameters */}
      {selectedCommand && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <Label className="text-sm">{t('parameters')}</Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={addParameter}
              className="h-6 text-xs"
            >
              <Plus className="h-3 w-3 mr-1" />
              {t('addParameter')}
            </Button>
          </div>

          {schema && (
            <div className="text-xs text-muted-foreground mb-2 p-2 bg-muted/50 rounded">
              <Info className="h-3 w-3 inline mr-1" />
              {t('parameterSchemaHint')}
            </div>
          )}

          <div className="space-y-2">
            {parameterValues.map((param) => (
              <div key={param.name} className="flex items-center gap-2">
                <Input
                  value={param.name}
                  onChange={(e) => {
                    const newName = e.target.value
                    setParameterValues(prev =>
                      prev.map(p => (p.name === param.name ? { ...p, name: newName } : p))
                    )
                  }}
                  placeholder={t('parameterName')}
                  className="flex-1 h-9"
                />
                <Input
                  value={String(param.value)}
                  onChange={(e) => {
                    let value: string | number | boolean = e.target.value
                    if (param.type === 'number') {
                      value = parseFloat(e.target.value) || 0
                    } else if (param.type === 'boolean') {
                      value = e.target.value === 'true'
                    }
                    updateParameter(param.name, value)
                  }}
                  placeholder={t('parameterValue')}
                  className="flex-1 h-9"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => removeParameter(param.name)}
                  className="h-9 w-9 p-0"
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ))}

            {parameterValues.length === 0 && (
              <div className="text-center py-4 text-sm text-muted-foreground border-2 border-dashed rounded-lg">
                <Code className="h-6 w-6 mx-auto mb-2 opacity-50" />
                {t('noParameters')}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Output Metrics */}
      {selectedCommand && (
        <div className="space-y-2">
          <Label className="text-sm">{t('outputMetrics')}</Label>
          <Input
            value={outputMetrics}
            onChange={(e) => setOutputMetrics(e.target.value)}
            placeholder="temp_f, humidity_status, is_alert"
            className="w-full"
          />
          <p className="text-xs text-muted-foreground">
            {t('outputMetricsHint')}
          </p>
        </div>
      )}

      {/* Summary */}
      {operation && (
        <div className="p-3 bg-purple-50 dark:bg-purple-950/30 border border-purple-200 dark:border-purple-800 rounded-lg">
          <div className="flex items-center gap-2 mb-2">
            <Zap className="h-4 w-4 text-purple-600 dark:text-purple-400" />
            <span className="font-medium text-sm text-purple-700 dark:text-purple-300">
              {t('transformSummary')}
            </span>
          </div>
          <div className="text-xs text-purple-600 dark:text-purple-400 space-y-1">
            <div><span className="font-medium">{t('extension')}:</span> {operation.extension_id}</div>
            <div><span className="font-medium">{t('command')}:</span> {operation.command}</div>
            {operation.output_metrics.length > 0 && (
              <div><span className="font-medium">{t('outputs')}:</span> {operation.output_metrics.join(', ')}</div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
