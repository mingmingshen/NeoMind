import { useEffect, useState, useRef } from "react"
import { useTranslation } from "react-i18next"
import {
  Loader2,
  ArrowLeft,
  TestTube,
  Webhook,
  Mail,
  Terminal,
  Database,
  Puzzle,
  Trash2,
  Pencil,
  Filter,
} from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { api } from "@/lib/api"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { UniversalPluginConfigDialog, type PluginInstance, type UnifiedPluginType } from "@/components/plugins/UniversalPluginConfigDialog"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import type { AlertChannel, ChannelTypeInfo, ChannelSchemaResponse, PluginConfigSchema, ExtensionCapabilityDto, ChannelFilter, MessageType, MessageSeverity } from "@/types"

type View = 'list' | 'detail'

interface UnifiedAlertChannelsTabProps {
  onListChannels?: () => Promise<{ channels: AlertChannel[]; stats: any }>
  onCreateChannel?: (data: any) => Promise<void>
  onUpdateChannel?: (name: string, config: Record<string, unknown>) => Promise<void>
  onDeleteChannel?: (name: string) => Promise<void>
  onTestChannel?: (name: string) => Promise<{ success: boolean; message: string }>
  /** Hide the filter configuration button (for Settings page) */
  hideFilterButton?: boolean
}

// Alert Channel type info
const CHANNEL_TYPE_INFO: Record<string, {
  name: string
  icon: React.ReactNode
  iconBg: string
}> = {
  webhook: {
    name: 'Webhook',
    icon: <Webhook className="h-6 w-6" />,
    iconBg: 'bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-400',
  },
  email: {
    name: 'Email',
    icon: <Mail className="h-6 w-6" />,
    iconBg: 'bg-orange-100 text-orange-700 dark:bg-orange-900/20 dark:text-orange-400',
  },
  console: {
    name: 'Console',
    icon: <Terminal className="h-6 w-6" />,
    iconBg: 'bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400',
  },
  memory: {
    name: 'Memory',
    icon: <Database className="h-6 w-6" />,
    iconBg: 'bg-purple-100 text-purple-700 dark:bg-purple-900/20 dark:text-purple-400',
  },
  extension: {
    name: 'Extension',
    icon: <Puzzle className="h-6 w-6" />,
    iconBg: 'bg-purple-100 text-purple-700 dark:bg-purple-900/20 dark:text-purple-400',
  },
}

// Fields to exclude from config schema (managed by the system)
const EXCLUDED_CHANNEL_CONFIG_FIELDS = ['name', 'channel_type']

/**
 * Convert JsonSchema to PluginConfigSchema
 */
function convertToPluginConfigSchema(jsonSchema: any): PluginConfigSchema {
  const properties: Record<string, any> = {}

  for (const [key, prop] of Object.entries(jsonSchema.properties || {})) {
    const typedProp = prop as any
    properties[key] = {
      type: typedProp.type || 'string',
      description: typedProp.description || typedProp.description_zh,
      default: typedProp.default,
      enum: typedProp.enum,
      minimum: typedProp.minimum,
      maximum: typedProp.maximum,
      secret: typedProp.x_secret || false,
    }
  }

  return {
    type: 'object',
    properties,
    required: jsonSchema.required || [],
    ui_hints: jsonSchema.ui_hints || {},
  }
}

/**
 * Convert ChannelSchemaResponse to UnifiedPluginType
 */
function toUnifiedPluginType(type: ChannelSchemaResponse): UnifiedPluginType {
  const info = CHANNEL_TYPE_INFO[type.id] || CHANNEL_TYPE_INFO.webhook

  // Filter out system-managed fields from config schema
  const filteredSchema: PluginConfigSchema = type.config_schema
    ? {
        ...convertToPluginConfigSchema(type.config_schema),
        properties: Object.fromEntries(
          Object.entries(convertToPluginConfigSchema(type.config_schema).properties).filter(
            ([key]) => !EXCLUDED_CHANNEL_CONFIG_FIELDS.includes(key)
          )
        ),
        required: convertToPluginConfigSchema(type.config_schema).required?.filter(
          (field: string) => !EXCLUDED_CHANNEL_CONFIG_FIELDS.includes(field)
        ),
      }
    : {
        type: 'object',
        properties: {},
        required: [],
        ui_hints: {},
      }

  return {
    id: type.id,
    type: 'alert_channel',
    name: type.name,
    description: type.description,
    icon: info.icon,
    color: info.iconBg,
    config_schema: filteredSchema,
    can_add_multiple: true,
    builtin: false,
  }
}

/**
 * Convert AlertChannel to PluginInstance
 */
function toPluginInstance(channel: AlertChannel): PluginInstance {
  return {
    id: channel.name,
    name: channel.name,
    plugin_type: channel.channel_type,
    enabled: channel.enabled,
    running: channel.enabled,
    config: channel.config || {},
    status: {
      connected: channel.enabled,
    },
  }
}

export function UnifiedAlertChannelsTab({
  onListChannels,
  onCreateChannel,
  onUpdateChannel,
  onDeleteChannel,
  onTestChannel,
  hideFilterButton = false,
}: UnifiedAlertChannelsTabProps) {
  const { t } = useTranslation(['plugins', 'alerts', 'common'])
  const { handleError } = useErrorHandler()
  const [view, setView] = useState<View>('list')
  const [loading, setLoading] = useState(true)
  const [schemaLoading, setSchemaLoading] = useState(false)
  const [channelTypes, setChannelTypes] = useState<ChannelTypeInfo[]>([])
  const [channels, setChannels] = useState<AlertChannel[]>([])
  const [selectedType, setSelectedType] = useState<UnifiedPluginType | null>(null)
  const [extensionCapabilities, setExtensionCapabilities] = useState<ExtensionCapabilityDto[]>([])

  // Config dialog state
  const [configDialogOpen, setConfigDialogOpen] = useState(false)
  const [editingInstance, setEditingInstance] = useState<PluginInstance | null>(null)
  const [testResults, setTestResults] = useState<Record<string, { success: boolean; message: string }>>({})

  // Filter configuration dialog state
  const [filterDialogChannel, setFilterDialogChannel] = useState<AlertChannel | null>(null)
  const [filterConfig, setFilterConfig] = useState<ChannelFilter>({
    message_types: [],
    source_types: [],
    categories: [],
    min_severity: null,
    source_ids: [],
  })
  const [savingFilter, setSavingFilter] = useState(false)

  const hasFetched = useRef(false)
  useEffect(() => {
    if (!hasFetched.current) {
      hasFetched.current = true
      loadData()
    }
  }, [])

  const loadData = async () => {
    setLoading(true)
    try {
      // Load channel types
      const typesResponse = await api.listChannelTypes()
      const builtInTypes = typesResponse.types || []
      setChannelTypes(builtInTypes)

      // Load extension capabilities for notifier channels
      try {
        const capabilities = await api.getExtensionCapabilities()
        setExtensionCapabilities(capabilities)

        // Convert extension channels to channel types
        const extensionChannelTypes: ChannelTypeInfo[] = []
        for (const cap of capabilities) {
          if (cap.type === 'notifier' && cap.channels) {
            for (const channel of cap.channels) {
              extensionChannelTypes.push({
                id: `extension:${cap.extension_id}:${channel.name}`,
                name: channel.name,
                name_zh: channel.name,
                description: channel.description || `Extension channel from ${cap.extension_name}`,
                description_zh: channel.description || `来自 ${cap.extension_name} 的扩展通道`,
                icon: 'extension',
                category: 'extension',
              })
            }
          } else if (cap.type === 'hybrid' && cap.channels) {
            // Hybrid extensions can also provide channels
            for (const channel of cap.channels) {
              extensionChannelTypes.push({
                id: `extension:${cap.extension_id}:${channel.name}`,
                name: channel.name,
                name_zh: channel.name,
                description: channel.description || `Extension channel from ${cap.extension_name}`,
                description_zh: channel.description || `来自 ${cap.extension_name} 的扩展通道`,
                icon: 'extension',
                category: 'extension',
              })
            }
          }
        }

        // Combine built-in and extension channel types
        setChannelTypes([...builtInTypes, ...extensionChannelTypes])
      } catch {
        // If extension capabilities fail, just use built-in types
        setChannelTypes(builtInTypes)
      }

      // Load channels
      if (onListChannels) {
        const response = await onListChannels()
        setChannels(response.channels || [])
      } else {
        const response = await api.listMessageChannels()
        setChannels(response.channels || [])
      }
    } catch (error) {
      handleError(error, { operation: 'Load alert channels data', showToast: false })
      setChannelTypes([])
      setChannels([])
    } finally {
      setLoading(false)
    }
  }

  const getChannelsForType = (typeId: string) => {
    return channels.filter(c => c.channel_type === typeId)
  }

  // Handle type selection and load schema
  const handleTypeSelect = async (type: ChannelTypeInfo) => {
    setSchemaLoading(true)
    try {
      // Check if this is an extension channel
      if (type.id.startsWith('extension:')) {
        // Parse the extension ID and channel name from the type ID
        const [, extensionId, channelName] = type.id.split(':')

        // Find the extension capability
        const extension = extensionCapabilities.find(cap => cap.extension_id === extensionId)
        if (!extension) {
          throw new Error('Extension not found')
        }

        // Find the channel descriptor
        const channel = extension.channels?.find(ch => ch.name === channelName)
        if (!channel) {
          throw new Error('Channel not found in extension')
        }

        // Convert the extension's channel schema to UnifiedPluginType
        const pluginType: UnifiedPluginType = {
          id: type.id,
          type: 'alert_channel',
          name: channel.name,
          description: channel.description,
          icon: <Puzzle className="h-6 w-6" />,
          color: CHANNEL_TYPE_INFO.extension.iconBg,
          config_schema: channel.config_schema ? {
            type: 'object',
            properties: Object.fromEntries(
              Object.entries(channel.config_schema).map(([key, prop]) => {
                const typedProp = prop as any
                return [key, {
                  type: typedProp.type || 'string',
                  description: typedProp.description,
                  default: typedProp.default,
                  enum: typedProp.enum,
                  minimum: typedProp.minimum,
                  maximum: typedProp.maximum,
                  secret: typedProp.secret || false,
                }]
              })
            ),
            required: [],
            ui_hints: {},
          } : {
            type: 'object',
            properties: {},
            required: [],
            ui_hints: {},
          },
          can_add_multiple: true,
          builtin: false,
        }
        setSelectedType(pluginType)
      } else {
        // Built-in channel type
        const schema = await api.getChannelSchema(type.id)
        setSelectedType(toUnifiedPluginType(schema))
      }
      setView('detail')
    } catch (error) {
      handleError(error, { operation: 'Load channel schema', showToast: false })
    } finally {
      setSchemaLoading(false)
    }
  }

  // Handle create channel
  const handleCreate = async (name: string, config: Record<string, unknown>) => {
    const type = selectedType!
    const data: any = {
      name,
      channel_type: type.id,
      ...config,
    }

    if (onCreateChannel) {
      await onCreateChannel(data)
    } else {
      await api.createMessageChannel(data)
    }
    return name // Return the ID (name for channels)
  }

  // Handle update channel
  const handleUpdate = async (id: string, config: Record<string, unknown>) => {
    if (onUpdateChannel) {
      await onUpdateChannel(id, config)
    } else {
      await api.updateMessageChannel(id, config)
    }
  }

  // Handle delete channel
  const handleDelete = async (id: string) => {
    if (onDeleteChannel) {
      await onDeleteChannel(id)
    } else {
      await api.deleteMessageChannel(id)
    }
  }

  // Handle test channel
  const handleTest = async (id: string) => {
    if (onTestChannel) {
      const result = await onTestChannel(id)
      return {
        success: result.success,
        message: result.message,
      }
    }

    const result = await api.testMessageChannel(id)
    return {
      success: result.success,
      message: result.message,
    }
  }

  // Open filter dialog
  const handleOpenFilterDialog = async (channel: AlertChannel) => {
    setFilterDialogChannel(channel)
    try {
      const filter = await api.getChannelFilter(channel.name)
      setFilterConfig(filter)
    } catch (error) {
      // Use default filter on error
      setFilterConfig({
        message_types: [],
        source_types: [],
        categories: [],
        min_severity: null,
        source_ids: [],
      })
    }
  }

  // Save filter configuration
  const handleSaveFilter = async () => {
    if (!filterDialogChannel) return
    setSavingFilter(true)
    try {
      await api.updateChannelFilter(filterDialogChannel.name, filterConfig)
      setFilterDialogChannel(null)
    } catch (error) {
      handleError(error, { operation: 'Save filter' })
    } finally {
      setSavingFilter(false)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (schemaLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  // ========== LIST VIEW ==========
  if (view === 'list') {
    return (
      <>
        {/* Channel Type Cards Grid */}
        <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {channelTypes.map((type) => {
            const typeChannels = getChannelsForType(type.id)
            const info = CHANNEL_TYPE_INFO[type.id] || CHANNEL_TYPE_INFO.webhook
            const hasEnabled = typeChannels.some((c) => c.enabled)

            return (
              <Card
                key={type.id}
                className={cn(
                  "cursor-pointer transition-all duration-200 hover:shadow-md",
                  hasEnabled && "border-green-500 border-2"
                )}
                onClick={() => handleTypeSelect(type)}
              >
                <CardHeader className="pb-3">
                  <div className={cn("flex items-center justify-center w-12 h-12 rounded-lg", info.iconBg)}>
                    {info.icon}
                  </div>
                  <CardTitle className="text-base mt-3">{info.name}</CardTitle>
                  <CardDescription className="mt-1 text-xs line-clamp-2 min-h-[2.5em]">
                    {type.description}
                  </CardDescription>
                </CardHeader>
                <CardContent className="text-sm">
                  <div className="flex justify-between items-center">
                    <span className="text-muted-foreground">{t('plugins:llm.status')}:</span>
                    <span className={hasEnabled ? "text-green-600 dark:text-green-400 font-medium" : "text-muted-foreground font-medium"}>
                      {hasEnabled ? t('alerts:enabled') : t('alerts:notConfigured')}
                    </span>
                  </div>
                  <div className="flex justify-between items-center mt-2">
                    <span className="text-muted-foreground">{t('plugins:llm.instances')}:</span>
                    <span className="font-medium">{typeChannels.length}</span>
                  </div>
                </CardContent>
              </Card>
            )
          })}
        </div>
      </>
    )
  }

  // ========== DETAIL VIEW ==========
  if (view === 'detail' && selectedType) {
    const typeId = selectedType.id || 'webhook'
    const typeChannels = getChannelsForType(typeId)
    const info = CHANNEL_TYPE_INFO[typeId] || CHANNEL_TYPE_INFO.webhook
    const pluginInstances = typeChannels.map(c => toPluginInstance(c))

    return (
      <>
        {/* Header with back button */}
        <div className="flex items-center gap-4 mb-4">
          <Button variant="ghost" size="sm" onClick={() => setView('list')} className="gap-1">
            <ArrowLeft className="h-4 w-4" />
            {t('plugins:llm.back')}
          </Button>
          <div className="flex items-center gap-3">
            <div className={cn("flex items-center justify-center w-10 h-10 rounded-lg", info.iconBg)}>
              {info.icon}
            </div>
            <div>
              <h2 className="text-2xl font-bold">{info.name}</h2>
              <p className="text-sm text-muted-foreground">{selectedType.description}</p>
            </div>
          </div>
        </div>

        {/* Instances */}
        {pluginInstances.length === 0 ? (
          <Card className="border-dashed">
            <CardContent className="flex flex-col items-center justify-center py-12">
              <div className={cn("flex items-center justify-center w-16 h-16 rounded-lg mb-4", info.iconBg)}>
                {info.icon}
              </div>
              <h3 className="text-lg font-semibold mb-1">{t('plugins:llm.noInstanceYet', { name: info.name })}</h3>
              <p className="text-sm text-muted-foreground mb-4">
                {t('plugins:llm.configureToStart', { name: info.name })}
              </p>
              <Button onClick={() => {
                setEditingInstance(null)
                setConfigDialogOpen(true)
              }}>
                <TestTube className="mr-2 h-4 w-4" />
                {t('alerts:addChannel')}
              </Button>
            </CardContent>
          </Card>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {pluginInstances.map((instance) => {
              const testResult = testResults[instance.id]

              return (
                <Card
                  key={instance.id}
                  className={cn(
                    "transition-all duration-200 hover:shadow-md",
                    instance.enabled && "border-green-500"
                  )}
                >
                  <CardHeader className="pb-3">
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <CardTitle className="text-base">{instance.name}</CardTitle>
                          {instance.enabled && (
                            <Badge variant="default" className="text-xs">{t('alerts:enabled')}</Badge>
                          )}
                        </div>
                        <CardDescription className="text-xs">
                          {instance.plugin_type}
                        </CardDescription>
                      </div>
                      <div className="flex items-center gap-1">
                        {onTestChannel && (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-8 w-8 p-0"
                            onClick={async () => {
                              const result = await onTestChannel(instance.id)
                              setTestResults(prev => ({
                                ...prev,
                                [instance.id]: result,
                              }))
                            }}
                            title={t('plugins:test')}
                          >
                            <TestTube className="h-4 w-4" />
                          </Button>
                        )}
                        {!hideFilterButton && (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-8 w-8 p-0"
                            onClick={() => {
                              const channel = typeChannels.find(c => c.name === instance.id)
                              if (channel) handleOpenFilterDialog(channel)
                            }}
                            title={t('common:messages.channels.configureFilter')}
                          >
                            <Filter className="h-4 w-4" />
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 w-8 p-0"
                          onClick={() => {
                            setEditingInstance(instance)
                            setConfigDialogOpen(true)
                          }}
                          title={t('common:edit')}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 w-8 p-0 text-destructive hover:text-destructive"
                          onClick={async () => {
                            if (confirm(t('common:messages.channels.confirmDelete', 'Are you sure you want to delete this channel?', { name: instance.name }))) {
                              try {
                                await handleDelete(instance.id)
                                await loadData()
                              } catch (err) {
                                handleError(err, { operation: 'Delete channel' })
                              }
                            }
                          }}
                          title={t('plugins:delete')}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  </CardHeader>

                  <CardContent className="pb-3">
                    <div className="space-y-2 text-sm">
                      {testResult && (
                        <div className={cn(
                          "text-xs p-2 rounded",
                          testResult.success
                            ? "bg-green-50 text-green-700 dark:bg-green-900 dark:text-green-300"
                            : "bg-red-50 text-red-700 dark:bg-red-900 dark:text-red-300"
                        )}>
                          {testResult.message}
                        </div>
                      )}
                    </div>
                  </CardContent>
                </Card>
              )
            })}
          </div>
        )}

        {/* Add Instance Button */}
        <div className="mt-4">
          <Button onClick={() => {
            setEditingInstance(null)
            setConfigDialogOpen(true)
          }}>
            <TestTube className="mr-2 h-4 w-4" />
            {t('alerts:addChannel')}
          </Button>
        </div>

        {/* Unified Config Dialog */}
        <UniversalPluginConfigDialog
          open={configDialogOpen}
          onOpenChange={(open) => {
            setConfigDialogOpen(open)
            if (!open) {
              setEditingInstance(null)
              setTestResults({})
            }
          }}
          pluginType={selectedType}
          instances={pluginInstances}
          editingInstance={editingInstance}
          onCreate={handleCreate}
          onUpdate={handleUpdate}
          onDelete={handleDelete}
          onTest={handleTest}
          onRefresh={loadData}
          testResults={testResults}
          setTestResults={setTestResults}
        />

        {/* Filter Configuration Dialog */}
        <Dialog open={!!filterDialogChannel} onOpenChange={() => setFilterDialogChannel(null)}>
          <DialogContent className="max-w-lg">
            <DialogHeader>
              <DialogTitle>{t('common:messages.channels.filterConfig')}</DialogTitle>
              <DialogDescription>
                {t('common:messages.channels.filterConfigDesc')}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-4">
              {/* Message Types */}
              <div className="space-y-2">
                <Label>{t('common:messages.channels.messageTypes')}</Label>
                <div className="flex gap-4">
                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={filterConfig.message_types.length === 0 || filterConfig.message_types.includes('notification')}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          setFilterConfig(prev => ({
                            ...prev,
                            message_types: [...new Set([...prev.message_types, 'notification' as MessageType])]
                          }))
                        } else {
                          setFilterConfig(prev => ({
                            ...prev,
                            message_types: prev.message_types.filter(t => t !== 'notification')
                          }))
                        }
                      }}
                    />
                    {t('common:messages.channels.notification')}
                  </label>
                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={filterConfig.message_types.length === 0 || filterConfig.message_types.includes('data_push')}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          setFilterConfig(prev => ({
                            ...prev,
                            message_types: [...new Set([...prev.message_types, 'data_push' as MessageType])]
                          }))
                        } else {
                          setFilterConfig(prev => ({
                            ...prev,
                            message_types: prev.message_types.filter(t => t !== 'data_push')
                          }))
                        }
                      }}
                    />
                    {t('common:messages.channels.dataPush')}
                  </label>
                </div>
                <p className="text-xs text-muted-foreground">
                  {t('common:messages.channels.messageTypesHint')}
                </p>
              </div>

              {/* Source Types */}
              <div className="space-y-2">
                <Label>{t('common:messages.channels.sourceTypes')}</Label>
                <div className="flex flex-wrap gap-2">
                  {['device', 'rule', 'telemetry', 'schedule', 'llm', 'system'].map(st => (
                    <label key={st} className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={filterConfig.source_types.includes(st)}
                        onCheckedChange={(checked) => {
                          if (checked) {
                            setFilterConfig(prev => ({
                              ...prev,
                              source_types: [...prev.source_types, st]
                            }))
                          } else {
                            setFilterConfig(prev => ({
                              ...prev,
                              source_types: prev.source_types.filter(t => t !== st)
                            }))
                          }
                        }}
                      />
                      {st}
                    </label>
                  ))}
                </div>
                <p className="text-xs text-muted-foreground">
                  {t('common:messages.channels.sourceTypesHint')}
                </p>
              </div>

              {/* Categories */}
              <div className="space-y-2">
                <Label>{t('common:messages.channels.categories')}</Label>
                <div className="flex flex-wrap gap-2">
                  {['alert', 'system', 'business', 'notification'].map(cat => (
                    <label key={cat} className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={filterConfig.categories.includes(cat)}
                        onCheckedChange={(checked) => {
                          if (checked) {
                            setFilterConfig(prev => ({
                              ...prev,
                              categories: [...prev.categories, cat]
                            }))
                          } else {
                            setFilterConfig(prev => ({
                              ...prev,
                              categories: prev.categories.filter(t => t !== cat)
                            }))
                          }
                        }}
                      />
                      {cat}
                    </label>
                  ))}
                </div>
              </div>

              {/* Min Severity */}
              <div className="space-y-2">
                <Label>{t('common:messages.channels.minSeverity')}</Label>
                <Select
                  value={filterConfig.min_severity || 'all'}
                  onValueChange={(value) => {
                    setFilterConfig(prev => ({
                      ...prev,
                      min_severity: value === 'all' ? null : value as MessageSeverity | null
                    }))
                  }}
                >
                  <SelectTrigger>
                    <SelectValue placeholder={t('common:messages.channels.allSeverities')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t('common:messages.channels.allSeverities')}</SelectItem>
                    <SelectItem value="info">Info</SelectItem>
                    <SelectItem value="warning">Warning</SelectItem>
                    <SelectItem value="critical">Critical</SelectItem>
                    <SelectItem value="emergency">Emergency</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Preview */}
              <div className="p-3 bg-muted/50 rounded-md">
                <p className="text-sm font-medium mb-1">{t('common:messages.channels.filterPreview')}</p>
                <p className="text-xs text-muted-foreground">
                  {filterConfig.message_types.length === 0 && filterConfig.source_types.length === 0
                    ? t('common:messages.channels.filterAcceptAll')
                    : t('common:messages.channels.filterWillMatch', {
                        types: filterConfig.message_types.length > 0 ? filterConfig.message_types.join(', ') : t('common:messages.channels.all'),
                        sources: filterConfig.source_types.length > 0 ? filterConfig.source_types.join(', ') : t('common:messages.channels.all')
                      })}
                </p>
              </div>
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={() => setFilterDialogChannel(null)}>
                {t('common:cancel')}
              </Button>
              <Button onClick={handleSaveFilter} disabled={savingFilter}>
                {savingFilter ? t('common:saving') : t('common:save')}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    )
  }

  return null
}
