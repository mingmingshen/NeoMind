import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { ExtensionCard } from "./ExtensionCard"
import { Skeleton } from "@/components/ui/skeleton"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  AlertCircle,
  Package,
  Search,
  Cpu,
  Shield,
  Wrench,
  Bell,
  FileCode,
  X,
} from "lucide-react"
import type { Extension } from "@/types"

interface ExtensionGridProps {
  extensions: Extension[]
  loading?: boolean
  onStart?: (id: string) => Promise<boolean>
  onStop?: (id: string) => Promise<boolean>
  onConfigure?: (id: string) => void
  onDelete?: (id: string) => Promise<boolean>
}

// Extension type icons
const EXTENSION_ICONS: Record<string, React.ElementType> = {
  llm_provider: Cpu,
  device_protocol: Shield,
  alert_channel_type: Bell,
  tool: Wrench,
  generic: FileCode,
}

// Extension type colors
const EXTENSION_COLORS: Record<string, string> = {
  llm_provider: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400",
  device_protocol: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
  alert_channel_type: "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400",
  tool: "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
  generic: "bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400",
}

export function ExtensionGrid({
  extensions,
  loading = false,
  onStart,
  onStop,
  onConfigure,
  onDelete,
}: ExtensionGridProps) {
  const { t } = useTranslation(["extensions", "common"])

  // Filter states
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")
  const [statusFilter, setStatusFilter] = useState<string>("all")

  // Handle actions
  const handleStart = async (id: string) => {
    return await onStart?.(id) ?? false
  }

  const handleStop = async (id: string) => {
    return await onStop?.(id) ?? false
  }

  const handleDelete = async (id: string) => {
    return await onDelete?.(id) ?? false
  }

  // Clear filters
  const clearFilters = () => {
    setSearchQuery("")
    setTypeFilter("all")
    setStatusFilter("all")
  }

  // Filter and group extensions
  const { filteredExtensions, extensionsByType, typeStats, hasActiveFilters } = useMemo(() => {
    let filtered = extensions

    // Search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase()
      filtered = filtered.filter(
        (ext) =>
          ext.name.toLowerCase().includes(query) ||
          ext.id.toLowerCase().includes(query) ||
          ext.description?.toLowerCase().includes(query) ||
          ext.extension_type.toLowerCase().includes(query)
      )
    }

    // Type filter
    if (typeFilter !== "all") {
      filtered = filtered.filter((ext) => ext.extension_type === typeFilter)
    }

    // Status filter
    if (statusFilter !== "all") {
      filtered = filtered.filter((ext) => {
        switch (statusFilter) {
          case "running":
            return ext.state === "Running"
          case "stopped":
            return ext.state === "Stopped" || ext.state === "Initialized"
          case "error":
            return ext.state === "Error"
          default:
            return true
        }
      })
    }

    // Group by type
    const grouped: Record<string, Extension[]> = {}
    const stats: Record<string, { total: number; running: number }> = {}

    // Get all unique types from filtered extensions
    const uniqueTypes = Array.from(new Set(filtered.map((ext) => ext.extension_type)))

    for (const type of uniqueTypes) {
      const typeExtensions = filtered.filter((ext) => ext.extension_type === type)
      grouped[type] = typeExtensions
      stats[type] = {
        total: typeExtensions.length,
        running: typeExtensions.filter((ext) => ext.state === "Running").length,
      }
    }

    // Sort types by name (llm_provider first, then alphabetically)
    const typeOrder = ["llm_provider", "device_protocol", "alert_channel_type", "tool", "generic"]
    const sortedTypes = uniqueTypes.sort((a, b) => {
      const aIndex = typeOrder.indexOf(a)
      const bIndex = typeOrder.indexOf(b)
      if (aIndex !== -1 && bIndex !== -1) return aIndex - bIndex
      if (aIndex !== -1) return -1
      if (bIndex !== -1) return 1
      return a.localeCompare(b)
    })

    return {
      filteredExtensions: filtered,
      extensionsByType: Object.fromEntries(sortedTypes.map((type) => [type, grouped[type] || []])),
      typeStats: stats,
      hasActiveFilters: searchQuery || typeFilter !== "all" || statusFilter !== "all",
    }
  }, [extensions, searchQuery, typeFilter, statusFilter])

  // Get available types from all extensions
  const availableTypes = useMemo(() => {
    const types = new Set(extensions.map((ext) => ext.extension_type))
    return Array.from(types)
  }, [extensions])

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
        <Package className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">{t("noExtensions")}</h3>
        <p className="text-sm text-muted-foreground text-center max-w-md">
          {t("noExtensionsDesc")}
        </p>
      </div>
    )
  }

  // Empty state for filtered results
  if (filteredExtensions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 px-4">
        <Search className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">{t("noMatchingExtensions")}</h3>
        <Button onClick={clearFilters} variant="outline" className="mt-4">
          {t("clearFilters", { ns: "common" })}
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Search and Filter Bar */}
      <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
        <div className="flex flex-1 gap-2 w-full sm:w-auto">
          {/* Search */}
          <div className="relative flex-1 max-w-sm">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder={t("searchPlaceholder")}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>

          {/* Type Filter */}
          <Select value={typeFilter} onValueChange={setTypeFilter}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder={t("filterByType")} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t("allTypes")}</SelectItem>
              {availableTypes.map((type) => (
                <SelectItem key={type} value={type}>
                  {t(`types.${type}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Status Filter */}
          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder={t("filterByStatus")} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t("allStatuses")}</SelectItem>
              <SelectItem value="running">{t("categories.running")}</SelectItem>
              <SelectItem value="stopped">{t("categories.stopped")}</SelectItem>
              <SelectItem value="error">{t("categories.error")}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Clear Filters Button */}
        {hasActiveFilters && (
          <Button variant="ghost" size="sm" onClick={clearFilters} className="gap-1">
            <X className="h-4 w-4" />
            {t("clearFilters", { ns: "common" })}
          </Button>
        )}

        {/* Result Count */}
        {!hasActiveFilters && (
          <div className="text-sm text-muted-foreground">
            {t("totalCount", { count: extensions.length })}
          </div>
        )}
      </div>

      {/* Extension Groups by Type */}
      <div className="space-y-6">
        {Object.entries(extensionsByType).map(([type, typeExtensions]) => {
          if (typeExtensions.length === 0) return null

          const stats = typeStats[type]
          const Icon = EXTENSION_ICONS[type] || FileCode
          const colorClass = EXTENSION_COLORS[type] || EXTENSION_COLORS.generic

          return (
            <div key={type} className="space-y-4">
              {/* Type Header */}
              <div className="flex items-center gap-3">
                <div className={`flex items-center justify-center w-10 h-10 rounded-lg ${colorClass}`}>
                  <Icon className="h-5 w-5" />
                </div>
                <div className="flex-1">
                  <h3 className="text-lg font-semibold">{t(`types.${type}`)}</h3>
                  <p className="text-sm text-muted-foreground">
                    {t(`typeDescriptions.${type}`)}
                  </p>
                </div>
                <Badge className={colorClass} variant="secondary">
                  {stats.running}/{stats.total} {t("running")}
                </Badge>
              </div>

              {/* Extension Cards */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {typeExtensions.map((extension) => (
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
            </div>
          )
        })}
      </div>
    </div>
  )
}
