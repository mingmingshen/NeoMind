/**
 * Component Marketplace Dialog
 *
 * Full-screen dialog for browsing and installing community components.
 */

import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import * as lucideReact from 'lucide-react'
import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
  FullScreenDialogMain,
} from '@/components/automation/dialog/FullScreenDialog'
import { Button } from '@/components/ui/button'
import { useStore } from '@/store'
import { notifySuccess, notifyError, notifyFromError } from '@/lib/notify'
import type { MarketComponentEntry } from '@/types/frontend-component'

interface ComponentMarketplaceProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Helper to get localized text from string or Record<string, string>
 */
function getLocalizedText(value: string | Record<string, string>, locale: string): string {
  if (typeof value === 'string') return value
  return value[locale] || value.en || Object.values(value)[0] || ''
}

/**
 * Component card for marketplace
 */
interface ComponentCardProps {
  component: MarketComponentEntry
  isInstalled: boolean
  locale: string
  onInstall: (id: string) => void
  onUninstall: (id: string) => void
  t: (key: string) => string
}

function ComponentCard({ component, isInstalled, locale, onInstall, onUninstall, t }: ComponentCardProps) {
  const [loading, setLoading] = useState(false)

  // Get icon component
  const iconName = component.icon || 'Box'
  const lucideRecord: any = lucideReact
  const IconComponent = lucideRecord[iconName] || lucideRecord.Box

  // Get localized name and description
  const name = getLocalizedText(component.name, locale)
  const description = getLocalizedText(component.description, locale)

  const handleInstall = async () => {
    if (loading || isInstalled) return
    setLoading(true)
    try {
      await onInstall(component.id)
      notifySuccess(t('installSuccess'))
    } catch (error) {
      notifyFromError(error, t('installError'))
    } finally {
      setLoading(false)
    }
  }

  const handleUninstall = async () => {
    if (loading) return
    setLoading(true)
    try {
      await onUninstall(component.id)
      notifySuccess(t('uninstallSuccess'))
    } catch (error) {
      notifyFromError(error, t('installError'))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="bg-card border border-border rounded-lg p-4 space-y-3">
      {/* Icon + Name */}
      <div className="flex items-start gap-3">
        <div className="w-10 h-10 rounded-lg bg-muted flex items-center justify-center flex-shrink-0">
          <IconComponent className="w-5 h-5 text-primary" />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-semibold text-foreground truncate">{name}</h3>
          {component.author && (
            <p className="text-xs text-muted-foreground">
              {t('by')} {component.author}
            </p>
          )}
        </div>
        {isInstalled && (
          <div className="flex-shrink-0">
            <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full bg-success-light text-success text-xs font-medium">
              <lucideReact.CheckCircle2 className="w-3 h-3" />
              {t('installed')}
            </span>
          </div>
        )}
      </div>

      {/* Description */}
      <p className="text-xs text-muted-foreground line-clamp-2">{description}</p>

      {/* Version + Actions */}
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-muted-foreground">
          {t('version')}: {component.version}
        </span>
        {isInstalled ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleUninstall}
            disabled={loading}
            className="h-7 px-2 text-muted-foreground hover:text-destructive"
          >
            {loading ? (
              <lucideReact.Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <lucideReact.Trash2 className="w-4 h-4" />
            )}
            <span className="ml-1">{t('uninstall')}</span>
          </Button>
        ) : (
          <Button
            variant="outline"
            size="sm"
            onClick={handleInstall}
            disabled={loading}
            className="h-7 px-3"
          >
            {loading ? (
              <lucideReact.Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <lucideReact.Download className="w-4 h-4" />
            )}
            <span className="ml-1">{t('install')}</span>
          </Button>
        )}
      </div>
    </div>
  )
}

/**
 * Skeleton card for loading state
 */
function SkeletonCard() {
  return (
    <div className="bg-card border border-border rounded-lg p-4 space-y-3">
      <div className="flex items-start gap-3">
        <div className="w-10 h-10 rounded-lg bg-muted animate-pulse" />
        <div className="flex-1 min-w-0 space-y-2">
          <div className="h-4 bg-muted rounded w-3/4 animate-pulse" />
          <div className="h-3 bg-muted rounded w-1/2 animate-pulse" />
        </div>
      </div>
      <div className="space-y-2">
        <div className="h-3 bg-muted rounded w-full animate-pulse" />
        <div className="h-3 bg-muted rounded w-2/3 animate-pulse" />
      </div>
      <div className="h-7 bg-muted rounded w-20 animate-pulse" />
    </div>
  )
}

/**
 * Main Component Marketplace Dialog
 */
export function ComponentMarketplace({ open, onOpenChange }: ComponentMarketplaceProps) {
  const { t, i18n } = useTranslation('dashboardComponents')
  const locale = i18n.language

  const {
    marketComponents,
    marketLoading,
    installed,
    error,
    fetchMarket,
    installFromMarket,
    uninstall,
  } = useStore()

  // Fetch marketplace on open
  useEffect(() => {
    if (open) {
      fetchMarket()
    }
  }, [open, fetchMarket])

  const installedIds = new Set(installed.map((c) => c.id))

  // Render loading state
  if (marketLoading) {
    return (
      <FullScreenDialog open={open} onOpenChange={onOpenChange}>
        <FullScreenDialogHeader
          icon={<lucideReact.Store className="w-full h-full" />}
          title={t('componentLibrary.marketplaceTitle')}
          onClose={() => onOpenChange(false)}
        />
        <FullScreenDialogContent>
          <FullScreenDialogMain className="p-4 md:p-6">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {Array.from({ length: 6 }).map((_, i) => (
                <SkeletonCard key={i} />
              ))}
            </div>
          </FullScreenDialogMain>
        </FullScreenDialogContent>
      </FullScreenDialog>
    )
  }

  // Render error state
  if (error) {
    return (
      <FullScreenDialog open={open} onOpenChange={onOpenChange}>
        <FullScreenDialogHeader
          icon={<lucideReact.Store className="w-full h-full" />}
          title={t('componentLibrary.marketplaceTitle')}
          onClose={() => onOpenChange(false)}
        />
        <FullScreenDialogContent>
          <FullScreenDialogMain className="p-4 md:p-6">
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <lucideReact.AlertCircle className="w-12 h-12 text-error mb-4" />
              <h3 className="text-lg font-semibold text-foreground mb-2">
                {t('componentLibrary.marketplaceError')}
              </h3>
              <p className="text-sm text-muted-foreground">{error}</p>
            </div>
          </FullScreenDialogMain>
        </FullScreenDialogContent>
      </FullScreenDialog>
    )
  }

  // Render empty state
  if (marketComponents.length === 0) {
    return (
      <FullScreenDialog open={open} onOpenChange={onOpenChange}>
        <FullScreenDialogHeader
          icon={<lucideReact.Store className="w-full h-full" />}
          title={t('componentLibrary.marketplaceTitle')}
          onClose={() => onOpenChange(false)}
        />
        <FullScreenDialogContent>
          <FullScreenDialogMain className="p-4 md:p-6">
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <lucideReact.Package className="w-12 h-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-semibold text-foreground mb-2">
                {t('componentLibrary.marketplaceEmpty')}
              </h3>
            </div>
          </FullScreenDialogMain>
        </FullScreenDialogContent>
      </FullScreenDialog>
    )
  }

  // Render component grid
  return (
    <FullScreenDialog open={open} onOpenChange={onOpenChange}>
      <FullScreenDialogHeader
        icon={<lucideReact.Store className="w-full h-full" />}
        title={t('componentLibrary.marketplaceTitle')}
        subtitle={`${marketComponents.length} ${t('componentLibrary.components')}`}
        onClose={() => onOpenChange(false)}
      />
      <FullScreenDialogContent>
        <FullScreenDialogMain className="p-4 md:p-6">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {marketComponents.map((component) => (
              <ComponentCard
                key={component.id}
                component={component}
                isInstalled={installedIds.has(component.id)}
                locale={locale}
                onInstall={installFromMarket}
                onUninstall={uninstall}
                t={t}
              />
            ))}
          </div>
        </FullScreenDialogMain>
      </FullScreenDialogContent>
    </FullScreenDialog>
  )
}
