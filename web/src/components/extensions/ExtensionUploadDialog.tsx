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
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { useToast } from "@/hooks/use-toast"
import { useStore } from "@/store"
import { Upload, FileCode, Loader2, Cpu, Shield, Bell, Wrench, Package } from "lucide-react"
import { useTranslation } from "react-i18next"

interface ExtensionUploadDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onUploadComplete?: (extensionId: string) => void
}

// Extension type info
const EXTENSION_TYPES = [
  {
    type: "llm_provider",
    name: "LLM Provider",
    nameKey: "types.llm_provider",
    description: "Custom LLM backend implementations (OpenAI-compatible, Anthropic, etc.)",
    descriptionKey: "typeDescriptions.llm_provider",
    icon: Cpu,
    color: "text-purple-500",
    bgColor: "bg-purple-100 dark:bg-purple-900/30",
  },
  {
    type: "device_protocol",
    name: "Device Protocol",
    nameKey: "types.device_protocol",
    description: "Device communication protocols (LoRaWAN, Zigbee, etc.)",
    descriptionKey: "typeDescriptions.device_protocol",
    icon: Shield,
    color: "text-blue-500",
    bgColor: "bg-blue-100 dark:bg-blue-900/30",
  },
  {
    type: "alert_channel_type",
    name: "Message Channel",
    nameKey: "types.alert_channel_type",
    description: "Notification channel types (Email, Slack, Discord, etc.)",
    descriptionKey: "typeDescriptions.alert_channel_type",
    icon: Bell,
    color: "text-orange-500",
    bgColor: "bg-orange-100 dark:bg-orange-900/30",
  },
  {
    type: "tool",
    name: "Tool",
    nameKey: "types.tool",
    description: "AI function calling tools",
    descriptionKey: "typeDescriptions.tool",
    icon: Wrench,
    color: "text-green-500",
    bgColor: "bg-green-100 dark:bg-green-900/30",
  },
  {
    type: "generic",
    name: "Generic",
    nameKey: "types.generic",
    description: "General-purpose extensions",
    descriptionKey: "typeDescriptions.generic",
    icon: Package,
    color: "text-gray-500",
    bgColor: "bg-gray-100 dark:bg-gray-900/30",
  },
]

export function ExtensionUploadDialog({
  open,
  onOpenChange,
  onUploadComplete,
}: ExtensionUploadDialogProps) {
  const { t } = useTranslation(["extensions", "common"])
  const { toast } = useToast()
  const registerExtension = useStore((state) => state.registerExtension)

  const [filePath, setFilePath] = useState("")
  const [autoStart, setAutoStart] = useState(false)
  const [uploading, setUploading] = useState(false)

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
      const success = await registerExtension({
        file_path: filePath,
        auto_start: autoStart,
      })

      if (success) {
        toast({
          title: t("registerSuccess"),
        })
        onUploadComplete?.(filePath)
        onOpenChange(false)
        // Reset form
        setFilePath("")
        setAutoStart(false)
      } else {
        toast({
          title: t("registerFailed"),
          variant: "destructive",
        })
      }
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
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Upload className="h-5 w-5" />
            {t("registerExtension")}
          </DialogTitle>
          <DialogDescription>
            {t("registerDesc")}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* File Path Input */}
          <div className="space-y-2">
            <Label htmlFor="file-path">{t("extensionFile")}</Label>
            <div className="flex items-center gap-2">
              <FileCode className="h-4 w-4 text-muted-foreground" />
              <Input
                id="file-path"
                placeholder={t("extensionPathPlaceholder")}
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                disabled={uploading}
                className="font-mono text-sm"
              />
            </div>
            <p className="text-xs text-muted-foreground">
              {t("extensionPathHint")}
            </p>
          </div>

          {/* Auto Start Switch */}
          <div className="flex items-center justify-between p-3 border rounded-lg">
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

          {/* Extension Types Info */}
          <div className="space-y-3">
            <Label className="text-base">{t("supportedTypes")}</Label>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              {EXTENSION_TYPES.map((extType) => {
                const Icon = extType.icon
                return (
                  <div
                    key={extType.type}
                    className={`flex items-start gap-3 p-3 rounded-lg border ${extType.bgColor}`}
                  >
                    <div className={`p-2 rounded ${extType.bgColor}`}>
                      <Icon className={`h-4 w-4 ${extType.color}`} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium text-sm">{t(extType.nameKey)}</p>
                      <p className="text-xs text-muted-foreground">
                        {t(extType.descriptionKey)}
                      </p>
                    </div>
                  </div>
                )
              })}
            </div>
          </div>

          {/* Info Box */}
          <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
            <p className="text-sm text-blue-800 dark:text-blue-300">
              <strong>Note:</strong> Extension files must be compatible with your system architecture.
              Make sure to only load extensions from trusted sources.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={uploading}
          >
            {t("cancel", { ns: "common" })}
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
