/**
 * UpdateDialog Component
 *
 * Dialog for displaying available updates and managing
 * the download and installation process.
 * Supports mobile full-screen and desktop modal views.
 */

import { getPortalRoot } from '@/lib/portal'
import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Badge } from '@/components/ui/badge'
import { Check, Download, AlertCircle, Loader2, Rocket, X } from 'lucide-react'
import { useUpdateCheck } from '@/hooks/useUpdateCheck'
import { useAppStore } from '@/store'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'
import { cn } from '@/lib/utils'
import { dialogHeader } from '@/design-system/tokens/size'

export interface UpdateDialogProps {
  /** Whether the dialog is open */
  open: boolean
  /** Called when the dialog is closed */
  onClose: () => void
}

export function UpdateDialog({ open, onClose }: UpdateDialogProps) {
  const { t } = useTranslation(['common', 'settings'])
  const { updateInfo, downloadProgress } = useAppStore()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  // Stable empty callback to prevent unnecessary re-renders
  const handleUpdateAvailable = useCallback(() => {
    // Dialog is already open, no action needed
  }, [])

  const { downloadAndInstall, relaunchApp } = useUpdateCheck({
    onUpdateAvailable: handleUpdateAvailable,
  })

  const [installStatus, setInstallStatus] = useState<'idle' | 'downloading' | 'installing' | 'done' | 'error'>('idle')
  const [installError, setInstallError] = useState<string | null>(null)

  const currentUpdateInfo = updateInfo
  const currentProgress = downloadProgress

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    // Reset state when dialog opens
    if (open) {
      setInstallStatus('idle')
      setInstallError(null)
    }
  }, [open])

  const handleUpdate = async () => {
    try {
      setInstallStatus('downloading')
      setInstallError(null)

      await downloadAndInstall()

      setInstallStatus('done')
    } catch (error) {
      setInstallStatus('error')
      setInstallError(error instanceof Error ? error.message : String(error))
    }
  }

  const handleRelaunch = async () => {
    await relaunchApp()
  }

  const getProgressPercent = () => {
    if (!currentProgress) return 0
    return Math.min(100, Math.max(0, currentProgress.progress))
  }

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
  }

  const getStatusMessage = () => {
    switch (installStatus) {
      case 'downloading':
        return t('settings:downloadingUpdate')
      case 'installing':
        return t('settings:installingUpdate')
      case 'done':
        return t('settings:updateReady')
      case 'error':
        return installError || t('settings:updateFailed')
      default:
        return ''
    }
  }

  const canClose = installStatus === 'idle' || installStatus === 'error' || installStatus === 'done'

  const handleClose = () => {
    if (canClose) {
      onClose()
    }
  }

  const getStatusIcon = () => {
    switch (installStatus) {
      case 'done':
        return <Check className="w-5 h-5" />
      case 'error':
        return <AlertCircle className="w-5 h-5" />
      case 'downloading':
      case 'installing':
        return <Loader2 className="w-5 h-5 animate-spin" />
      default:
        return <Rocket className="w-5 h-5" />
    }
  }

  const getStatusColor = () => {
    switch (installStatus) {
      case 'done':
        return 'bg-success-light dark:bg-success-light text-success dark:text-success'
      case 'error':
        return 'bg-error-light text-error'
      case 'downloading':
      case 'installing':
        return 'bg-info-light text-info'
      default:
        return 'bg-info-light text-info'
    }
  }

  const getTitle = () => {
    switch (installStatus) {
      case 'downloading':
      case 'installing':
        return t('settings:updating')
      case 'done':
        return t('settings:updateReadyTitle')
      case 'error':
        return t('settings:updateFailedTitle')
      default:
        return t('settings:newVersionAvailable')
    }
  }

  const getDescription = () => {
    switch (installStatus) {
      case 'idle':
        return t('settings:updateAvailableDesc')
      case 'downloading':
        return t('settings:downloadingUpdateDesc')
      case 'installing':
        return t('settings:installingUpdateDesc')
      case 'done':
        return t('settings:updateReadyDesc')
      case 'error':
        return getStatusMessage()
      default:
        return ''
    }
  }

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-50 bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className={dialogHeader}
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <div className={cn('flex items-center justify-center w-10 h-10 rounded-full', getStatusColor())}>
                  {getStatusIcon()}
                </div>
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{getTitle()}</h1>
                  {currentUpdateInfo?.version && (
                    <Badge variant="secondary" className="text-xs mt-1">
                      v{updateInfo?.version}
                    </Badge>
                  )}
                </div>
              </div>
              {canClose && (
                <Button variant="ghost" size="icon" onClick={handleClose} className="shrink-0" aria-label={t('common:close')}>
                </Button>
              )}
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <p className="text-sm text-muted-foreground mb-4">{getDescription()}</p>
                <div className="space-y-4">
                  {/* Release Notes */}
                  {currentUpdateInfo?.body && installStatus === 'idle' && (
                    <div className="max-h-60 overflow-y-auto rounded-md border p-3 text-sm">
                      <div className="prose prose-sm dark:prose-invert max-w-none">
                        {updateInfo?.body}
                      </div>
                    </div>
                  )}

                  {/* Progress Bar */}
                  {(installStatus === 'downloading' || installStatus === 'installing') && (
                    <div className="space-y-2">
                      <Progress value={getProgressPercent()} className="h-2" />
                      <div className="flex justify-between text-xs text-muted-foreground">
                        <span>{getStatusMessage()}</span>
                        {currentProgress && (
                          <span>
                            {formatBytes(currentProgress.current)} / {formatBytes(currentProgress.total)} ({Math.round(getProgressPercent())}%)
                          </span>
                        )}
                      </div>
                    </div>
                  )}

                  {/* Success Message */}
                  {installStatus === 'done' && (
                    <div className="flex items-center gap-2 p-3 rounded-md bg-success-light dark:bg-success-light border border-success-light dark:border-success-light">
                      <Check className="w-5 h-5 text-success dark:text-success" />
                      <p className="text-sm text-success">
                        {t('settings:updateCompleteMessage')}
                      </p>
                    </div>
                  )}

                  {/* Error Message */}
                  {installStatus === 'error' && (
                    <div className="flex items-start gap-2 p-3 rounded-md bg-error-light border border-error">
                      <AlertCircle className="w-5 h-5 text-error mt-0.5" />
                      <p className="text-sm text-error">
                        {getStatusMessage()}
                      </p>
                    </div>
                  )}
                </div>
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              {installStatus === 'idle' ? (
                <>
                  <Button variant="ghost" onClick={handleClose} className="text-muted-foreground">
                    {t('settings:remindLater')}
                  </Button>
                  <Button variant="outline" onClick={handleClose}>
                    {t('settings:skipThisUpdate')}
                  </Button>
                  <Button onClick={handleUpdate} className="gap-2">
                    <Download className="w-4 h-4" />
                    {t('settings:updateNow')}
                  </Button>
                </>
              ) : installStatus === 'downloading' ? (
                <Button disabled variant="secondary">
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  {t('settings:downloading')}
                </Button>
              ) : installStatus === 'installing' ? (
                <Button disabled variant="secondary">
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  {t('settings:installing')}
                </Button>
              ) : installStatus === 'done' ? (
                <Button onClick={handleRelaunch} className="gap-2">
                  <Rocket className="w-4 h-4" />
                  {t('settings:relaunchToComplete')}
                </Button>
              ) : installStatus === 'error' ? (
                <>
                  <Button variant="outline" onClick={handleClose}>
                    {t('common:close')}
                  </Button>
                  <Button onClick={handleUpdate} variant="default">
                    {t('common:retry')}
                  </Button>
                </>
              ) : null}
            </div>
          </div>
        </div>
      ) : null, getPortalRoot()
    )
  }

  // Desktop: Traditional dialog
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className={cn(
            'fixed inset-0 z-50 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200',
            !canClose && 'pointer-events-none'
          )}
          onClick={canClose ? handleClose : undefined}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-50',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[calc(100vh-2rem)]',
            'flex flex-col',
            'max-w-md',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <div className={cn('flex items-center justify-center w-10 h-10 rounded-full', getStatusColor())}>
                {getStatusIcon()}
              </div>
              <div className="flex-1 min-w-0">
                <h2 className="text-lg font-semibold leading-none truncate">{getTitle()}</h2>
              </div>
              {currentUpdateInfo?.version && (
                <Badge variant="secondary" className="text-sm">
                  v{updateInfo?.version}
                </Badge>
              )}
            </div>
            {canClose && (
              <button
                onClick={handleClose}
                aria-label={t('common:close')}
                className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>

          {/* Description */}
          <div className="px-6 pt-4">
            <p className="text-sm text-muted-foreground">{getDescription()}</p>
          </div>

          {/* Content */}
          <div className="px-6 py-4">
            <div className="space-y-4">
              {/* Release Notes */}
              {currentUpdateInfo?.body && installStatus === 'idle' && (
                <div className="max-h-60 overflow-y-auto rounded-md border p-3 text-sm">
                  <div className="prose prose-sm dark:prose-invert max-w-none">
                    {updateInfo?.body}
                  </div>
                </div>
              )}

              {/* Progress Bar */}
              {(installStatus === 'downloading' || installStatus === 'installing') && (
                <div className="space-y-2">
                  <Progress value={getProgressPercent()} className="h-2" />
                  <div className="flex justify-between text-xs text-muted-foreground">
                    <span>{getStatusMessage()}</span>
                    {currentProgress && (
                      <span>
                        {formatBytes(currentProgress.current)} / {formatBytes(currentProgress.total)} ({Math.round(getProgressPercent())}%)
                      </span>
                    )}
                  </div>
                </div>
              )}

              {/* Success Message */}
              {installStatus === 'done' && (
                <div className="flex items-center gap-2 p-3 rounded-md bg-success-light dark:bg-success-light border border-success-light dark:border-success-light">
                  <Check className="w-5 h-5 text-success dark:text-success" />
                  <p className="text-sm text-success">
                    {t('settings:updateCompleteMessage')}
                  </p>
                </div>
              )}

              {/* Error Message */}
              {installStatus === 'error' && (
                <div className="flex items-start gap-2 p-3 rounded-md bg-error-light border border-error">
                  <AlertCircle className="w-5 h-5 text-error mt-0.5" />
                  <p className="text-sm text-error">
                    {getStatusMessage()}
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted-30">
            {installStatus === 'idle' ? (
              <>
                <Button variant="ghost" onClick={handleClose} className="text-muted-foreground">
                  {t('settings:remindLater')}
                </Button>
                <Button variant="outline" onClick={handleClose}>
                  {t('settings:skipThisUpdate')}
                </Button>
                <Button onClick={handleUpdate} className="gap-2">
                  <Download className="w-4 h-4" />
                  {t('settings:updateNow')}
                </Button>
              </>
            ) : installStatus === 'downloading' ? (
              <Button disabled variant="secondary">
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                {t('settings:downloading')}
              </Button>
            ) : installStatus === 'installing' ? (
              <Button disabled variant="secondary">
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                {t('settings:installing')}
              </Button>
            ) : installStatus === 'done' ? (
              <Button onClick={handleRelaunch} className="gap-2">
                <Rocket className="w-4 h-4" />
                {t('settings:relaunchToComplete')}
              </Button>
            ) : installStatus === 'error' ? (
              <>
                <Button variant="outline" onClick={handleClose}>
                  {t('common:close')}
                </Button>
                <Button onClick={handleUpdate} variant="default">
                  {t('common:retry')}
                </Button>
              </>
            ) : null}
          </div>
        </div>
      )}
    </>
  )
}

export default UpdateDialog
