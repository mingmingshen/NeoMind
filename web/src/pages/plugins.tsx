import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { PluginUploadDialog, PluginGrid } from "@/components/plugins"
import { UnifiedLLMBackendsTab } from "@/components/llm/UnifiedLLMBackendsTab"
import { UnifiedAlertChannelsTab } from "@/components/alerts/UnifiedAlertChannelsTab"
import { UnifiedDeviceConnectionsTab } from "@/components/connections"
import { Button } from "@/components/ui/button"
import { useToast } from "@/hooks/use-toast"
import { Upload } from "lucide-react"

type PluginTabValue = "llm" | "connections" | "alert-channels" | "extensions"

export function PluginsPage() {
  const { t } = useTranslation(["common", "plugins", "devices"])
  const { toast } = useToast()

  const {
    plugins,
    pluginsLoading,
    fetchPlugins,
    // Plugin actions
    enablePlugin,
    disablePlugin,
    startPlugin,
    stopPlugin,
    unregisterPlugin,
    setSelectedPlugin,
    setConfigDialogOpen,
    // LLM Backend actions
    createBackend,
    updateBackend,
    deleteBackend,
    testBackend,
  } = useStore()

  const [activeTab, setActiveTab] = useState<PluginTabValue>("llm")
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false)

  // Filter to only external dynamic plugins (exclude built-in types)
  // Built-in types: llm_backend, device_adapter, alert_channel
  const externalPlugins = plugins.filter((p) => p.path && !['llm_backend', 'device_adapter', 'alert_channel'].includes(p.plugin_type))

  // Fetch external plugins on mount and when extensions tab is activated
  useEffect(() => {
    if (activeTab === "extensions") {
      fetchPlugins({ builtin: false })
    }
  }, [fetchPlugins, activeTab])

  const tabs = [
    { value: "llm" as PluginTabValue, label: t("plugins:llmBackends") },
    { value: "connections" as PluginTabValue, label: t("plugins:deviceConnections") },
    { value: "alert-channels" as PluginTabValue, label: t("plugins:alertChannels") },
    { value: "extensions" as PluginTabValue, label: t("plugins:extensionPlugins") },
  ]

  // Plugin action handlers
  const handleToggle = async (id: string, enabled: boolean) => {
    const result = enabled ? await enablePlugin(id) : await disablePlugin(id)
    if (result) {
      toast({
        title: t(enabled ? "plugins:pluginEnabled" : "plugins:pluginDisabled"),
      })
    } else {
      toast({
        title: t("plugins:actionFailed"),
        variant: "destructive",
      })
    }
    // Refresh to get updated state
    fetchPlugins({ builtin: false })
    return result
  }

  const handleStart = async (id: string) => {
    const result = await startPlugin(id)
    if (result) {
      toast({
        title: t("plugins:pluginStarted"),
      })
    } else {
      toast({
        title: t("plugins:actionFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleStop = async (id: string) => {
    const result = await stopPlugin(id)
    if (result) {
      toast({
        title: t("plugins:pluginStopped"),
      })
    } else {
      toast({
        title: t("plugins:actionFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleConfigure = (id: string) => {
    const plugin = plugins.find((p) => p.id === id)
    if (plugin) {
      setSelectedPlugin(plugin)
      setConfigDialogOpen(true)
    }
  }

  const handleDelete = async (id: string) => {
    const result = await unregisterPlugin(id)
    if (result) {
      toast({
        title: t("plugins:unregisterSuccess"),
      })
    } else {
      toast({
        title: t("plugins:unregisterFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleRefresh = async () => {
    await fetchPlugins({ builtin: false })
    toast({
      title: t("plugins:refreshed"),
    })
    return true
  }

  return (
    <PageLayout>
      <PageTabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(v) => setActiveTab(v as PluginTabValue)}
      >
        {/* LLM Backends Tab */}
        <PageTabsContent value="llm" activeTab={activeTab}>
          <UnifiedLLMBackendsTab
            onCreateBackend={createBackend}
            onUpdateBackend={updateBackend}
            onDeleteBackend={deleteBackend}
            onTestBackend={testBackend}
          />
        </PageTabsContent>

        {/* Device Connections Tab */}
        <PageTabsContent value="connections" activeTab={activeTab}>
          <UnifiedDeviceConnectionsTab />
        </PageTabsContent>

        {/* Alert Channels Tab */}
        <PageTabsContent value="alert-channels" activeTab={activeTab}>
          <UnifiedAlertChannelsTab />
        </PageTabsContent>

        {/* Extension Plugins Tab - External Only */}
        <PageTabsContent value="extensions" activeTab={activeTab}>
          <div className="space-y-4">
            {/* Header */}
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-2xl font-bold tracking-tight">{t("plugins:extensionPlugins")}</h2>
                <p className="text-muted-foreground text-sm">
                  动态加载的外部插件 (.so/.wasm)
                </p>
              </div>
              <Button onClick={() => setUploadDialogOpen(true)}>
                <Upload className="mr-2 h-4 w-4" />
                {t("plugins:upload")}
              </Button>
            </div>

            {/* External Plugins List */}
            <PluginGrid
              plugins={externalPlugins}
              loading={pluginsLoading}
              onToggle={handleToggle}
              onStart={handleStart}
              onStop={handleStop}
              onConfigure={handleConfigure}
              onDelete={handleDelete}
              onRefresh={handleRefresh}
              onAddPlugin={() => setUploadDialogOpen(true)}
            />
          </div>
        </PageTabsContent>
      </PageTabs>

      {/* Upload Plugin Dialog */}
      <PluginUploadDialog
        open={uploadDialogOpen}
        onOpenChange={(open) => {
          setUploadDialogOpen(open)
          if (!open) {
            fetchPlugins({ builtin: false })
          }
        }}
        onUploadComplete={(pluginId) => {
          toast({
            title: t("plugins:pluginLoaded", { id: pluginId }),
          })
          fetchPlugins({ builtin: false })
        }}
      />
    </PageLayout>
  )
}
