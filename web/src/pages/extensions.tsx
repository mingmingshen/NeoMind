import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { PageLayout } from "@/components/layout/PageLayout"
import { ExtensionGrid, ExtensionDetailsDialog, DiscoverExtensionsDialog, MarketplaceDialog } from "@/components/extensions"
import { ExtensionUploadDialog } from "@/components/extensions"
import { useToast } from "@/hooks/use-toast"
import { RefreshCw, Upload, Search, Globe } from "lucide-react"
import { Button } from "@/components/ui/button"
import type { Extension } from "@/types"

export function ExtensionsPage() {
  const { t } = useTranslation(["extensions", "common"])
  const { toast } = useToast()

  // Use the main store to access extension state and actions
  const extensions = useStore((state) => state.extensions)
  const extensionsLoading = useStore((state) => state.extensionsLoading)
  const fetchExtensions = useStore((state) => state.fetchExtensions)
  const unregisterExtension = useStore((state) => state.unregisterExtension)

  const [selectedExtension, setSelectedExtension] = useState<Extension | null>(null)
  const [detailsDialogOpen, setDetailsDialogOpen] = useState(false)
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false)
  const [discoverDialogOpen, setDiscoverDialogOpen] = useState(false)
  const [marketplaceDialogOpen, setMarketplaceDialogOpen] = useState(false)

  // Fetch extensions on mount
  useEffect(() => {
    fetchExtensions()
  }, [fetchExtensions])

  // Refresh handler
  const handleRefresh = async () => {
    await fetchExtensions()
    toast({
      title: t("common:refreshed"),
      variant: "default",
    })
  }

  // Extension action handlers
  const handleUnregister = async (id: string): Promise<boolean> => {
    const result = await unregisterExtension(id)
    if (result) {
      toast({
        title: t("extensions:extensionUnregistered"),
      })
    } else {
      toast({
        title: t("extensions:actionFailed"),
        variant: "destructive",
      })
    }
    return result
  }

  const handleConfigure = (id: string) => {
    const ext = extensions.find(e => e.id === id)
    if (ext) {
      setSelectedExtension(ext)
      setDetailsDialogOpen(true)
    }
  }

  const handleUploadComplete = (extensionId: string) => {
    fetchExtensions()
    toast({
      title: t("extensions:extensionUploaded"),
    })
    setSelectedExtension(extensions.find(e => e.id === extensionId) || null)
    setDetailsDialogOpen(true)
  }

  const handleDiscoverDialogChange = (open: boolean) => {
    setDiscoverDialogOpen(open)
    // Refresh extensions when discover dialog closes
    if (!open) {
      fetchExtensions()
    }
  }

  return (
    <>
      <PageLayout
        title={t("extensions:title", { defaultValue: "Extensions" })}
        subtitle={t("extensions:description", { defaultValue: "Manage dynamic extensions and plugins" })}
        borderedHeader={false}
        actions={
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setMarketplaceDialogOpen(true)}
            >
              <Globe className="h-4 w-4 mr-2" />
              {t("extensions:marketplace", { defaultValue: "Marketplace" })}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setDiscoverDialogOpen(true)}
            >
              <Search className="h-4 w-4 mr-2" />
              {t("extensions:discoverExtensions", { defaultValue: "Discover" })}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setUploadDialogOpen(true)}
            >
              <Upload className="h-4 w-4 mr-2" />
              {t("extensions:uploadExtension", { defaultValue: "Upload" })}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={handleRefresh}
              disabled={extensionsLoading}
            >
              <RefreshCw className={`h-4 w-4 ${extensionsLoading ? "animate-spin" : ""}`} />
            </Button>
          </>
        }
      >
        {/* Extensions Grid */}
        <ExtensionGrid
          extensions={extensions}
          loading={extensionsLoading}
          onUnregister={handleUnregister}
          onConfigure={handleConfigure}
        />
      </PageLayout>

      {/* Extension Details Dialog */}
      <ExtensionDetailsDialog
        extension={selectedExtension}
        open={detailsDialogOpen}
        onOpenChange={setDetailsDialogOpen}
      />

      {/* Upload Dialog */}
      <ExtensionUploadDialog
        open={uploadDialogOpen}
        onOpenChange={setUploadDialogOpen}
        onUploadComplete={handleUploadComplete}
      />

      {/* Discover Dialog */}
      <DiscoverExtensionsDialog
        open={discoverDialogOpen}
        onOpenChange={handleDiscoverDialogChange}
      />

      {/* Marketplace Dialog */}
      <MarketplaceDialog
        open={marketplaceDialogOpen}
        onOpenChange={setMarketplaceDialogOpen}
        onInstallComplete={(extensionId) => {
          fetchExtensions()
          const ext = extensions.find(e => e.id === extensionId)
          if (ext) {
            setSelectedExtension(ext)
            setDetailsDialogOpen(true)
          }
        }}
      />
    </>
  )
}
