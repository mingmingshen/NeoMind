import { useState, useEffect, useCallback, useRef, useMemo } from "react"
import { useTranslation } from "react-i18next"
import { RefreshCw, Eye, Brain, Wrench, Loader2, Server } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { FormField } from "@/components/ui/field"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { ConfigFormBuilder } from "@/components/plugins/ConfigFormBuilder"
import { UnifiedFormDialog } from "@/components/dialog/UnifiedFormDialog"
import { useToast } from "@/hooks/use-toast"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import type { PluginConfigSchema } from "@/types"

/**
 * Ollama model with capabilities
 */
interface OllamaModel {
  name: string
  size?: number
  modified_at?: string
  digest?: string
  details?: {
    format?: string
    family?: string
    families?: string[]
    parameter_size?: string
    quantization_level?: string
  }
  supports_multimodal: boolean
  supports_thinking: boolean
  supports_tools: boolean
  max_context: number
}

/**
 * Unified plugin instance data structure
 */
export interface PluginInstance {
  id: string
  name: string
  plugin_type: string
  enabled: boolean
  running?: boolean
  config?: Record<string, unknown>
  status?: {
    connected?: boolean
    active?: boolean
    error?: string
    latency_ms?: number
  }
  [key: string]: unknown
}

/**
 * Unified plugin type definition
 */
export interface UnifiedPluginType {
  id: string
  type: "llm_backend" | "device_adapter" | "alert_channel"
  name: string
  description: string
  icon: React.ReactNode
  color: string
  config_schema: PluginConfigSchema
  can_add_multiple: boolean
  builtin: boolean
  requires_api_key?: boolean
  supports_streaming?: boolean
  default_model?: string
  default_endpoint?: string
}

interface UniversalPluginConfigDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  pluginType: UnifiedPluginType
  instances: PluginInstance[]
  editingInstance?: PluginInstance | null
  onCreate: (name: string, config: Record<string, unknown>) => Promise<string>
  onUpdate: (id: string, config: Record<string, unknown>) => Promise<void>
  onDelete?: (id: string) => Promise<void>
  onTest?: (id: string) => Promise<{ success: boolean; message?: string; error?: string; latency_ms?: number }>
  onRefresh: () => Promise<void>
  testResults?: Record<string, { success: boolean; message: string }>
  setTestResults?: (results: Record<string, { success: boolean; message: string }>) => void
}

export function UniversalPluginConfigDialog(props: UniversalPluginConfigDialogProps) {
  const {
    open,
    onOpenChange,
    pluginType,
    editingInstance,
    onCreate,
    onUpdate,
    onRefresh,
    testResults: externalTestResults,
    setTestResults: setExternalTestResults,
  } = props

  const { t } = useTranslation(["common", "plugins", "devices"])
  const { toast } = useToast()
  const { handleError } = useErrorHandler()

  const [saving, setSaving] = useState(false)
  const [newInstanceName, setNewInstanceName] = useState("")
  const [nameError, setNameError] = useState<string | null>(null)
  const [internalTestResults, setInternalTestResults] = useState<Record<string, { success: boolean; message: string }>>({})

  // Ollama model state
  const [ollamaModels, setOllamaModels] = useState<OllamaModel[]>([])
  const [loadingModels, setLoadingModels] = useState(false)
  const [selectedModel, setSelectedModel] = useState("")
  const [ollamaEndpoint, setOllamaEndpoint] = useState("http://localhost:11434")

  // llama.cpp server info state
  const [llamacppEndpoint, setLlamacppEndpoint] = useState("http://127.0.0.1:8080")
  const [llamacppApiKey, setLlamacppApiKey] = useState("")
  const [llamacppServerInfo, setLlamacppServerInfo] = useState<{
    status: string
    health: { status: string; latency_ms: number }
    server: {
      model_name?: string
      n_ctx?: number
      total_slots?: number
      version?: string
    }
    capabilities: {
      supports_streaming: boolean
      supports_multimodal: boolean
      supports_thinking: boolean
      supports_tools: boolean
      max_context: number
    }
  } | null>(null)
  const [loadingLlamacppInfo, setLoadingLlamacppInfo] = useState(false)

  // Auto-detected capabilities state
  const [detectedCapabilities, setDetectedCapabilities] = useState({
    supports_multimodal: false,
    supports_thinking: false,
    supports_tools: true,
    max_context: 8192,
  })

  const testResults = externalTestResults ?? internalTestResults
  const setTestResults = setExternalTestResults ?? setInternalTestResults
  const isOllamaBackend = pluginType.type === "llm_backend" && pluginType.id === "ollama"
  const isLlamaCppBackend = pluginType.type === "llm_backend" && pluginType.id === "llamacpp"

  const fetchOllamaModels = useCallback(async (endpoint?: string) => {
    if (!isOllamaBackend) return

    setLoadingModels(true)
    try {
      const response = await api.listOllamaModels(endpoint)
      setOllamaModels(response.models || [])

      if (editingInstance?.config?.model) {
        const currentModel = String(editingInstance.config.model)
        setSelectedModel(currentModel)
        const modelWithCaps = response.models?.find(m => m.name === currentModel)
        if (modelWithCaps) {
          setDetectedCapabilities({
            supports_multimodal: modelWithCaps.supports_multimodal,
            supports_thinking: modelWithCaps.supports_thinking,
            supports_tools: modelWithCaps.supports_tools,
            max_context: modelWithCaps.max_context,
          })
        }
      }
    } catch (error) {
      handleError(error, { operation: 'Fetch Ollama models', showToast: false })
    } finally {
      setLoadingModels(false)
    }
  }, [isOllamaBackend, editingInstance, handleError])

  const fetchLlamacppServerInfo = useCallback(async (endpoint?: string, apiKey?: string) => {
    if (!isLlamaCppBackend) return

    setLoadingLlamacppInfo(true)
    try {
      const response = await api.listLlamaCppServerInfo(endpoint, apiKey)
      setLlamacppServerInfo(response)
      if (response.status === "ok") {
        setDetectedCapabilities({
          supports_multimodal: response.capabilities.supports_multimodal,
          supports_thinking: response.capabilities.supports_thinking,
          supports_tools: response.capabilities.supports_tools,
          max_context: response.capabilities.max_context,
        })
      }
    } catch (error) {
      handleError(error, { operation: 'Fetch llama.cpp server info', showToast: false })
      setLlamacppServerInfo(null)
    } finally {
      setLoadingLlamacppInfo(false)
    }
  }, [isLlamaCppBackend, handleError])

  // Track previous open state to detect dialog open transitions
  const prevOpenRef = useRef(open)

  // Reset form when dialog opens (only on open transition, not on every render)
  useEffect(() => {
    // Only reset when dialog transitions from closed to open
    if (open && !prevOpenRef.current) {
      setNewInstanceName("")
      setSelectedModel("")
      setNameError(null)

      if (editingInstance && (editingInstance as any).capabilities) {
        const existingCaps = (editingInstance as any).capabilities
        setDetectedCapabilities({
          supports_multimodal: existingCaps.supports_multimodal ?? false,
          supports_thinking: existingCaps.supports_thinking ?? false,
          supports_tools: existingCaps.supports_tools ?? true,
          max_context: existingCaps.max_context ?? 8192,
        })
        if (pluginType.id === "ollama") {
          const instanceEndpoint = editingInstance?.config?.endpoint as string | undefined
          fetchOllamaModels(instanceEndpoint)
          if (instanceEndpoint) {
            setOllamaEndpoint(instanceEndpoint)
          }
        }
        if (pluginType.id === "llamacpp") {
          const instanceEndpoint = editingInstance?.config?.endpoint as string | undefined
          const instanceApiKey = editingInstance?.config?.api_key as string | undefined
          if (instanceEndpoint) {
            setLlamacppEndpoint(instanceEndpoint)
          }
          if (instanceApiKey) {
            setLlamacppApiKey(instanceApiKey)
          }
          fetchLlamacppServerInfo(instanceEndpoint, instanceApiKey)
        }
      } else if (pluginType.type === "llm_backend") {
        if (pluginType.id === "ollama") {
          setDetectedCapabilities({
            supports_multimodal: false,
            supports_thinking: true,
            supports_tools: true,
            max_context: 8192,
          })
          fetchOllamaModels()
        } else if (pluginType.id === "openai" || pluginType.id === "google" || pluginType.id === "anthropic") {
          setDetectedCapabilities({
            supports_multimodal: true,
            supports_thinking: false,
            supports_tools: true,
            max_context: 128000,
          })
        } else if (pluginType.id === "llamacpp") {
          setDetectedCapabilities({
            supports_multimodal: false,
            supports_thinking: true,
            supports_tools: true,
            max_context: 4096,
          })
          setLlamacppServerInfo(null)
        } else {
          setDetectedCapabilities({
            supports_multimodal: false,
            supports_thinking: false,
            supports_tools: true,
            max_context: 128000,
          })
        }
      }
    }
    // Update ref for next render
    prevOpenRef.current = open
  }, [open, pluginType.id, pluginType.type, fetchOllamaModels, fetchLlamacppServerInfo, editingInstance])

  const handleModelChange = (modelName: string) => {
    setSelectedModel(modelName)
    const model = ollamaModels.find(m => m.name === modelName)
    if (model) {
      setDetectedCapabilities({
        supports_multimodal: model.supports_multimodal,
        supports_thinking: model.supports_thinking,
        supports_tools: model.supports_tools,
        max_context: model.max_context,
      })
    }
  }

  const getInstanceStatus = (instance: PluginInstance) => {
    if (instance.status?.connected !== undefined) return instance.status.connected
    if (instance.status?.active !== undefined) return instance.status.active
    return instance.running ?? instance.enabled
  }

  const handleCreate = async (values: Record<string, unknown>) => {
    if (!newInstanceName.trim()) {
      setNameError(t("plugins:instanceNameRequired", { defaultValue: "Instance name is required" }))
      return
    }

    setSaving(true)
    try {
      let configWithCaps = values
      if (pluginType.type === "llm_backend") {
        configWithCaps = {
          ...values,
          capabilities: detectedCapabilities,
          ...(isOllamaBackend && selectedModel ? { model: selectedModel } : {}),
          ...(isOllamaBackend ? { endpoint: ollamaEndpoint } : {}),
          ...(isLlamaCppBackend ? { endpoint: llamacppEndpoint } : {}),
          ...(isLlamaCppBackend && llamacppApiKey ? { api_key: llamacppApiKey } : {}),
          ...(isLlamaCppBackend && llamacppServerInfo?.server?.model_name ? { model: llamacppServerInfo.server.model_name } : {}),
        }
      }

      await onCreate(newInstanceName.trim(), configWithCaps)
      toast({
        title: t("common:success"),
        description: t("plugins:instanceCreated", { defaultValue: "Instance created successfully" }),
      })
      setNewInstanceName("")
      setSelectedModel("")
      onOpenChange(false)
      await onRefresh()
    } catch (error) {
      toast({
        title: t("common:failed"),
        description: String(error),
        variant: "destructive",
      })
    } finally {
      setSaving(false)
    }
  }

  const handleUpdate = async (values: Record<string, unknown>) => {
    if (!editingInstance) return

    setSaving(true)
    try {
      let configWithCaps = values
      if (pluginType.type === "llm_backend") {
        configWithCaps = {
          ...values,
          capabilities: detectedCapabilities,
          ...(isOllamaBackend && selectedModel ? { model: selectedModel } : {}),
          ...(isOllamaBackend ? { endpoint: ollamaEndpoint } : {}),
          ...(isLlamaCppBackend ? { endpoint: llamacppEndpoint } : {}),
          ...(isLlamaCppBackend && llamacppApiKey ? { api_key: llamacppApiKey } : {}),
          ...(isLlamaCppBackend && llamacppServerInfo?.server?.model_name ? { model: llamacppServerInfo.server.model_name } : {}),
        }
      }

      await onUpdate(editingInstance.id, configWithCaps)
      toast({
        title: t("common:success"),
        description: t("plugins:instanceUpdated", { defaultValue: "Instance updated successfully" }),
      })
      onOpenChange(false)
      await onRefresh()
    } catch (error) {
      toast({
        title: t("common:failed"),
        description: String(error),
        variant: "destructive",
      })
    } finally {
      setSaving(false)
    }
  }

  const getConfigSchema = () => {
    const schema = { ...pluginType.config_schema }

    // For Ollama backends, exclude endpoint and model fields since they're handled separately
    if (isOllamaBackend && schema.properties) {
      const { model, endpoint, ...restProperties } = schema.properties
      schema.properties = restProperties
      if (schema.required) {
        schema.required = schema.required.filter((field: string) => field !== 'model' && field !== 'endpoint')
      }
      if (schema.ui_hints?.field_order) {
        schema.ui_hints.field_order = schema.ui_hints.field_order.filter((field: string) => field !== 'model' && field !== 'endpoint')
      }
    }

    // For llama.cpp backends, exclude endpoint and model fields since they're handled by the server info UI
    if (isLlamaCppBackend && schema.properties) {
      const { endpoint, model, ...restProperties } = schema.properties
      schema.properties = restProperties
      if (schema.required) {
        schema.required = schema.required.filter((field: string) => field !== 'endpoint' && field !== 'model')
      }
      if (schema.ui_hints?.field_order) {
        schema.ui_hints.field_order = schema.ui_hints.field_order.filter((field: string) => field !== 'endpoint' && field !== 'model')
      }
    }

    if (editingInstance && editingInstance.config) {
      if (!schema.properties) schema.properties = {}
      for (const [key, value] of Object.entries(editingInstance.config)) {
        if (schema.properties[key]) {
          schema.properties[key] = {
            ...schema.properties[key] as any,
            default: value,
          }
        }
      }
    }

    return schema
  }

  const isEditing = !!editingInstance
  const schema = getConfigSchema()

  const renderCapabilityBadges = () => (
    <div className="flex flex-wrap gap-2 mt-2">
      {detectedCapabilities.supports_multimodal && (
        <Badge variant="outline" className="text-xs">
          <Eye className="h-4 w-4 mr-1" />
          Vision
        </Badge>
      )}
      {detectedCapabilities.supports_thinking && (
        <Badge variant="outline" className="text-xs">
          <Brain className="h-4 w-4 mr-1" />
          Thinking
        </Badge>
      )}
      {detectedCapabilities.supports_tools && (
        <Badge variant="outline" className="text-xs">
          <Wrench className="h-4 w-4 mr-1" />
          Tools
        </Badge>
      )}
      <Badge variant="secondary" className="text-xs">
        {detectedCapabilities.max_context >= 100000
          ? `${Math.round(detectedCapabilities.max_context / 1000)}k ctx`
          : `${detectedCapabilities.max_context} ctx`}
      </Badge>
    </div>
  )

  const getModelIcon = (model: OllamaModel) => {
    const icons = []
    if (model.supports_multimodal) icons.push(<Eye key="vision" className="h-4 w-4 text-info" />)
    if (model.supports_thinking) icons.push(<Brain key="thinking" className="h-4 w-4 text-accent-purple" />)
    if (model.supports_tools) icons.push(<Wrench key="tools" className="h-4 w-4 text-accent-orange" />)
    if (icons.length === 0) return null
    return <span className="flex items-center gap-0.5">{icons}</span>
  }

  // Stable key for form content to prevent unnecessary remounting
  // Changes only when dialog type/instance changes, not on every keystroke
  const formKey = useMemo(() =>
    `${pluginType.id}-${editingInstance?.id || 'new'}-${open ? 'open' : 'closed'}`,
    [pluginType.id, editingInstance?.id, open]
  )

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={
        isEditing
          ? t("plugins:editInstance", { defaultValue: "Edit Instance" })
          : pluginType.name
      }
      description={
        isEditing
          ? t("plugins:editInstanceDesc", { defaultValue: "Configure this instance" })
          : pluginType.description
      }
      icon={<span className={pluginType.color}>{pluginType.icon}</span>}
      width="xl"
      hideFooter={true}
      isSubmitting={saving}
      preventCloseOnSubmit={false}
    >
      <FormSectionGroup key={formKey}>
        {/* Instance Name Field (only for create mode) */}
        {!isEditing && (
          <FormField
            label={t("plugins:instanceName", { defaultValue: "Instance Name" })}
            required
            error={nameError || undefined}
          >
            <Input
              value={newInstanceName}
              onChange={(e) => {
                setNewInstanceName(e.target.value)
                if (nameError) setNameError(null)
              }}
              placeholder={t("plugins:instanceNamePlaceholder", { defaultValue: "My Instance" })}
              disabled={saving}
            />
          </FormField>
        )}

        {/* Edit Mode: Show instance info */}
        {isEditing && (
          <div className="flex items-center justify-between p-3 bg-muted-30 rounded-lg">
            <div>
              <h3 className="font-medium">{editingInstance.name}</h3>
              <div className="flex items-center gap-2 mt-1">
                <Badge variant={getInstanceStatus(editingInstance) ? "default" : "secondary"}>
                  {getInstanceStatus(editingInstance)
                    ? t("plugins:active", { defaultValue: "Active" })
                    : t("plugins:inactive", { defaultValue: "Inactive" })
                  }
                </Badge>
              </div>
            </div>
          </div>
        )}

        {/* Ollama endpoint configuration */}
        {isOllamaBackend && (
          <FormField label={t("plugins:llm.endpoint", { defaultValue: "Ollama Endpoint" })}>
            <div className="flex items-center gap-2">
              <Input
                value={ollamaEndpoint}
                onChange={(e) => setOllamaEndpoint(e.target.value)}
                placeholder="http://localhost:11434"
                disabled={saving}
              />
              <Button
                variant="outline"
                size="icon"
                onClick={() => fetchOllamaModels(ollamaEndpoint)}
                disabled={loadingModels}
              >
                <RefreshCw className={cn("h-4 w-4", loadingModels && "animate-spin")} />
              </Button>
            </div>
          </FormField>
        )}

        {/* Ollama model selector */}
        {isOllamaBackend && ollamaModels.length > 0 && (
          <FormField label={t("plugins:llm.selectModel", { defaultValue: "Select Model" })}>
            <div className="flex items-center gap-2">
              <Select value={selectedModel} onValueChange={handleModelChange}>
                <SelectTrigger className="flex-1">
                  <SelectValue placeholder={t("plugins:llm.selectModelPlaceholder", { defaultValue: "Select a model..." })} />
                </SelectTrigger>
                <SelectContent>
                  {ollamaModels.map((model) => (
                    <SelectItem
                      key={model.name}
                      value={model.name}
                    >
                      {model.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                variant="outline"
                size="icon"
                onClick={() => fetchOllamaModels(ollamaEndpoint)}
                disabled={loadingModels}
              >
                <RefreshCw className={cn("h-4 w-4", loadingModels && "animate-spin")} />
              </Button>
            </div>
            {selectedModel && renderCapabilityBadges()}
          </FormField>
        )}

        {/* No models message for Ollama */}
        {isOllamaBackend && ollamaModels.length === 0 && !loadingModels && (
          <div className="p-3 bg-muted-30 rounded-lg text-sm text-muted-foreground">
            {t("plugins:llm.noModelsFound", { defaultValue: "No models found. Click refresh to fetch from Ollama." })}
          </div>
        )}

        {/* Capability display for non-Ollama, non-llama.cpp LLM backends */}
        {pluginType.type === "llm_backend" && !isOllamaBackend && !isLlamaCppBackend && (
          <div className="space-y-2">
            <label className="text-sm font-medium">{t("plugins:llm.capabilities", { defaultValue: "Capabilities" })}</label>
            {renderCapabilityBadges()}
          </div>
        )}

        {/* llama.cpp endpoint check + server info */}
        {isLlamaCppBackend && (
          <FormField label={t("plugins:llm.llamacppEndpoint", { defaultValue: "llama.cpp Endpoint" })}>
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Input
                  value={llamacppEndpoint}
                  onChange={(e) => setLlamacppEndpoint(e.target.value)}
                  placeholder="http://127.0.0.1:8080"
                  disabled={saving}
                />
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => fetchLlamacppServerInfo(llamacppEndpoint, llamacppApiKey || undefined)}
                  disabled={loadingLlamacppInfo}
                >
                  <RefreshCw className={cn("h-4 w-4", loadingLlamacppInfo && "animate-spin")} />
                </Button>
              </div>

              {/* Model name - always visible when available */}
              {(() => {
                // Priority: live server info > saved config
                const liveModel = llamacppServerInfo?.status === "ok" ? llamacppServerInfo.server.model_name : undefined
                const savedModel = editingInstance?.config?.model as string | undefined
                const modelName = liveModel || savedModel

                if (!modelName) return (
                  <div className="p-2 bg-muted-30 rounded-md text-xs text-muted-foreground">
                    {t("plugins:llm.clickToDetectModel", { defaultValue: "Click the check button to detect the loaded model from the server." })}
                  </div>
                )

                return (
                  <div className="flex items-center gap-2 p-2 bg-background rounded-md border">
                    <Brain className="h-4 w-4 text-info shrink-0" />
                    <div className="min-w-0 flex-1">
                      <div className="text-xs text-muted-foreground">{t("plugins:llm.loadedModel", { defaultValue: "Loaded Model" })}</div>
                      <div className="font-medium text-sm truncate">{modelName}</div>
                    </div>
                    {!liveModel && savedModel && (
                      <Badge variant="outline" className="text-xs shrink-0">
                        {t("plugins:llm.saved", { defaultValue: "saved" })}
                      </Badge>
                    )}
                  </div>
                )
              })()}

              {/* Server info display (live details) */}
              {llamacppServerInfo && llamacppServerInfo.status === "ok" && (
                <div className="p-3 bg-muted-30 rounded-lg space-y-2 text-sm">
                  <div className="flex items-center gap-2">
                    <Server className="h-4 w-4 text-success" />
                    <span className="font-medium text-success">
                      {t("plugins:llm.serverConnected", { defaultValue: "Server connected" })}
                    </span>
                    <span className="text-muted-foreground">
                      ({llamacppServerInfo.health.latency_ms}ms)
                    </span>
                  </div>

                  {/* Server properties */}
                  {(llamacppServerInfo.server.n_ctx || llamacppServerInfo.server.total_slots || llamacppServerInfo.server.version) && (
                    <div className="flex flex-wrap gap-2">
                      {llamacppServerInfo.server.n_ctx && (
                        <Badge variant="secondary" className="text-xs">
                          {llamacppServerInfo.server.n_ctx >= 100000
                            ? `${Math.round(llamacppServerInfo.server.n_ctx / 1000)}k ctx`
                            : `${llamacppServerInfo.server.n_ctx} ctx`}
                        </Badge>
                      )}
                      {llamacppServerInfo.server.total_slots && (
                        <Badge variant="secondary" className="text-xs">
                          {llamacppServerInfo.server.total_slots} slots
                        </Badge>
                      )}
                      {llamacppServerInfo.server.version && (
                        <Badge variant="secondary" className="text-xs">
                          v{llamacppServerInfo.server.version}
                        </Badge>
                      )}
                    </div>
                  )}
                  {renderCapabilityBadges()}
                </div>
              )}

              {/* Error state */}
              {llamacppServerInfo && llamacppServerInfo.status !== "ok" && (
                <div className="p-3 bg-muted rounded-lg text-sm text-destructive">
                  {t("plugins:llm.serverUnreachable", { defaultValue: "Server unreachable" })}: {llamacppServerInfo.health.status}
                </div>
              )}
            </div>
          </FormField>
        )}

        {/* Config Form - Embedded directly */}
        <div className="mt-4">
          <ConfigFormBuilder
            schema={schema}
            onSubmit={isEditing ? handleUpdate : handleCreate}
            loading={saving}
            submitLabel={isEditing
              ? t("common:save", { defaultValue: "Save" })
              : t("common:create", { defaultValue: "Create" })
            }
          />
        </div>
      </FormSectionGroup>
    </UnifiedFormDialog>
  )
}
