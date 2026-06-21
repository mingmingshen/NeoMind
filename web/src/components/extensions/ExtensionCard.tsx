import { Card, CardContent } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
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

  const hasError = extension.state === "Error"
  const hasWarning = extension.state === "Warning"
  const isFailed = extension.state === "Failed" || extension.state === "Stopped"
  const isHealthy = !hasError && !hasWarning && !isFailed

  const displayState = hasError
    ? t('error', { defaultValue: 'Error' })
    : hasWarning
      ? t('warning', { defaultValue: 'Warning' })
      : isFailed
        ? extension.state
        : t('active', { defaultValue: 'Active' })

  return (
    <Card
      role="button"
      tabIndex={0}
      onClick={() => onDetails?.()}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onDetails?.() } }}
      className="group border-border hover:border-primary hover:shadow-md hover:-translate-y-0.5 transition-all duration-200 overflow-hidden bg-card backdrop-blur-sm flex flex-col cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
    >
      <CardContent className="p-4 flex flex-col flex-1">
        {/* Header */}
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-center gap-3 min-w-0">
            <div className={cn(
              "relative p-2 rounded-lg transition-all shrink-0",
              isHealthy ? "bg-primary text-primary-foreground" : "bg-muted",
            )}>
              <Code2 className="h-4 w-4" />
              {/* Status indicator dot */}
              <div className={cn(
                "absolute -top-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-background",
                hasError ? "bg-error" : hasWarning ? "bg-warning" : isFailed ? "bg-muted-foreground" : "bg-success"
              )} />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h3 className="font-medium text-sm truncate">{extension.name}</h3>
                <span className={cn(
                  textNano, "px-1.5 py-0.5 rounded-full shrink-0",
                  hasError ? "bg-error-light text-error" :
                  hasWarning ? "bg-warning-light text-warning" :
                  isFailed ? "bg-muted-30 text-muted-foreground" :
                  "bg-success-light text-success"
                )}>
                  {displayState}
                </span>
              </div>
              <div className="flex items-center gap-2 mt-0.5">
                <span className={cn(textMini, "text-muted-foreground font-mono truncate max-w-[100px]")}>{extension.id}</span>
                {extension.version && (
                  <span className={cn(textNano, "text-muted-foreground shrink-0")}>v{extension.version}</span>
                )}
              </div>
            </div>
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
              <Button variant="ghost" size="icon-sm" className="shrink-0" aria-label={t('card.moreActions', { defaultValue: 'More actions' })}>
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-40" onClick={(e) => e.stopPropagation()}>
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
              {extension.commands.length} {t('commandsLabel', { defaultValue: 'commands' })}
            </span>
          )}
          {extension.metrics?.length > 0 && (
            <span className={cn(textMini, "text-muted-foreground flex items-center gap-1")}>
              <Database className="h-3 w-3" />
              {extension.metrics.length} {t('metricsLabel', { defaultValue: 'metrics' })}
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
          <p className={cn(textNano, "text-muted-foreground mt-auto pt-2")}>
            {t('card.byAuthor', { author: extension.author })}
          </p>
        )}
      </CardContent>
    </Card>
  )
}
