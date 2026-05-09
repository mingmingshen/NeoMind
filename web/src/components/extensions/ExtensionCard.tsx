import { Card, CardContent } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  MoreHorizontal,
  Trash,
  Code2,
  Terminal,
  Database,
  RefreshCw,
  Settings,
} from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import { textNano, textMini } from "@/design-system/tokens/typography"
import type { Extension } from "@/types"

interface ExtensionCardProps {
  extension: Extension
  onUninstall?: () => void
  onDetails?: () => void
  onReload?: () => void
}

export function ExtensionCard({
  extension,
  onUninstall,
  onDetails,
  onReload,
}: ExtensionCardProps) {
  const { t } = useTranslation(["extensions"])

  const isActive = extension.state !== "Error"

  return (
    <Card className="group border-border hover:border-border transition-all duration-200 overflow-hidden bg-card backdrop-blur-sm">
      <CardContent className="p-4">
        {/* Header */}
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-center gap-3">
            <div className={cn(
              "relative p-2 rounded-lg transition-all",
              isActive ? "bg-primary text-primary-foreground" : "bg-muted",
            )}>
              <Code2 className="h-4 w-4" />
              {/* Status indicator dot */}
              <div className={cn(
                "absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-background",
                isActive ? "bg-green-500" : "bg-red-500"
              )} />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="font-medium text-sm">{extension.name}</h3>
                <span className={cn(
                  textNano, "px-1.5 py-0.5 rounded-full",
                  isActive ? "bg-green-500/10 text-green-600 dark:text-green-400" :
                  "bg-red-500/10 text-red-600 dark:text-red-400"
                )}>
                  {isActive ? t('active', { defaultValue: 'Active' }) : extension.state}
                </span>
              </div>
              <div className="flex items-center gap-2 mt-0.5">
                <span className={cn(textMini, "text-muted-foreground font-mono truncate max-w-[100px]")}>{extension.id}</span>
                {extension.version && (
                  <span className={cn(textNano, "text-muted-foreground")}>v{extension.version}</span>
                )}
              </div>
            </div>
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-40">
              <DropdownMenuItem onClick={() => onDetails?.()}>
                <Settings className="mr-2 h-4 w-4" />
                {t('card.details', { defaultValue: 'Details' })}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => onReload?.()}>
                <RefreshCw className="mr-2 h-4 w-4" />
                {t('card.reload', { defaultValue: 'Reload' })}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => onUninstall?.()} className="text-destructive">
                <Trash className="mr-2 h-4 w-4" />
                {t('card.uninstall', { defaultValue: "Uninstall" })}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {/* Capabilities Badge */}
        <div className="flex items-center gap-2 mb-3">
          {extension.commands?.length > 0 && (
            <span className={cn(textMini, "text-muted-foreground flex items-center gap-1")}>
              <Terminal className="h-3 w-3" />
              {extension.commands.length} {t('commands', { defaultValue: 'commands' })}
            </span>
          )}
          {extension.metrics?.length > 0 && (
            <span className={cn(textMini, "text-muted-foreground flex items-center gap-1")}>
              <Database className="h-3 w-3" />
              {extension.metrics.length} {t('metrics', { defaultValue: 'metrics' })}
            </span>
          )}
        </div>

        {/* Description */}
        {extension.description && (
          <p className="text-xs text-muted-foreground line-clamp-2 h-8 mb-3 leading-4">
            {extension.description}
          </p>
        )}

        {/* Author */}
        {extension.author && (
          <p className={cn(textNano, "text-muted-foreground mb-3")}>
            {t('card.byAuthor', { author: extension.author })}
          </p>
        )}

        {/* Action Bar */}
        <div className="flex items-center justify-between pt-2 border-t border-border">
          <span className={cn(textNano, "text-muted-foreground")}>
            {isActive ? t('card.active', { defaultValue: 'Active' }) : extension.state}
          </span>
          <Button
            size="sm"
            variant="ghost"
            className={cn("h-6", textNano, "px-2")}
            onClick={() => onDetails?.()}
          >
            <Settings className="mr-1 h-2.5 w-2.5" />
            {t('card.details', { defaultValue: 'Details' })}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
