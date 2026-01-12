import { useState, useEffect, useMemo } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { PluginGrid } from "@/components/plugins/PluginGrid"
import { LLMBackendsTab } from "@/components/llm/LLMBackendsTab"
import { ConnectionsTab } from "@/components/connections"
import { Button } from "@/components/ui/button"
import { useToast } from "@/hooks/use-toast"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Package, RefreshCw, Radar } from "lucide-react"

type PluginTabValue = "unified" | "llm" | "connections"
type PluginCategory = "ai" | "devices" | "notify" | "all"

export function PluginsPage() {
  const { t } = useTranslation(["common", "plugins"])
  const { toast } = useToast()

  const {
    // Unified plugin actions
    plugins,
    pluginsLoading,
    discovering,
    fetchPlugins,
    enablePlugin,
    disablePlugin,
    startPlugin,
    stopPlugin,
    unregisterPlugin,
    getPluginConfig,
    getPluginStats,
    discoverPlugins,
    // LLM Backend actions (for legacy tab)
    createBackend,
    updateBackend,
    deleteBackend,
    testBackend,
  } = useStore()

  const [activeTab, setActiveTab] = useState<PluginTabValue>("llm")
  const [categoryFilter, setCategoryFilter] = useState<PluginCategory>("all")
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [configDialogOpen, setConfigDialogOpen] = useState(false)
  const [selectedPlugin, setSelectedPlugin] = useState<any>(null)

  // Fetch plugins on mount
  useEffect(() => {
    fetchPlugins()
  }, [fetchPlugins])

  // Filter plugins by category
  const filteredPlugins = useMemo(() => {
    if (categoryFilter === "all") return plugins
    return plugins.filter((p) => p.category === categoryFilter)
  }, [plugins, categoryFilter])

  // Plugin actions
  const handleToggle = async (id: string, enabled: boolean): Promise<boolean> => {
    const result = enabled ? await enablePlugin(id) : await disablePlugin(id)
    if (result) {
      toast({
        title: enabled ? t("plugins:pluginEnabled") : t("plugins:pluginDisabled"),
      })
    } else {
      toast({
        title: t("plugins:actionFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleStart = async (id: string): Promise<boolean> => {
    const result = await startPlugin(id)
    if (result) {
      toast({ title: t("plugins:pluginStarted") })
      await fetchPlugins()
    } else {
      toast({ title: t("plugins:actionFailed"), variant: "destructive" })
    }
    return result
  }

  const handleStop = async (id: string): Promise<boolean> => {
    const result = await stopPlugin(id)
    if (result) {
      toast({ title: t("plugins:pluginStopped") })
      await fetchPlugins()
    } else {
      toast({ title: t("plugins:actionFailed"), variant: "destructive" })
    }
    return result
  }

  const handleDelete = async (id: string): Promise<boolean> => {
    const result = await unregisterPlugin(id)
    if (result) {
      toast({ title: t("plugins:unregisterSuccess") })
      await fetchPlugins()
    } else {
      toast({ title: t("plugins:unregisterFailed"), variant: "destructive" })
    }
    return result
  }

  const handleRefresh = async (id: string): Promise<boolean> => {
    await getPluginStats(id)
    await fetchPlugins()
    return true
  }

  const handleConfigure = async (id: string) => {
    const config = await getPluginConfig(id)
    setSelectedPlugin({ id, config })
    setConfigDialogOpen(true)
  }

  const handleDiscover = async () => {
    const result = await discoverPlugins()
    if (result.discovered > 0) {
      toast({
        title: t("plugins:discoveredCount", {
          count: result.discovered,
        }),
      })
    } else {
      toast({
        title: t("plugins:noPluginsDiscovered"),
      })
    }
  }

  const handleViewDevices = (id: string) => {
    // Navigate to devices page filtered by this adapter
    window.location.href = `/devices?adapter=${id}`
  }

  // Category filter options
  const categoryOptions = [
    { value: "all", label: t("plugins:allPlugins") },
    { value: "ai", label: t("plugins:categories.ai") },
    { value: "devices", label: t("plugins:categories.devices") },
    { value: "notify", label: t("plugins:categories.notify") },
  ]

  const tabs = [
    { value: "llm" as PluginTabValue, label: t("plugins:llmBackends") },
    { value: "connections" as PluginTabValue, label: t("plugins:deviceAdapters") },
    { value: "unified" as PluginTabValue, label: t("plugins:allPlugins") },
  ]

  return (
    <PageLayout>
      <PageTabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(v) => setActiveTab(v as PluginTabValue)}
      >
        {/* Unified Plugin View */}
        <PageTabsContent value="unified" activeTab={activeTab}>
          <div className="space-y-4">
            {/* Info banner */}
            <div className="bg-muted/50 border border-border rounded-lg p-4">
              <div className="flex items-start gap-3">
                <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-primary/10">
                  <Package className="h-5 w-5 text-primary" />
                </div>
                <div className="flex-1">
                  <h3 className="font-semibold">{t("plugins:title")}</h3>
                  <p className="text-sm text-muted-foreground mt-1">
                    {t("plugins:noPluginsDesc")}
                  </p>
                </div>
              </div>
            </div>

            {/* Category filter */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Select value={categoryFilter} onValueChange={(v) => setCategoryFilter(v as PluginCategory)}>
                  <SelectTrigger className="w-[180px]">
                    <SelectValue placeholder={t("plugins:filterCategory")} />
                  </SelectTrigger>
                  <SelectContent>
                    {categoryOptions.map((opt) => (
                      <SelectItem key={opt.value} value={opt.value}>
                        {opt.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button variant="outline" size="sm" onClick={handleDiscover} disabled={discovering}>
                  <Radar className="mr-2 h-4 w-4" />
                  {discovering ? t("plugins:discovering") : t("plugins:discover")}
                </Button>
                <Button variant="outline" size="sm" onClick={() => fetchPlugins()}>
                  <RefreshCw className="mr-2 h-4 w-4" />
                  {t("common:refresh")}
                </Button>
              </div>
            </div>

            <PluginGrid
              plugins={filteredPlugins}
              loading={pluginsLoading}
              onToggle={handleToggle}
              onStart={handleStart}
              onStop={handleStop}
              onConfigure={handleConfigure}
              onDelete={handleDelete}
              onRefresh={handleRefresh}
              onViewDevices={handleViewDevices}
            />
          </div>
        </PageTabsContent>

        {/* LLM Backends Tab (legacy) */}
        <PageTabsContent value="llm" activeTab={activeTab}>
          <LLMBackendsTab
            onCreateBackend={createBackend}
            onUpdateBackend={updateBackend}
            onDeleteBackend={deleteBackend}
            onTestBackend={testBackend}
          />
        </PageTabsContent>

        {/* Device Connections Tab (legacy) */}
        <PageTabsContent value="connections" activeTab={activeTab}>
          <ConnectionsTab />
        </PageTabsContent>
      </PageTabs>

      {/* Add Plugin Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t("plugins:registerPlugin")}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <p className="text-sm text-muted-foreground">{t("plugins:registerDesc")}</p>
            <div className="space-y-2">
              <Label htmlFor="plugin-path">{t("plugins:pluginPathLabel")}</Label>
              <Input
                id="plugin-path"
                placeholder={t("plugins:pluginPathPlaceholder")}
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                {t("plugins:pluginPathHint")}
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddDialogOpen(false)}>
              {t("common:cancel")}
            </Button>
            <Button onClick={() => setAddDialogOpen(false)}>
              {t("common:confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Config Dialog */}
      <Dialog open={configDialogOpen} onOpenChange={setConfigDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t("plugins:pluginConfig")}</DialogTitle>
          </DialogHeader>
          <div className="py-4">
            <pre className="bg-muted p-4 rounded text-sm overflow-auto">
              {JSON.stringify(selectedPlugin?.config, null, 2)}
            </pre>
          </div>
          <DialogFooter>
            <Button onClick={() => setConfigDialogOpen(false)}>
              {t("common:close")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageLayout>
  )
}
