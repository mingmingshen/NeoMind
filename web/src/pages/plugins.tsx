import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { useNavigate, useLocation } from "react-router-dom"
import { useStore } from "@/store"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { ExtensionGrid, ExtensionConfigDialog } from "@/components/extensions"
import { UnifiedLLMBackendsTab } from "@/components/llm/UnifiedLLMBackendsTab"
import { UnifiedDeviceConnectionsTab } from "@/components/connections"
import { UnifiedAlertChannelsTab } from "@/components/alerts/UnifiedAlertChannelsTab"
import { useToast } from "@/hooks/use-toast"
import { RefreshCw, Plus, Cpu, Plug, BellRing, Puzzle } from "lucide-react"
import { ExtensionUploadDialog } from "@/components/extensions"

type TabValue = "llm" | "connections" | "alert-channels" | "extensions"

export function PluginsPage() {
  const { t } = useTranslation(["extensions", "common"])
  const { toast } = useToast()

  // Router integration
  const navigate = useNavigate()
  const location = useLocation()

  // Get tab from URL path
  const getTabFromPath = (): TabValue => {
    const pathSegments = location.pathname.split('/')
    const lastSegment = pathSegments[pathSegments.length - 1]
    if (lastSegment === 'connections' || lastSegment === 'alert-channels' || lastSegment === 'extensions') {
      return lastSegment as TabValue
    }
    return 'llm'
  }

  const {
    extensions,
    extensionsLoading,
    fetchExtensions,
    // Extension actions
    startExtension,
    stopExtension,
    unregisterExtension,
    selectedExtension,
    setSelectedExtension,
    extensionDialogOpen,
    setExtensionDialogOpen,
    discoverExtensions,
    // LLM Backend actions
    createBackend,
    updateBackend,
    deleteBackend,
    testBackend,
  } = useStore()

  // Active tab state - sync with URL
  const [activeTab, setActiveTab] = useState<TabValue>(getTabFromPath)

  // Update tab when URL changes
  useEffect(() => {
    const tabFromPath = getTabFromPath()
    setActiveTab(tabFromPath)
  }, [location.pathname])

  // Update URL when tab changes
  const handleTabChange = (tab: string) => {
    const validTab = tab as TabValue
    setActiveTab(validTab)
    if (validTab === 'llm') {
      navigate('/plugins')
    } else {
      navigate(`/plugins/${validTab}`)
    }
  }

  const [uploadDialogOpen, setUploadDialogOpen] = useState(false)

  // Fetch extensions on mount and when extensions tab is activated
  useEffect(() => {
    if (activeTab === "extensions") {
      fetchExtensions()
    }
  }, [fetchExtensions, activeTab])

  const tabs = [
    { value: "llm" as TabValue, label: t("llmBackends"), icon: <Cpu className="h-4 w-4" /> },
    { value: "connections" as TabValue, label: t("deviceConnections"), icon: <Plug className="h-4 w-4" /> },
    { value: "alert-channels" as TabValue, label: t("alertChannels"), icon: <BellRing className="h-4 w-4" /> },
    { value: "extensions" as TabValue, label: t("extensionPlugins"), icon: <Puzzle className="h-4 w-4" /> },
  ]

  // Extension action handlers
  const handleStart = async (id: string) => {
    const result = await startExtension(id)
    if (result) {
      toast({
        title: t("extensionStarted"),
      })
    } else {
      toast({
        title: t("actionFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleStop = async (id: string) => {
    const result = await stopExtension(id)
    if (result) {
      toast({
        title: t("extensionStopped"),
      })
    } else {
      toast({
        title: t("actionFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleConfigure = (id: string) => {
    const extension = extensions.find((e) => e.id === id)
    if (extension) {
      setSelectedExtension(extension)
      setExtensionDialogOpen(true)
    }
  }

  const handleDelete = async (id: string) => {
    const result = await unregisterExtension(id)
    if (result) {
      toast({
        title: t("unregisterSuccess"),
      })
    } else {
      toast({
        title: t("unregisterFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleRefresh = async () => {
    await fetchExtensions()
    toast({
      title: t("refreshed"),
    })
    return true
  }

  const handleDiscover = async () => {
    const result = await discoverExtensions()
    toast({
      title: t("discovered", { count: result.discovered }),
    })
  }

  return (
    <PageLayout
      title={t('title')}
      subtitle={t('description')}
    >
      <PageTabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={handleTabChange}
        actions={
          activeTab === 'extensions'
            ? [
                {
                  label: t('discover'),
                  icon: <RefreshCw className="h-4 w-4" />,
                  variant: 'outline' as const,
                  onClick: handleDiscover,
                },
                {
                  label: t('refresh'),
                  icon: <RefreshCw className="h-4 w-4" />,
                  variant: 'outline' as const,
                  onClick: handleRefresh,
                },
                {
                  label: t('add'),
                  icon: <Plus className="h-4 w-4" />,
                  variant: 'default' as const,
                  onClick: () => setUploadDialogOpen(true),
                },
              ]
            : []
        }
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

        {/* Extension Plugins Tab */}
        <PageTabsContent value="extensions" activeTab={activeTab}>
          <ExtensionGrid
            extensions={extensions}
            loading={extensionsLoading}
            onStart={handleStart}
            onStop={handleStop}
            onConfigure={handleConfigure}
            onDelete={handleDelete}
          />
        </PageTabsContent>
      </PageTabs>

      {/* Upload Extension Dialog */}
      <ExtensionUploadDialog
        open={uploadDialogOpen}
        onOpenChange={(open) => {
          setUploadDialogOpen(open)
          if (!open) {
            fetchExtensions()
          }
        }}
        onUploadComplete={(extensionId) => {
          toast({
            title: t("registerSuccess"),
          })
          fetchExtensions()
        }}
      />

      {/* Extension Config Dialog */}
      <ExtensionConfigDialog
        extension={selectedExtension}
        open={extensionDialogOpen}
        onOpenChange={setExtensionDialogOpen}
      />
    </PageLayout>
  )
}
