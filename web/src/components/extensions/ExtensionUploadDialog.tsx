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
import { Upload, FileCode, Loader2 } from "lucide-react"

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
  const { toast } = useToast()
  const registerExtension = useStore((state) => state.registerExtension)

  const [filePath, setFilePath] = useState("")
  const [autoStart, setAutoStart] = useState(false)
  const [uploading, setUploading] = useState(false)

  const handleSubmit = async () => {
    if (!filePath.trim()) {
      toast({
        title: "File path is required",
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
          title: "Extension registered successfully",
        })
        onUploadComplete?.(filePath)
        onOpenChange(false)
        // Reset form
        setFilePath("")
        setAutoStart(false)
      } else {
        toast({
          title: "Failed to register extension",
          variant: "destructive",
        })
      }
    } catch (error) {
      toast({
        title: "Failed to register extension",
        description: error instanceof Error ? error.message : "Unknown error",
        variant: "destructive",
      })
    } finally {
      setUploading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Register Extension</DialogTitle>
          <DialogDescription>
            Enter the path to the extension file (.so/.wasm) to register it with NeoMind.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* File Path Input */}
          <div className="space-y-2">
            <Label htmlFor="file-path">Extension File Path</Label>
            <div className="flex items-center gap-2">
              <FileCode className="h-4 w-4 text-muted-foreground" />
              <Input
                id="file-path"
                placeholder="/path/to/extension.so"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                disabled={uploading}
              />
            </div>
            <p className="text-xs text-muted-foreground">
              Path to the extension file on the server
            </p>
          </div>

          {/* Auto Start Switch */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="auto-start">Auto Start</Label>
              <p className="text-xs text-muted-foreground">
                Automatically start the extension after registration
              </p>
            </div>
            <Switch
              id="auto-start"
              checked={autoStart}
              onCheckedChange={setAutoStart}
              disabled={uploading}
            />
          </div>

          {/* Info Box */}
          <div className="bg-muted rounded-lg p-3 text-sm">
            <p className="font-medium mb-1">Supported Extension Types:</p>
            <ul className="text-xs text-muted-foreground space-y-1">
              <li>• <strong>llm_provider</strong> - Custom LLM backend implementations</li>
              <li>• <strong>device_protocol</strong> - Device communication protocols</li>
              <li>• <strong>alert_channel_type</strong> - Notification channel types</li>
              <li>• <strong>tool</strong> - AI function calling tools</li>
              <li>• <strong>generic</strong> - General-purpose extensions</li>
            </ul>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={uploading}
          >
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={uploading || !filePath.trim()}>
            {uploading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Registering...
              </>
            ) : (
              <>
                <Upload className="mr-2 h-4 w-4" />
                Register Extension
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
