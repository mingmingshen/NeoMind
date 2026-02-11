/**
 * Command Button Component
 *
 * A button that opens a command form dialog when clicked.
 * Supports both device commands and extension commands.
 * Shows command parameters for user input before sending.
 *
 * This is NOT a toggle switch - it's a command trigger button.
 */

import { Power, Lightbulb, Fan, Lock, Play, ChevronRight, Info } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useState, useCallback, useEffect } from 'react'
import { cn } from '@/lib/utils'
import type { DataSource } from '@/types/dashboard'
import type { ParameterDefinition } from '@/types'
import { api } from '@/lib/api'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useToast } from '@/hooks/use-toast'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'

export interface CommandButtonProps {
  // Command data source
  dataSource?: DataSource

  // Display
  title?: string
  label?: string
  size?: 'sm' | 'md' | 'lg'

  // Edit mode - disable click when editing dashboard
  editMode?: boolean

  disabled?: boolean
  className?: string
}

// Get icon based on title
function getIconForTitle(title?: string): React.ComponentType<{ className?: string }> {
  if (!title) return Power
  const lower = title.toLowerCase()
  if (lower.includes('light') || lower.includes('lamp')) return Lightbulb
  if (lower.includes('fan')) return Fan
  if (lower.includes('lock')) return Lock
  return Power
}

export function ToggleSwitch({
  dataSource,
  title,
  size = 'md',
  editMode = false,
  disabled = false,
  className,
}: CommandButtonProps) {
  const { t } = useTranslation('dashboardComponents')
  const { toast } = useToast()

  // Dialog and parameter states
  const [dialogOpen, setDialogOpen] = useState(false)
  const [commandParams, setCommandParams] = useState<Record<string, unknown>>({})
  const [parameterDefinitions, setParameterDefinitions] = useState<ParameterDefinition[]>([])
  const [commandDisplayName, setCommandDisplayName] = useState<string>('')
  const [loadingParams, setLoadingParams] = useState(false)
  const [sending, setSending] = useState(false)

  // Check data source type
  const isDeviceCommand = dataSource?.type === 'command'
  const isExtensionCommand = dataSource?.type === 'extension-command'
  const hasCommand = isDeviceCommand || isExtensionCommand

  const deviceId = isDeviceCommand ? dataSource?.deviceId : undefined
  const commandName = isDeviceCommand ? (dataSource?.command || 'setValue') : undefined

  const extensionId = isExtensionCommand ? dataSource?.extensionId : undefined
  const extensionCommand = isExtensionCommand ? dataSource?.extensionCommand : undefined

  // Fetch command parameters when dialog opens
  useEffect(() => {
    if (!dialogOpen) return

    const fetchCommandParams = async () => {
      setLoadingParams(true)
      try {
        if (isDeviceCommand && deviceId) {
          // Get device current state which includes command definitions
          const response = await api.getDeviceCurrent(deviceId)
          const commandDef = response.commands?.find((c: any) => c.name === commandName)
          if (commandDef) {
            setParameterDefinitions(commandDef.parameters || [])
            setCommandDisplayName(commandDef.display_name || commandDef.name)

            // Initialize parameters with defaults
            const defaults: Record<string, unknown> = {}
            commandDef.parameters?.forEach((param: any) => {
              if (param.default_value !== undefined) {
                defaults[param.name] = param.default_value
              } else if (param.data_type === 'integer' || param.data_type === 'float') {
                defaults[param.name] = 0
              } else if (param.data_type === 'boolean') {
                defaults[param.name] = false
              } else if (param.data_type === 'string') {
                defaults[param.name] = ''
              }
            })
            setCommandParams(defaults)
          }
        } else if (isExtensionCommand && extensionId && extensionCommand) {
          // Get extension command parameters
          const extensions = await api.listExtensions()
          const ext = extensions.find(e => e.id === extensionId)
          if (ext) {
            const cmd = ext.commands?.find(c => c.id === extensionCommand)
            if (cmd) {
              // Convert extension command format to ParameterDefinition
              const params = cmd.input_schema?.properties || {}
              const required = (cmd.input_schema?.required as string[]) || []
              const paramDefs: ParameterDefinition[] = Object.entries(params).map(([name, schema]: [string, any]) => ({
                name,
                display_name: schema.title || name,
                help_text: schema.description,
                data_type: schema.type || 'string',
                required: required.includes(name),
                default_value: schema.default,
                min: schema.minimum,
                max: schema.maximum,
                allowed_values: schema.enum,
              }))
              setParameterDefinitions(paramDefs)
              setCommandDisplayName(cmd.display_name || cmd.id)

              // Initialize with defaults
              const defaults: Record<string, unknown> = {}
              Object.entries(params).forEach(([name, schema]: [string, any]) => {
                defaults[name] = schema.default !== undefined ? schema.default : ''
              })
              setCommandParams(defaults)
            }
          }
        }
      } catch (error) {
        console.error('[CommandButton] Failed to fetch command parameters:', error)
      } finally {
        setLoadingParams(false)
      }
    }

    fetchCommandParams()
  }, [dialogOpen, isDeviceCommand, isExtensionCommand, deviceId, commandName, extensionId, extensionCommand])

  const handleClick = () => {
    // Don't execute click in edit mode - allows dragging
    if (editMode) return
    if (disabled || sending || !hasCommand) return

    // Open the command dialog
    setDialogOpen(true)
  }

  const handleConfirmSend = async () => {
    if (!hasCommand) return

    setSending(true)

    try {
      if (isDeviceCommand && deviceId && commandName) {
        // Send device command
        await api.sendCommand(deviceId, commandName, commandParams)
        toast({
          title: t('commandButton.commandSent'),
          description: t('commandButton.commandSentDesc'),
        })
      } else if (isExtensionCommand && extensionId && extensionCommand) {
        // Send extension command
        await api.invokeExtension(extensionId, extensionCommand, commandParams)
        toast({
          title: t('commandButton.commandSent'),
          description: t('commandButton.commandSentDesc'),
        })
      }

      setDialogOpen(false)
      setCommandParams({})
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Command failed'
      toast({
        title: t('commandButton.commandFailed'),
        description: errorMessage,
        variant: 'destructive',
      })
    } finally {
      setSending(false)
    }
  }

  const updateParameter = (name: string, value: unknown) => {
    setCommandParams(prev => ({
      ...prev,
      [name]: value,
    }))
  }

  // Render parameter input based on data type
  const renderParameterInput = (param: ParameterDefinition) => {
    const value = commandParams[param.name]

    if (param.allowed_values && param.allowed_values.length > 0) {
      return (
        <Select
          value={String(value ?? '')}
          onValueChange={(v) => updateParameter(param.name, v)}
        >
          <SelectTrigger>
            <SelectValue placeholder={t('commandButton.selectValue')} />
          </SelectTrigger>
          <SelectContent>
            {param.allowed_values.map((allowed, idx) => (
              <SelectItem key={idx} value={String(allowed)}>
                {String(allowed)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )
    }

    if (param.data_type === 'boolean') {
      return (
        <Select
          value={String(value ?? 'false')}
          onValueChange={(v) => updateParameter(param.name, v === 'true')}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="true">{t('commandButton.yes')}</SelectItem>
            <SelectItem value="false">{t('commandButton.no')}</SelectItem>
          </SelectContent>
        </Select>
      )
    }

    if (param.data_type === 'integer' || param.data_type === 'float') {
      return (
        <div className="flex items-center gap-2">
          <Input
            type="number"
            value={String(value ?? 0)}
            onChange={(e) =>
              updateParameter(
                param.name,
                param.data_type === 'integer'
                  ? parseInt(e.target.value) || 0
                  : parseFloat(e.target.value) || 0
              )
            }
            min={param.min}
            max={param.max}
            step={param.data_type === 'integer' ? 1 : 0.1}
          />
          {param.unit && <span className="text-xs text-muted-foreground">{param.unit}</span>}
        </div>
      )
    }

    return (
      <Input
        value={String(value ?? '')}
        onChange={(e) => updateParameter(param.name, e.target.value)}
        placeholder={param.display_name || param.name}
      />
    )
  }

  const config = dashboardComponentSize[size]
  const Icon = getIconForTitle(title)

  // Determine button label
  const buttonLabel = title || t('commandButton.defaultLabel')
  const buttonSubtext = isDeviceCommand
    ? (deviceId || t('commandButton.deviceCommand'))
    : isExtensionCommand
      ? (extensionCommand || t('commandButton.extensionCommand'))
      : t('commandButton.notConfigured')

  return (
    <>
      <button
        onClick={handleClick}
        disabled={disabled || sending || !hasCommand || editMode}
        className={cn(
          dashboardCardBase,
          'flex-row items-center',
          config.contentGap,
          config.padding,
          'transition-all duration-200',
          'relative overflow-hidden group',
          !disabled && !sending && hasCommand && !editMode && 'hover:scale-[1.02] active:scale-[0.98]',
          !disabled && !sending && hasCommand && !editMode && 'hover:shadow-md hover:bg-accent/50',
          (disabled || sending || !hasCommand || editMode) && 'opacity-50 cursor-not-allowed',
          editMode && 'pointer-events-none',
          className
        )}
      >
        {/* Icon Section */}
        <div className={cn(
          'flex items-center justify-center shrink-0 rounded-full transition-all duration-300',
          config.iconContainer,
          'bg-primary text-primary-foreground shadow-md',
        )}>
          <Play className={cn(config.iconSize, 'fill-current')} />
        </div>

        {/* Text section */}
        <div className="flex flex-col min-w-0 flex-1 text-left">
          <span className={cn('font-medium text-foreground truncate', config.titleText)}>
            {buttonLabel}
          </span>
          <span className={cn('text-muted-foreground', config.labelText)}>
            {buttonSubtext}
          </span>
        </div>

        {/* Arrow indicator */}
        <ChevronRight className={cn(
          'h-4 w-4 text-muted-foreground transition-transform duration-200',
          !disabled && !sending && hasCommand && !editMode && 'group-hover:translate-x-0.5'
        )} />

        {/* Sending indicator */}
        {sending && (
          <span className="absolute top-3 right-3 w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
        )}

        {/* Warning: no command configured */}
        {!hasCommand && (
          <span className="absolute top-3 right-3 w-2 h-2 rounded-full bg-orange-500" title={t('commandButton.noCommandConfig')} />
        )}
      </button>

      {/* Command Dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Icon className="h-5 w-5" />
              {title || commandDisplayName || t('commandButton.sendCommand')}
            </DialogTitle>
            <DialogDescription>
              {isDeviceCommand && (
                <>
                  {t('commandButton.deviceCommandDesc')}
                  {deviceId && <span className="font-medium ml-1">({deviceId})</span>}
                </>
              )}
              {isExtensionCommand && (
                <>
                  {t('commandButton.extensionCommandDesc')}
                  {extensionCommand && <span className="font-medium ml-1">({extensionCommand})</span>}
                </>
              )}
            </DialogDescription>
          </DialogHeader>

          <ScrollArea className="max-h-[60vh] pr-4">
            <div className="space-y-4 py-4">
              {/* Parameter inputs - skip parameters with default values (fixed values) */}
              {!loadingParams && parameterDefinitions.filter(p => p.default_value === undefined).length > 0 && (
                <div className="space-y-4">
                  <div className="text-sm font-medium">{t('commandButton.parameters')}</div>
                  {parameterDefinitions
                    .filter(param => param.default_value === undefined)
                    .map(param => (
                    <div key={param.name} className="space-y-2">
                      <div className="flex items-center justify-between">
                        <Label className="text-sm">
                          {param.display_name || param.name}
                          {param.required && <span className="text-red-500 ml-1">*</span>}
                        </Label>
                        {(param.min !== undefined && param.min !== null || param.max !== undefined && param.max !== null) && (
                          <span className="text-xs text-muted-foreground">
                            {param.min !== undefined && param.min !== null && `${t('range.min')} ${param.min}`}
                            {param.min !== undefined && param.min !== null && param.max !== undefined && param.max !== null && ' | '}
                            {param.max !== undefined && param.max !== null && `${t('range.max')} ${param.max}`}
                          </span>
                        )}
                      </div>
                      {renderParameterInput(param)}
                      {param.help_text && (
                        <p className="text-xs text-muted-foreground">{param.help_text}</p>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {/* All parameters have fixed values */}
              {!loadingParams && parameterDefinitions.length > 0 && parameterDefinitions.filter(p => p.default_value === undefined).length === 0 && (
                <div className="flex items-center gap-2 p-3 rounded-lg bg-green-500/10 border border-green-500/20">
                  <Info className="h-4 w-4 text-green-600 dark:text-green-400" />
                  <span className="text-sm text-green-700 dark:text-green-300">
                    {t('commandButton.allParametersFixed')}
                  </span>
                </div>
              )}

              {/* No parameters */}
              {!loadingParams && parameterDefinitions.length === 0 && (
                <div className="flex items-center gap-2 p-3 rounded-lg bg-muted/50">
                  <Info className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm text-muted-foreground">
                    {t('commandButton.noParameters')}
                  </span>
                </div>
              )}

              {/* Loading params */}
              {loadingParams && (
                <div className="text-sm text-muted-foreground text-center py-4">
                  {t('commandButton.loadingParameters')}
                </div>
              )}
            </div>
          </ScrollArea>

          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={sending}
            >
              {t('commandButton.cancel')}
            </Button>
            <Button
              onClick={handleConfirmSend}
              disabled={sending || loadingParams}
            >
              {sending ? t('commandButton.sending') : t('commandButton.send')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
