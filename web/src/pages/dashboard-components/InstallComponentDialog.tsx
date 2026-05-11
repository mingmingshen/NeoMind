/**
 * Install Component Dialog
 *
 * Dialog for manually importing a community component
 * from manifest.json and bundle.js files.
 */

import { useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import * as lucideReact from 'lucide-react'
import { UnifiedFormDialog } from '@/components/dialog/UnifiedFormDialog'
import { useStore } from '@/store'
import { notifySuccess, notifyFromError } from '@/lib/notify'
import type { ComponentManifest } from '@/types/frontend-component'

interface InstallComponentDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * File drop zone component
 */
interface FileDropZoneProps {
  label: string
  accept: string
  file: File | null
  onFileSelect: (file: File) => void
  t: (key: string) => string
}

function FileDropZone({ label, accept, file, onFileSelect, t }: FileDropZoneProps) {
  const inputRef = useRef<HTMLInputElement>(null)

  const handleClick = () => {
    inputRef.current?.click()
  }

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const selected = e.target.files?.[0]
    if (selected) {
      onFileSelect(selected)
    }
  }

  return (
    <div className="space-y-2">
      <label className="text-sm font-medium text-foreground">{label}</label>
      <div
        onClick={handleClick}
        className={`
          border-2 border-dashed rounded-lg p-6 text-center cursor-pointer
          transition-colors
          ${file
            ? 'border-success bg-success-light/20'
            : 'border-border hover:border-primary-foreground hover:bg-muted-30'
          }
        `}
      >
        <input
          ref={inputRef}
          type="file"
          accept={accept}
          onChange={handleChange}
          className="hidden"
        />
        {file ? (
          <div className="space-y-1">
            <lucideReact.FileCheck className="w-8 h-8 text-success mx-auto" />
            <p className="text-sm font-medium text-foreground">{file.name}</p>
            <p className="text-xs text-muted-foreground">
              {(file.size / 1024).toFixed(1)} KB
            </p>
          </div>
        ) : (
          <div className="space-y-1">
            <lucideReact.Upload className="w-8 h-8 text-muted-foreground mx-auto" />
            <p className="text-sm text-muted-foreground">{t('selectFile')}</p>
          </div>
        )}
      </div>
    </div>
  )
}

/**
 * Preview section
 */
interface PreviewSectionProps {
  manifest: ComponentManifest
  locale: string
  t: (key: string) => string
}

function PreviewSection({ manifest, locale, t }: PreviewSectionProps) {
  // Helper to get localized text
  const getLocalizedText = (value: string | Record<string, string>): string => {
    if (typeof value === 'string') return value
    return value[locale] || value.en || Object.values(value)[0] || ''
  }

  const name = getLocalizedText(manifest.name)
  const description = getLocalizedText(manifest.description)
  const version = manifest.version || '1.0.0'
  const author = manifest.author
  const id = manifest.id

  return (
    <div className="space-y-4 p-4 bg-muted-30 rounded-lg border border-border">
      <div>
        <h4 className="text-sm font-semibold text-foreground mb-3">{t('preview')}</h4>
        <div className="space-y-2">
          <div className="flex items-baseline gap-2">
            <span className="text-sm text-muted-foreground w-20">ID:</span>
            <span className="text-sm font-medium text-foreground font-mono">{id}</span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-sm text-muted-foreground w-20">{t('componentLibrary.valueCard')}:</span>
            <span className="text-sm font-medium text-foreground">{name}</span>
          </div>
          {description && (
            <div className="flex items-start gap-2">
              <span className="text-sm text-muted-foreground w-20 shrink-0">{t('description')}:</span>
              <span className="text-sm text-foreground">{description}</span>
            </div>
          )}
          <div className="flex items-baseline gap-2">
            <span className="text-sm text-muted-foreground w-20">{t('version')}:</span>
            <span className="text-sm text-foreground">{version}</span>
          </div>
          {author && (
            <div className="flex items-baseline gap-2">
              <span className="text-sm text-muted-foreground w-20">{t('by')}:</span>
              <span className="text-sm text-foreground">{author}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

/**
 * Main Install Component Dialog
 */
export function InstallComponentDialog({ open, onOpenChange }: InstallComponentDialogProps) {
  const { t, i18n } = useTranslation('dashboardComponents')
  const locale = i18n.language

  const { installManual } = useStore()

  const [manifestFile, setManifestFile] = useState<File | null>(null)
  const [bundleFile, setBundleFile] = useState<File | null>(null)
  const [parsedManifest, setParsedManifest] = useState<ComponentManifest | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [submitError, setSubmitError] = useState<string | null>(null)

  // Parse manifest when file changes
  const handleManifestSelect = async (file: File) => {
    setManifestFile(file)
    setSubmitError(null)

    try {
      const text = await file.text()
      const manifest = JSON.parse(text) as ComponentManifest

      // Basic validation
      if (!manifest.id || !manifest.name || !manifest.global_name) {
        throw new Error('Invalid manifest: missing required fields')
      }

      setParsedManifest(manifest)
    } catch (error) {
      setSubmitError(error instanceof Error ? error.message : 'Failed to parse manifest')
      setParsedManifest(null)
    }
  }

  const handleBundleSelect = (file: File) => {
    setBundleFile(file)
    setSubmitError(null)
  }

  const handleInstall = async () => {
    if (!parsedManifest || !bundleFile) return

    setIsSubmitting(true)
    setSubmitError(null)

    try {
      await installManual(parsedManifest, bundleFile)
      notifySuccess(t('installSuccess'))

      // Reset and close
      setManifestFile(null)
      setBundleFile(null)
      setParsedManifest(null)
      onOpenChange(false)
    } catch (error) {
      setSubmitError(error instanceof Error ? error.message : t('installError'))
      notifyFromError(error, t('installError'))
    } finally {
      setIsSubmitting(false)
    }
  }

  const canInstall = parsedManifest && bundleFile && !isSubmitting

  // Reset when dialog closes
  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      setManifestFile(null)
      setBundleFile(null)
      setParsedManifest(null)
      setSubmitError(null)
    }
    onOpenChange(newOpen)
  }

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={handleOpenChange}
      title={t('componentLibrary.importTitle')}
      icon={<lucideReact.PackagePlus className="w-full h-full" />}
      width="lg"
      isSubmitting={isSubmitting}
      submitError={submitError}
      onSubmit={handleInstall}
      submitLabel={t('installConfirm')}
      submitDisabled={!canInstall}
    >
      <div className="space-y-6">
        {/* File drop zones */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <FileDropZone
            label={t('componentLibrary.manifestFile')}
            accept=".json"
            file={manifestFile}
            onFileSelect={handleManifestSelect}
            t={t}
          />
          <FileDropZone
            label={t('componentLibrary.bundleFile')}
            accept=".js"
            file={bundleFile}
            onFileSelect={handleBundleSelect}
            t={t}
          />
        </div>

        {/* Preview */}
        {parsedManifest && (
          <PreviewSection manifest={parsedManifest} locale={locale} t={t} />
        )}

        {/* Instructions */}
        {!parsedManifest && (
          <div className="p-4 bg-muted-30 rounded-lg border border-border">
            <h4 className="text-sm font-semibold text-foreground mb-2">{t('description')}</h4>
            <ul className="text-xs text-muted-foreground space-y-1 list-disc list-inside">
              <li>{t('componentLibrary.manifestFile')}: {t('componentLibrary.aiAnalystDesc')}</li>
              <li>{t('componentLibrary.bundleFile')}: JavaScript bundle file</li>
            </ul>
          </div>
        )}
      </div>
    </UnifiedFormDialog>
  )
}
