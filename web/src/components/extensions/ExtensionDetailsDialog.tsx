import { useState, useEffect, useCallback } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { Settings, FileCode, Info, Loader2, Terminal, Database, Zap, Save, RefreshCw, X } from "lucide-react"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { useStore } from "@/store"
import { formatTimestamp } from "@/lib/utils/format"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import type { Extension, ExtensionStatsDto, ExtensionConfigResponse } from "@/types"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"

interface ExtensionDetailsDialogProps {
  extension: Extension | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ExtensionDetailsDialog({
  extension,
  open,
  onOpenChange,
}: ExtensionDetailsDialogProps) {
  const { t } = useTranslation(['extensions', 'common'])
  const { handleError } = useErrorHandler()
  const { toast } = useToast()
  const getExtensionStats = useStore((state) => state.getExtensionStats)
  const getExtensionHealth = useStore((state) => state.getExtensionHealth)
  const fetchExtensions = useStore((state) => state.fetchExtensions)
  const reloadExtensionStore = useStore((state) => state.reloadExtension)
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [stats, setStats] = useState<ExtensionStatsDto | null>(null)
  const [health, setHealth] = useState<{ healthy: boolean } | null>(null)
  const [loading, setLoading] = useState(false)
  const [reloading, setReloading] = useState(false)

  // Config state
  const [configData, setConfigData] = useState<ExtensionConfigResponse | null>(null)
  const [configLoading, setConfigLoading] = useState(false)
  const [configValues, setConfigValues] = useState<Record<string, unknown>>({})
  const [saving, setSaving] = useState(false)
  const [activeTab, setActiveTab] = useState('info')

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  // Load extension details when dialog opens
  const loadDetails = async () => {
    if (!extension) return

    setLoading(true)
    try {
      const [statsData, healthData] = await Promise.all([
        getExtensionStats(extension.id),
        getExtensionHealth(extension.id),
      ])
      setStats(statsData)
      setHealth(healthData)
    } catch (error) {
      handleError(error, { operation: 'Load extension details', showToast: false })
    } finally {
      setLoading(false)
    }
  }

  // Load config when config tab is opened
  const loadConfig = async () => {
    if (!extension) return

    setConfigLoading(true)
    try {
      const response = await api.get<ExtensionConfigResponse>(`/extensions/${extension.id}/config`)
      setConfigData(response)
      setConfigValues(response.current_config || {})
    } catch (error) {
      handleError(error, { operation: 'Load extension config', showToast: false })
    } finally {
      setConfigLoading(false)
    }
  }

  // Save config
  const saveConfig = async () => {
    if (!extension) return

    setSaving(true)
    try {
      await api.put(`/extensions/${extension.id}/config`, configValues)
      toast({ title: "Configuration saved successfully" })

      // Reload extension to apply new config
      const reloadSuccess = await reloadExtensionStore(extension.id)
      if (reloadSuccess) {
        toast({ title: "Extension reloaded with new configuration" })
      } else {
        toast({ title: "Extension reloaded, but config may not have been applied", variant: "default" })
      }

      // Refresh extensions list and config
      await Promise.all([
        fetchExtensions(),
        loadConfig()
      ])
    } catch (error) {
      handleError(error, { operation: 'Save extension config' })
    } finally {
      setSaving(false)
    }
  }

  // Update config value
  const updateConfigValue = (name: string, value: unknown) => {
    setConfigValues(prev => ({ ...prev, [name]: value }))
  }

  // Handle tab change
  const handleTabChange = (tab: string) => {
    setActiveTab(tab)
    if (tab === "info") loadDetails()
    if (tab === "config") loadConfig()
    if (tab === "stats") loadDetails()
  }

  // Reset state when dialog closes
  const handleClose = useCallback(() => {
    if (!saving && !reloading) {
      setStats(null)
      setHealth(null)
      setConfigData(null)
      setConfigValues({})
      onOpenChange(false)
    }
  }, [saving, reloading, onOpenChange])

  // Reload extension
  const handleReloadExtension = async () => {
    if (!extension) return

    setReloading(true)
    try {
      const success = await reloadExtensionStore(extension.id)
      if (success) {
        toast({ title: "Extension reloaded successfully" })
        // Refresh details
        await loadDetails()
      } else {
        handleError(new Error("Failed to reload extension"), { operation: 'Reload extension' })
      }
    } catch (error) {
      handleError(error, { operation: 'Reload extension' })
    } finally {
      setReloading(false)
    }
  }

  // Load details when dialog opens
  useEffect(() => {
    if (open && extension) {
      loadDetails()
    }
  }, [open, extension?.id])

  // Render config input based on type
  const renderConfigInput = (paramName: string, param: any) => {
    const value = configValues[paramName] ?? param.default

    switch (param.type) {
      case 'boolean':
        return (
          <div key={paramName} className="flex items-center justify-between py-2 gap-2">
            <div className="flex-1 min-w-0">
              <Label className="text-sm font-medium break-words">{param.title || paramName}</Label>
              {param.description && (
                <p className="text-xs text-muted-foreground mt-1 break-words">{param.description}</p>
              )}
            </div>
            <Switch
              checked={value as boolean}
              onCheckedChange={(v) => updateConfigValue(paramName, v)}
            />
          </div>
        )

      case 'integer':
      case 'number':
        return (
          <div key={paramName} className="space-y-2 py-2">
            <Label htmlFor={paramName} className="text-sm font-medium">
              {param.title || paramName}
            </Label>
            <Input
              id={paramName}
              type="number"
              value={value as number ?? ''}
              onChange={(e) => updateConfigValue(paramName, Number(e.target.value))}
              min={param.minimum}
              max={param.maximum}
            />
            {param.description && (
              <p className="text-xs text-muted-foreground">{param.description}</p>
            )}
          </div>
        )

      case 'string':
      default:
        if (param.enum && param.enum.length > 0) {
          return (
            <div key={paramName} className="space-y-2 py-2">
              <Label htmlFor={paramName} className="text-sm font-medium">
                {param.title || paramName}
              </Label>
              <Select
                value={value as string ?? ''}
                onValueChange={(v) => updateConfigValue(paramName, v)}
              >
                <SelectTrigger id={paramName}>
                  <SelectValue placeholder="Select..." />
                </SelectTrigger>
                <SelectContent>
                  {param.enum.map((opt: string) => (
                    <SelectItem key={opt} value={opt}>
                      {opt}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {param.description && (
                <p className="text-xs text-muted-foreground">{param.description}</p>
              )}
            </div>
          )
        }

        return (
          <div key={paramName} className="space-y-2 py-2">
            <Label htmlFor={paramName} className="text-sm font-medium">
              {param.title || paramName}
            </Label>
            <Input
              id={paramName}
              type="text"
              value={value as string ?? ''}
              onChange={(e) => updateConfigValue(paramName, e.target.value)}
            />
            {param.description && (
              <p className="text-xs text-muted-foreground">{param.description}</p>
            )}
          </div>
        )
    }
  }

  // Check if config has any parameters
  const hasConfigParams = configData?.config_schema &&
    Object.keys(configData.config_schema.properties || {}).length > 0

  const isBusy = saving || reloading || loading

  const TabButtons = () => (
    <div className={cn(
      "flex gap-1 p-1 bg-[var(--muted-50)] rounded-xl",
      isMobile ? "overflow-x-auto" : ""
    )}>
      {[
        { value: 'info', icon: Info, label: t('extensions:tabs.info', { defaultValue: 'Info' }) },
        { value: 'capabilities', icon: Zap, label: t('extensions:tabs.caps', { defaultValue: 'Caps' }) },
        { value: 'config', icon: Settings, label: t('extensions:tabs.cfg', { defaultValue: 'Cfg' }) },
        { value: 'stats', icon: Terminal, label: t('extensions:tabs.stats', { defaultValue: 'Stats' }) },
        { value: 'file', icon: FileCode, label: t('extensions:tabs.file', { defaultValue: 'File' }) },
      ].map(tab => (
        <button
          key={tab.value}
          onClick={() => handleTabChange(tab.value)}
          className={cn(
            'flex items-center gap-1.5 py-2 px-3 text-sm font-medium rounded-lg transition-all whitespace-nowrap',
            activeTab === tab.value
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          )}
        >
          <tab.icon className="h-4 w-4" />
          <span className={cn(isMobile && "hidden")}>{tab.label}</span>
        </button>
      ))}
    </div>
  )

  const TabContent = () => {
    switch (activeTab) {
      case 'info':
        return (
          <FormSectionGroup>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.id', { defaultValue: 'ID' })}</Label>
                <p className="text-sm font-mono break-all">{extension?.id}</p>
              </div>
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.name', { defaultValue: 'Name' })}</Label>
                <p className="text-sm break-words">{extension?.name}</p>
              </div>
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.version', { defaultValue: 'Version' })}</Label>
                <p className="text-sm">{extension?.version}</p>
              </div>
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.state', { defaultValue: 'State' })}</Label>
                <Badge variant={extension?.state === "Running" ? "default" : "secondary"}>
                  {extension?.state}
                </Badge>
              </div>
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.commands', { defaultValue: 'Commands' })}</Label>
                <p className="text-sm">{extension?.commands?.length ?? 0}</p>
              </div>
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.metrics', { defaultValue: 'Metrics' })}</Label>
                <p className="text-sm">{extension?.metrics?.length ?? 0}</p>
              </div>
            </div>

            {extension?.description && (
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.description', { defaultValue: 'Description' })}</Label>
                <p className="text-sm break-words">{extension.description}</p>
              </div>
            )}

            {extension?.author && (
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.author', { defaultValue: 'Author' })}</Label>
                <p className="text-sm break-words">{extension.author}</p>
              </div>
            )}

            {health && (
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:info.health', { defaultValue: 'Health Status' })}</Label>
                <Badge variant={health.healthy ? "default" : "destructive"}>
                  {health.healthy ? t('extensions:info.healthy', { defaultValue: 'Healthy' }) : t('extensions:info.unhealthy', { defaultValue: 'Unhealthy' })}
                </Badge>
              </div>
            )}
          </FormSectionGroup>
        )

      case 'capabilities':
        return (
          <FormSectionGroup>
            {/* Commands Section */}
            {extension?.commands && extension.commands.length > 0 && (
              <FormSection
                title={`${t('extensions:capabilities.commands', { defaultValue: 'Commands' })} (${extension.commands.length})`}
                collapsible
                defaultExpanded
              >
                <div className="space-y-2">
                  {extension.commands.map((command) => (
                    <div
                      key={command.id}
                      className="p-3 rounded-lg border bg-[var(--muted-30)] space-y-2"
                    >
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge variant="outline" className="text-xs break-all">
                          {command.id}
                        </Badge>
                        <span className="font-medium text-sm break-words">{command.display_name}</span>
                      </div>
                      {command.description && (
                        <p className="text-xs text-muted-foreground break-words">{command.description}</p>
                      )}
                    </div>
                  ))}
                </div>
              </FormSection>
            )}

            {/* Metrics Section */}
            {extension?.metrics && extension.metrics.length > 0 && (
              <FormSection
                title={`${t('extensions:capabilities.metrics', { defaultValue: 'Metrics' })} (${extension.metrics.length})`}
                collapsible
                defaultExpanded
              >
                <div className="space-y-2">
                  {extension.metrics.map((metric) => (
                    <div
                      key={metric.name}
                      className="p-3 rounded-lg border bg-[var(--muted-20)] flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2"
                    >
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-medium text-sm break-words">{metric.display_name}</span>
                        <Badge variant="outline" className="text-xs break-all">
                          {metric.name}
                        </Badge>
                      </div>
                      <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                        <span>{metric.data_type}</span>
                        {metric.unit && <span>({metric.unit})</span>}
                        {metric.min !== undefined && metric.max !== undefined && (
                          <span>[{metric.min} - {metric.max}]</span>
                        )}
                        {metric.required && (
                          <Badge variant="secondary" className="text-xs">{t('common:required', { defaultValue: 'required' })}</Badge>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </FormSection>
            )}

            {/* Empty State */}
            {(!extension?.commands || extension.commands.length === 0) &&
             (!extension?.metrics || extension.metrics.length === 0) && (
              <div className="text-center py-8 text-muted-foreground text-sm">
                {t('extensions:capabilities.noCapabilities', { defaultValue: 'No commands or metrics available for this extension' })}
              </div>
            )}
          </FormSectionGroup>
        )

      case 'config':
        return configLoading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : hasConfigParams ? (
          <FormSectionGroup>
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 pb-2 border-b">
              <div>
                <h3 className="text-sm font-medium">{t('extensions:config.title', { defaultValue: 'Extension Configuration' })}</h3>
                <p className="text-xs text-muted-foreground">
                  {t('extensions:config.configure', { defaultValue: 'Configure' })} {extension?.name}
                </p>
              </div>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  onClick={loadConfig}
                  disabled={saving}
                >
                  <RefreshCw className="h-4 w-4 mr-1" />
                  {t('common:reload', { defaultValue: 'Reload' })}
                </Button>
                <Button
                  size="sm"
                  onClick={saveConfig}
                  disabled={saving}
                >
                  <Save className="h-4 w-4 mr-1" />
                  {saving ? t('common:saving', { defaultValue: 'Saving...' }) : t('extensions:config.saveReload', { defaultValue: 'Save & Reload' })}
                </Button>
              </div>
            </div>

            <div className="space-y-1">
              {Object.entries(configData!.config_schema.properties || {}).map(([name, param]) =>
                renderConfigInput(name, param)
              )}
            </div>

            <div className="pt-4 border-t">
              <p className="text-xs text-muted-foreground">
                {t('extensions:config.changesNote', { defaultValue: 'Changes will be saved and the extension will be reloaded to apply the new configuration.' })}
              </p>
            </div>
          </FormSectionGroup>
        ) : (
          <div className="text-center py-8 text-muted-foreground text-sm">
            <Settings className="h-8 w-8 mx-auto mb-2 opacity-50" />
            <p>{t('extensions:config.noConfig', { defaultValue: 'This extension does not have configurable options.' })}</p>
          </div>
        )

      case 'stats':
        return loading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : stats ? (
          <FormSectionGroup>
            <div className="grid grid-cols-2 gap-3">
              <div className="border rounded-lg p-3">
                <p className="text-xs text-muted-foreground">{t('extensions:stats.startCount', { defaultValue: 'Start Count' })}</p>
                <p className="text-2xl font-semibold">{stats.start_count ?? 0}</p>
              </div>
              <div className="border rounded-lg p-3">
                <p className="text-xs text-muted-foreground">{t('extensions:stats.stopCount', { defaultValue: 'Stop Count' })}</p>
                <p className="text-2xl font-semibold">{stats.stop_count ?? 0}</p>
              </div>
              <div className="border rounded-lg p-3">
                <p className="text-xs text-muted-foreground">{t('extensions:stats.errorCount', { defaultValue: 'Error Count' })}</p>
                <p className="text-2xl font-semibold">{stats.error_count ?? 0}</p>
              </div>
              {stats.last_error && (
                <div className="border rounded-lg p-3 col-span-2">
                  <p className="text-xs text-muted-foreground">{t('extensions:stats.lastError', { defaultValue: 'Last Error' })}</p>
                  <p className="text-sm text-destructive break-words">{stats.last_error}</p>
                </div>
              )}
            </div>
          </FormSectionGroup>
        ) : (
          <div className="text-center py-8 text-muted-foreground text-sm">
            {t('extensions:stats.noStats', { defaultValue: 'Select Info tab to load extension details' })}
          </div>
        )

      case 'file':
        return (
          <FormSectionGroup>
            <div>
              <Label className="text-muted-foreground text-xs">{t('extensions:file.filePath', { defaultValue: 'File Path' })}</Label>
              <p className="text-sm font-mono break-all">{extension?.file_path || "N/A"}</p>
            </div>
            <div>
              <Label className="text-muted-foreground text-xs">{t('extensions:file.version', { defaultValue: 'Version' })}</Label>
              <p className="text-sm">{extension?.version}</p>
            </div>
            {extension?.loaded_at && (
              <div>
                <Label className="text-muted-foreground text-xs">{t('extensions:file.loadedAt', { defaultValue: 'Loaded At' })}</Label>
                <p className="text-sm break-all">{formatTimestamp(extension.loaded_at)}</p>
              </div>
            )}
          </FormSectionGroup>
        )

      default:
        return null
    }
  }

  if (!extension) {
    return null
  }

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
                <Info className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('extensions:details.title', { defaultValue: 'Extension Details' })}</h1>
                  <p className="text-xs text-muted-foreground truncate">{extension.name}</p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={isBusy} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Tabs */}
            <div className="px-4 py-3 border-b shrink-0 bg-background">
              <TabButtons />
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <TabContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={isBusy} className="min-w-[80px]">
                {t('common:close')}
              </Button>
              <Button
                variant="outline"
                onClick={handleReloadExtension}
                disabled={isBusy}
                className="min-w-[80px]"
              >
                {reloading ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <RefreshCw className="h-4 w-4 mr-2" />
                )}
                {t('common:reload', { defaultValue: 'Reload' })}
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
                <Info className="h-5 w-5 text-primary" />
                <h2 className="text-lg font-semibold leading-none truncate">
                  {t('extensions:details.title', { defaultValue: 'Extension Details' })}
                </h2>
              </div>
              <p className="text-sm text-muted-foreground truncate">
                {extension.name}
              </p>
            </div>
            <button
              onClick={handleClose}
              disabled={isBusy}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Tabs */}
          <div className="px-6 py-3 border-b shrink-0 bg-[var(--muted-20)]">
            <TabButtons />
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <TabContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t shrink-0 bg-[var(--muted-30)]">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={isBusy}>
              {t('common:close')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleReloadExtension}
              disabled={isBusy}
            >
              {reloading ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <RefreshCw className="h-4 w-4 mr-2" />
              )}
              {t('common:reload', { defaultValue: 'Reload' })}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
