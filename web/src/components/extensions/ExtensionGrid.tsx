import { ExtensionCard } from "./ExtensionCard"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import type { Extension } from "@/types"

interface ExtensionGridProps {
  extensions: Extension[]
  loading?: boolean
  onStart?: (id: string) => Promise<boolean>
  onStop?: (id: string) => Promise<boolean>
  onConfigure?: (id: string) => void
  onDelete?: (id: string) => Promise<boolean>
}

export function ExtensionGrid({
  extensions,
  loading = false,
  onStart,
  onStop,
  onConfigure,
  onDelete,
}: ExtensionGridProps) {
  const handleStart = async (id: string) => {
    return await onStart?.(id) ?? false
  }

  const handleStop = async (id: string) => {
    return await onStop?.(id) ?? false
  }

  const handleDelete = async (id: string) => {
    return await onDelete?.(id) ?? false
  }

  if (loading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="border rounded-lg p-4 space-y-4">
            <Skeleton className="h-5 w-3/4" />
            <Skeleton className="h-4 w-1/2" />
            <Skeleton className="h-20 w-full" />
          </div>
        ))}
      </div>
    )
  }

  if (extensions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 px-4 border-2 border-dashed rounded-lg">
        <AlertCircle className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">No Extensions Found</h3>
        <p className="text-sm text-muted-foreground text-center max-w-md">
          No extensions are currently registered. Extensions are dynamically loaded modules (.so/.wasm)
          that extend NeoTalk's capabilities.
        </p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {extensions.map((extension) => (
        <ExtensionCard
          key={extension.id}
          extension={extension}
          onStart={() => handleStart(extension.id)}
          onStop={() => handleStop(extension.id)}
          onConfigure={() => onConfigure?.(extension.id)}
          onDelete={() => handleDelete(extension.id)}
        />
      ))}
    </div>
  )
}
