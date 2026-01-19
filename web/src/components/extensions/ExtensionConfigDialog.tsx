import { useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Settings, FileCode, Info, Loader2 } from "lucide-react"
import { useStore } from "@/store"
import type { Extension, ExtensionStatsDto } from "@/types"

interface ExtensionConfigDialogProps {
  extension: Extension | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ExtensionConfigDialog({
  extension,
  open,
  onOpenChange,
}: ExtensionConfigDialogProps) {
  const getExtensionStats = useStore((state) => state.getExtensionStats)
  const getExtensionHealth = useStore((state) => state.getExtensionHealth)

  const [stats, setStats] = useState<ExtensionStatsDto | null>(null)
  const [health, setHealth] = useState<{ healthy: boolean } | null>(null)
  const [loading, setLoading] = useState(false)

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
      console.error("Failed to load extension details:", error)
    } finally {
      setLoading(false)
    }
  }

  // Reset state when dialog closes
  const handleClose = (open: boolean) => {
    if (!open) {
      setStats(null)
      setHealth(null)
    }
    onOpenChange(open)
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Extension Configuration</DialogTitle>
          <DialogDescription>
            {extension ? `Configure ${extension.name}` : "Select an extension"}
          </DialogDescription>
        </DialogHeader>

        {!extension ? (
          <div className="py-8 text-center text-muted-foreground">
            No extension selected
          </div>
        ) : (
          <Tabs defaultValue="info" className="w-full" onValueChange={(v) => v === "info" && loadDetails()}>
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="info">
                <Info className="mr-2 h-4 w-4" />
                Info
              </TabsTrigger>
              <TabsTrigger value="stats">
                <Settings className="mr-2 h-4 w-4" />
                Statistics
              </TabsTrigger>
              <TabsTrigger value="file">
                <FileCode className="mr-2 h-4 w-4" />
                File Info
              </TabsTrigger>
            </TabsList>

            {/* Info Tab */}
            <TabsContent value="info" className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label className="text-muted-foreground text-xs">ID</Label>
                  <p className="text-sm font-mono">{extension.id}</p>
                </div>
                <div>
                  <Label className="text-muted-foreground text-xs">Name</Label>
                  <p className="text-sm">{extension.name}</p>
                </div>
                <div>
                  <Label className="text-muted-foreground text-xs">Type</Label>
                  <p className="text-sm">{extension.extension_type}</p>
                </div>
                <div>
                  <Label className="text-muted-foreground text-xs">State</Label>
                  <Badge variant={extension.state === "Running" ? "default" : "secondary"}>
                    {extension.state}
                  </Badge>
                </div>
              </div>

              {extension.description && (
                <div>
                  <Label className="text-muted-foreground text-xs">Description</Label>
                  <p className="text-sm">{extension.description}</p>
                </div>
              )}

              {extension.author && (
                <div>
                  <Label className="text-muted-foreground text-xs">Author</Label>
                  <p className="text-sm">{extension.author}</p>
                </div>
              )}

              {health && (
                <div>
                  <Label className="text-muted-foreground text-xs">Health Status</Label>
                  <Badge variant={health.healthy ? "default" : "destructive"}>
                    {health.healthy ? "Healthy" : "Unhealthy"}
                  </Badge>
                </div>
              )}
            </TabsContent>

            {/* Stats Tab */}
            <TabsContent value="stats" className="space-y-4">
              {loading ? (
                <div className="flex justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              ) : stats ? (
                <div className="grid grid-cols-2 gap-4">
                  <div className="border rounded-lg p-3">
                    <p className="text-xs text-muted-foreground">Start Count</p>
                    <p className="text-2xl font-semibold">{stats.start_count ?? 0}</p>
                  </div>
                  <div className="border rounded-lg p-3">
                    <p className="text-xs text-muted-foreground">Stop Count</p>
                    <p className="text-2xl font-semibold">{stats.stop_count ?? 0}</p>
                  </div>
                  <div className="border rounded-lg p-3">
                    <p className="text-xs text-muted-foreground">Error Count</p>
                    <p className="text-2xl font-semibold">{stats.error_count ?? 0}</p>
                  </div>
                  {stats.last_error && (
                    <div className="border rounded-lg p-3 col-span-2">
                      <p className="text-xs text-muted-foreground">Last Error</p>
                      <p className="text-sm text-destructive">{stats.last_error}</p>
                    </div>
                  )}
                </div>
              ) : (
                <div className="text-center py-8 text-muted-foreground text-sm">
                  Select Info tab to load extension details
                </div>
              )}
            </TabsContent>

            {/* File Info Tab */}
            <TabsContent value="file" className="space-y-4">
              <div>
                <Label className="text-muted-foreground text-xs">File Path</Label>
                <p className="text-sm font-mono break-all">{extension.file_path || "N/A"}</p>
              </div>
              <div>
                <Label className="text-muted-foreground text-xs">Version</Label>
                <p className="text-sm">{extension.version}</p>
              </div>
              {extension.loaded_at && (
                <div>
                  <Label className="text-muted-foreground text-xs">Loaded At</Label>
                  <p className="text-sm">{new Date(extension.loaded_at * 1000).toLocaleString()}</p>
                </div>
              )}
            </TabsContent>
          </Tabs>
        )}

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>Close</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
