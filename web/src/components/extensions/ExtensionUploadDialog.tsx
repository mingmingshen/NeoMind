import { useState, useRef } from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { useToast } from "@/hooks/use-toast"
import { useStore } from "@/store"
import { Upload, Loader2, FolderOpen } from "lucide-react"
import { useTranslation } from "react-i18next"

interface ExtensionUploadDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onUploadComplete?: (extensionId: string) => void
}

export function ExtensionUploadDialog({
  open,
  onOpenChange,
  onUploadComplete,
}: ExtensionUploadDialogProps) {
  const { t } = useTranslation(["extensions", "common"])
  const { toast } = useToast()
  const registerExtension = useStore((state) => state.registerExtension)
  const fetchExtensions = useStore((state) => state.fetchExtensions)

  const [filePath, setFilePath] = useState("")
  const [autoStart, setAutoStart] = useState(false)
  const [uploading, setUploading] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFileSelect = () => {
    // Trigger the hidden file input
    fileInputRef.current?.click()
  }

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      // In Tauri, file has a path property; in web, use the name
      const filePath = (file as any).path || file.name
      setFilePath(filePath)
    }
  }

  const handleSubmit = async () => {
    if (!filePath.trim()) {
      toast({
        title: t("extensionFile"),
        description: t("extensionPathLabel"),
        variant: "destructive",
      })
      return
    }

    setUploading(true)
    try {
      await registerExtension({
        file_path: filePath,
        auto_start: autoStart,
      })

      toast({
        title: t("registerSuccess"),
      })
      onUploadComplete?.(filePath)
      onOpenChange(false)
      // Reset form
      setFilePath("")
      setAutoStart(false)
    } catch (error) {
      toast({
        title: t("registerFailed"),
        description: error instanceof Error ? error.message : "Unknown error",
        variant: "destructive",
      })
    } finally {
      setUploading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Upload className="h-5 w-5" />
            {t("registerExtension")}
          </DialogTitle>
          <DialogDescription>
            {t("registerExtensionDesc")}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 pt-6 pb-4 px-4 sm:px-6">
          {/* File Path Input */}
          <div className="space-y-2">
            <Label htmlFor="file-path">{t("extensionFile")}</Label>
            <div className="flex items-center gap-2">
              <Input
                id="file-path"
                placeholder={t("extensionPathPlaceholder")}
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                disabled={uploading}
                className="font-mono text-sm"
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={handleFileSelect}
                disabled={uploading}
              >
                <FolderOpen className="h-4 w-4" />
              </Button>
              <input
                ref={fileInputRef}
                type="file"
                accept=".so,.dylib,.dll,.wasm"
                className="hidden"
                onChange={handleFileInputChange}
              />
            </div>
            <p className="text-xs text-muted-foreground">
              {t("extensionPathHint")}
            </p>
          </div>

          {/* Auto Start Switch */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="auto-start" className="cursor-pointer">
                {t("autoStart")}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t("autoStartDesc")}
              </p>
            </div>
            <Switch
              id="auto-start"
              checked={autoStart}
              onCheckedChange={setAutoStart}
              disabled={uploading}
            />
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={uploading}
          >
            {t("common:cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={uploading || !filePath.trim()}>
            {uploading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                {t("registering")}
              </>
            ) : (
              <>
                <Upload className="mr-2 h-4 w-4" />
                {t("registerExtension")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
