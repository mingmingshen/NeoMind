import { useState, useEffect, useCallback, useRef, useMemo } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { X, RefreshCw, Eye, Brain, Wrench, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { FormField } from "@/components/ui/field"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { ConfigFormBuilder } from "@/components/plugins/ConfigFormBuilder"
import { useToast } from "@/hooks/use-toast"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
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
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [saving, setSaving] = useState(false)
  const [newInstanceName, setNewInstanceName] = useState("")
  const [nameError, setNameError] = useState<string | null>(null)
  const [internalTestResults, setInternalTestResults] = useState<Record<string, { success: boolean; message: string }>>({})

  // Ollama model state
  const [ollamaModels, setOllamaModels] = useState<OllamaModel[]>([])
  const [loadingModels, setLoadingModels] = useState(false)
  const [selectedModel, setSelectedModel] = useState("")
  const [ollamaEndpoint, setOllamaEndpoint] = useState("http://localhost:11434")

  // Auto-detected capabilities state
  const [detectedCapabilities, setDetectedCapabilities] = useState({
    supports_multimodal: false,
    supports_thinking: false,
    supports_tools: true,
    max_context: 8192,
  })

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  const testResults = externalTestResults ?? internalTestResults
  const setTestResults = setExternalTestResults ?? setInternalTestResults
  const isOllamaBackend = pluginType.type === "llm_backend" && pluginType.id === "ollama"

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
  }, [open, pluginType.id, pluginType.type, fetchOllamaModels, editingInstance])

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
          <Eye className="h-3 w-3 mr-1" />
          Vision
        </Badge>
      )}
      {detectedCapabilities.supports_thinking && (
        <Badge variant="outline" className="text-xs">
          <Brain className="h-3 w-3 mr-1" />
          Thinking
        </Badge>
      )}
      {detectedCapabilities.supports_tools && (
        <Badge variant="outline" className="text-xs">
          <Wrench className="h-3 w-3 mr-1" />
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
    if (model.supports_multimodal) icons.push(<Eye key="vision" className="h-3 w-3 text-blue-500" />)
    if (model.supports_thinking) icons.push(<Brain key="thinking" className="h-3 w-3 text-purple-500" />)
    if (model.supports_tools) icons.push(<Wrench key="tools" className="h-3 w-3 text-orange-500" />)
    if (icons.length === 0) return null
    return <span className="flex items-center gap-0.5">{icons}</span>
  }

  const handleClose = () => {
    if (!saving) {
      onOpenChange(false)
    }
  }

  // Stable key for form content to prevent unnecessary remounting
  // Changes only when dialog type/instance changes, not on every keystroke
  const formKey = useMemo(() =>
    `${pluginType.id}-${editingInstance?.id || 'new'}-${open ? 'open' : 'closed'}`,
    [pluginType.id, editingInstance?.id, open]
  )

  // Render form content - defined as a function to avoid JSX-in-JSX issues
  // but called directly (not as component) to prevent remounting
  const renderFormContent = () => (
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
        <div className="flex items-center justify-between p-3 bg-muted/30 rounded-lg">
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
        <div className="p-3 bg-muted/30 rounded-lg text-sm text-muted-foreground">
          {t("plugins:llm.noModelsFound", { defaultValue: "No models found. Click refresh to fetch from Ollama." })}
        </div>
      )}

      {/* Capability display for non-Ollama LLM backends */}
      {pluginType.type === "llm_backend" && !isOllamaBackend && (
        <div className="space-y-2">
          <label className="text-sm font-medium">{t("plugins:llm.capabilities", { defaultValue: "Capabilities" })}</label>
          {renderCapabilityBadges()}
        </div>
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
                <span className={pluginType.color}>{pluginType.icon}</span>
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">
                    {isEditing
                      ? t("plugins:editInstance", { defaultValue: "Edit Instance" })
                      : pluginType.name
                    }
                  </h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {isEditing
                      ? t("plugins:editInstanceDesc", { defaultValue: "Configure this instance" })
                      : pluginType.description
                    }
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={saving} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                {renderFormContent()}
              </div>
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
          onClick={() => !saving && onOpenChange(false)}
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
                <span className={pluginType.color}>{pluginType.icon}</span>
                <h2 className="text-lg font-semibold leading-none truncate">
                  {isEditing
                    ? t("plugins:editInstance", { defaultValue: "Edit Instance" })
                    : pluginType.name
                  }
                </h2>
              </div>
              <p className="text-sm text-muted-foreground">
                {isEditing
                  ? t("plugins:editInstanceDesc", { defaultValue: "Configure this instance" })
                  : pluginType.description
                }
              </p>
            </div>
            <button
              onClick={handleClose}
              disabled={saving}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-6">
            {renderFormContent()}
          </div>
        </div>
      )}
    </>
  )
}
