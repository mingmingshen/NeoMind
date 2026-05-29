import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Trash2, Upload } from 'lucide-react'
import { notifyError } from '@/lib/notify'
import { Field } from '@/components/ui/field'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Button } from '@/components/ui/button'

// ============================================================================
// SelectField - Unified select dropdown for config dialogs
// ============================================================================

export interface SelectOption {
  value: string
  label: string
}

export interface SelectFieldProps {
  label: string
  value: string
  onChange: (value: string) => void
  options: SelectOption[]
  className?: string
}

export function SelectField({ label, value, onChange, options, className }: SelectFieldProps) {
  const { t } = useTranslation()

  const handleChange = (newValue: string) => {
    onChange(newValue)
  }

  return (
    <Field className={className}>
      <Label>{label}</Label>
      <Select value={value} onValueChange={handleChange}>
        <SelectTrigger>
          <SelectValue placeholder={t('visualDashboard.selectPlaceholder', { label })} />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  )
}

// ============================================================================
// ImageSourceField - URL input + file upload for image config
// ============================================================================

export interface ImageSourceFieldProps {
  value: string
  onChange: (value: string) => void
}

/**
 * Compress an image file to a compact JPEG data URL.
 * Resizes to fit within MAX_DIMENSION and reduces quality to hit target size.
 * Returns original data URL for tiny images that don't need compression.
 */
function compressImage(file: File): Promise<string> {
  const MAX_DIMENSION = 1200
  const TARGET_BYTES = 150 * 1024 // 150KB target

  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file)
    const img = new Image()
    img.onload = () => {
      URL.revokeObjectURL(url)

      // Skip compression for tiny images
      if (img.width <= MAX_DIMENSION && img.height <= MAX_DIMENSION && file.size <= TARGET_BYTES) {
        const reader = new FileReader()
        reader.onload = (e) => resolve(e.target?.result as string)
        reader.onerror = () => reject(new Error('Failed to read file'))
        reader.readAsDataURL(file)
        return
      }

      // Calculate scaled dimensions
      let { width, height } = img
      if (width > MAX_DIMENSION || height > MAX_DIMENSION) {
        const ratio = Math.min(MAX_DIMENSION / width, MAX_DIMENSION / height)
        width = Math.round(width * ratio)
        height = Math.round(height * ratio)
      }

      const canvas = document.createElement('canvas')
      canvas.width = width
      canvas.height = height
      const ctx = canvas.getContext('2d')!
      ctx.drawImage(img, 0, 0, width, height)

      // Try quality levels until under target
      let quality = 0.8
      let dataUrl = canvas.toDataURL('image/jpeg', quality)
      while (dataUrl.length > TARGET_BYTES * 1.37 && quality > 0.2) { // base64 ~37% overhead
        quality -= 0.15
        dataUrl = canvas.toDataURL('image/jpeg', quality)
      }

      resolve(dataUrl)
    }
    img.onerror = () => {
      URL.revokeObjectURL(url)
      reject(new Error('Failed to load image'))
    }
    img.src = url
  })
}

export function ImageSourceField({ value, onChange }: ImageSourceFieldProps) {
  const { t } = useTranslation()
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    if (!file.type.startsWith('image/')) {
      notifyError(t('visualDashboard.invalidFileType'))
      return
    }

    try {
      const compressed = await compressImage(file)
      onChange(compressed)
    } catch {
      notifyError(t('visualDashboard.fileTooLarge'))
    }
    // Reset input to allow re-uploading the same file
    e.target.value = ''
  }

  const handleUploadClick = () => {
    fileInputRef.current?.click()
  }

  const handleClear = () => {
    onChange('')
  }

  const isBase64Image = value?.startsWith('data:image')

  return (
    <div className="space-y-3">
      <Field>
        <Label>{t('visualDashboard.imageSource')}</Label>
        <div className="flex gap-2">
          <Input
            value={value || ''}
            onChange={(e) => onChange(e.target.value)}
            placeholder={t('visualDashboard.urlPlaceholder')}
            className="h-9 flex-1"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={handleUploadClick}
            className="h-9 px-3 shrink-0"
          >
            <Upload className="h-4 w-4 mr-1.5" />
            {t('visualDashboard.upload')}
          </Button>
        </div>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          onChange={handleFileSelect}
          className="hidden"
        />
        <p className="text-xs text-muted-foreground mt-1">
          {isBase64Image
            ? t('visualDashboard.uploadedHint')
            : t('visualDashboard.urlHint')}
        </p>
      </Field>

      {isBase64Image && (
        <div className="flex items-center gap-2">
          <div className="w-12 h-12 rounded border overflow-hidden bg-muted-30">
            <img
              src={value}
              alt="Preview"
              className="w-full h-full object-contain"
            />
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClear}
            className="h-8 text-destructive hover:text-destructive"
          >
            <Trash2 className="h-4 w-4 mr-1" />
            {t('visualDashboard.clear')}
          </Button>
        </div>
      )}
    </div>
  )
}
