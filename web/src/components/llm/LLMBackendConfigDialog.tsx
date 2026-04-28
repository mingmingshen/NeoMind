import { getPortalRoot } from '@/lib/portal'
import { useEffect, useState, useCallback } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { X, Loader2, Settings } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { fetchAPI } from "@/lib/api"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { useToast } from "@/hooks/use-toast"
import { cn } from "@/lib/utils"
import { dialogHeader } from '@/design-system/tokens/size'
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { FormField } from "@/components/ui/field"

interface LLMBackendConfigDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  backend?: LlmBackendInstance | null
  mode: 'create' | 'edit'
  backendType?: string  // Required for create mode
  onRefresh?: () => void  // Callback to refresh the backend list
}

interface LlmBackendInstance {
  id: string
  name: string
  backend_type: string
  config: Record<string, unknown>
  enabled: boolean
}

interface BackendTypeDefinition {
  id: string
  name: string
  description: string
  config_schema?: Record<string, unknown>
}

export function LLMBackendConfigDialog({
  open,
  onOpenChange,
  backend,
  mode,
  backendType,
  onRefresh,
}: LLMBackendConfigDialogProps) {
  const { t } = useTranslation()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()
  const { toast } = useToast()

  const [loading, setLoading] = useState(false)
  const [config, setConfig] = useState<Record<string, unknown>>({})
  const [name, setName] = useState('')
  const [endpoint, setEndpoint] = useState('')
  const [model, setModel] = useState('')
  const [apiKey, setApiKey] = useState('')

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    if (backend && mode === 'edit') {
      setName(backend.name)
      setConfig(backend.config || {})
      setEndpoint((backend.config as any)?.endpoint || '')
      setModel((backend.config as any)?.model || '')
      setApiKey('')
    } else {
      setName('')
      setConfig({})
      setEndpoint('')
      setModel('')
      setApiKey('')
    }
  }, [backend, mode])

  const handleSubmit = async () => {
    setLoading(true)
    try {
      if (mode === 'create') {
        if (!backendType) {
          toast({
            title: t('common:error'),
            description: t('settings:llm.backendTypeRequired', { defaultValue: 'Backend type is required' }),
            variant: 'destructive',
          })
          return
        }

        const createData = {
          name,
          backend_type: backendType,
          endpoint: endpoint || undefined,
          model,
          api_key: apiKey || undefined,
        }

        await fetchAPI('/llm-backends', {
          method: 'POST',
          body: JSON.stringify(createData),
        })

        toast({
          title: t('common:success'),
          description: t('settings:llm.backendCreated', { defaultValue: 'Backend created successfully' }),
        })

        onRefresh?.()
      } else {
        await fetchAPI(`/api/llm-backends/${backend?.id}`, {
          method: 'PUT',
          body: JSON.stringify({ name, config }),
        })
      }
      onOpenChange(false)
    } catch (error) {
      console.error('Failed to save backend:', error)
      toast({
        title: t('common:error'),
        description: mode === 'create'
          ? t('settings:llm.createFailed', { defaultValue: 'Failed to create backend' })
          : t('settings:llm.updateFailed', { defaultValue: 'Failed to update backend' }),
        variant: 'destructive',
      })
    } finally {
      setLoading(false)
    }
  }

  const handleClose = useCallback(() => {
    if (!loading) {
      onOpenChange(false)
    }
  }, [loading, onOpenChange])

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-50 bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className={dialogHeader}
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <Settings className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">
                    {mode === 'create' ? t('settings:llm.create') : t('settings:llm.edit')}
                  </h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {backend?.name || t('settings:llm.newBackend', { defaultValue: 'New Backend' })}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={loading} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <FormSectionGroup>
      <FormField
        label={t('settings:llm.name')}
        required
      >
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('settings:llm.namePlaceholder')}
        />
      </FormField>

      {mode === 'create' && (
        <>
          <FormField
            label={t('settings:llm.endpoint')}
          >
            <Input
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="http://localhost:11434"
            />
          </FormField>

          <FormField
            label={t('settings:llm.model')}
            required
          >
            <Input
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="qwen2.5:7b"
            />
          </FormField>

          <FormField
            label={t('settings:llm.apiKey')}
          >
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t('settings:llm.apiKeyPlaceholder', { defaultValue: 'Optional API key' })}
            />
          </FormField>
        </>
      )}

      {mode === 'edit' && (
        <div className="text-sm text-muted-foreground p-4 border rounded-lg bg-muted-30">
          {t('settings:llm.configPlaceholder', { defaultValue: 'Configuration fields will be dynamically generated based on backend type' })}
        </div>
      )}
    </FormSectionGroup>
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={loading} className="min-w-[80px]">
                {t('common:cancel')}
              </Button>
              <Button onClick={handleSubmit} disabled={loading} className="min-w-[80px]">
                {loading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {mode === 'create' ? t('common:create') : t('common:save')}
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
            'max-h-[calc(100vh-2rem)] sm:max-h-[80vh]',
            'flex flex-col',
            'max-w-2xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <Settings className="h-5 w-5 text-primary" />
              <h2 className="text-lg font-semibold leading-none truncate">
                {mode === 'create' ? t('settings:llm.create') : t('settings:llm.edit')}
              </h2>
            </div>
            <button
              onClick={handleClose}
              disabled={loading}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <FormSectionGroup>
      <FormField
        label={t('settings:llm.name')}
        required
      >
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('settings:llm.namePlaceholder')}
        />
      </FormField>

      {mode === 'create' && (
        <>
          <FormField
            label={t('settings:llm.endpoint')}
          >
            <Input
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="http://localhost:11434"
            />
          </FormField>

          <FormField
            label={t('settings:llm.model')}
            required
          >
            <Input
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="qwen2.5:7b"
            />
          </FormField>

          <FormField
            label={t('settings:llm.apiKey')}
          >
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t('settings:llm.apiKeyPlaceholder', { defaultValue: 'Optional API key' })}
            />
          </FormField>
        </>
      )}

      {mode === 'edit' && (
        <div className="text-sm text-muted-foreground p-4 border rounded-lg bg-muted-30">
          {t('settings:llm.configPlaceholder', { defaultValue: 'Configuration fields will be dynamically generated based on backend type' })}
        </div>
      )}
    </FormSectionGroup>
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted-30">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={loading}>
              {t('common:cancel')}
            </Button>
            <Button size="sm" onClick={handleSubmit} disabled={loading}>
              {loading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {mode === 'create' ? t('common:create') : t('common:save')}
            </Button>
          </div>
        </div>
      )}
    </>,
    getPortalRoot()
  )
}
