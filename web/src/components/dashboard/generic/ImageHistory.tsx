/**
 * Image History Component
 *
 * Displays historical image data with manual slider navigation.
 * Supports base64 images, URLs, and data URLs.
 */

import { useState, useCallback, useRef, useEffect, useMemo } from 'react'
import { Slider } from '@/components/ui/slider'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'
import { ImageOff, AlertTriangle, RefreshCw, Images } from 'lucide-react'
import type { DataSource } from '@/types/dashboard'

type ImageFormatType = 'png' | 'jpeg' | 'jpg' | 'gif' | 'webp' | 'bmp' | 'svg' | 'tiff' | 'ico' | 'unknown'

// Magic bytes for image type detection
const IMAGE_MAGIC_BYTES: Record<string, { magic: number[]; type: ImageFormatType; mime: string }> = {
  png: { magic: [0x89, 0x50, 0x4E, 0x47], type: 'png', mime: 'image/png' },
  jpeg: { magic: [0xFF, 0xD8, 0xFF], type: 'jpeg', mime: 'image/jpeg' },
  jpg: { magic: [0xFF, 0xD8, 0xFF], type: 'jpg', mime: 'image/jpeg' },
  gif: { magic: [0x47, 0x49, 0x46], type: 'gif', mime: 'image/gif' },
  webp: { magic: [0x52, 0x49, 0x46, 0x46], type: 'webp', mime: 'image/webp' },
  bmp: { magic: [0x42, 0x4D], type: 'bmp', mime: 'image/bmp' },
  tiff: { magic: [0x49, 0x49, 0x2A, 0x00], type: 'tiff', mime: 'image/tiff' },
  ico: { magic: [0x00, 0x00, 0x01, 0x00], type: 'ico', mime: 'image/x-icon' },
}

function detectImageFormatFromMagicBytes(base64Data: string): { type: ImageFormatType; mime: string } | null {
  try {
    const pureBase64 = base64Data.replace(/^data:image\/[^;]+;base64,/, '').replace(/^data:,/, '')
    const binaryString = atob(pureBase64.slice(0, 32))

    for (const [name, info] of Object.entries(IMAGE_MAGIC_BYTES)) {
      if (info.magic.every((byte, i) => binaryString.charCodeAt(i) === byte)) {
        return { type: info.type as ImageFormatType, mime: info.mime }
      }
    }
  } catch {
    // Invalid base64
  }
  return null
}

function isPureBase64(str: string): boolean {
  if (!str || str.length < 100) return false
  const cleaned = str.trim()

  if (cleaned.startsWith('http://') || cleaned.startsWith('https://') || cleaned.startsWith('/')) {
    return false
  }
  if (cleaned.startsWith('data:')) {
    return false
  }

  const base64Regex = /^[A-Za-z0-9+/=_-]+$/
  if (!base64Regex.test(cleaned)) {
    return false
  }

  try {
    atob(cleaned.slice(0, 100))
    return true
  } catch {
    return false
  }
}

function normalizeImageUrl(value: string): string | null {
  if (!value) return null

  const trimmed = value.trim()

  if (trimmed === '-' || trimmed === 'undefined' || trimmed === 'null' || trimmed === '') {
    return null
  }

  if (trimmed.startsWith('data:image/')) {
    return trimmed
  }

  if (trimmed.startsWith('data:base64,')) {
    const base64Data = trimmed.slice(12)
    const formatInfo = detectImageFormatFromMagicBytes(base64Data) || { type: 'png', mime: 'image/png' }
    return `data:${formatInfo.mime};base64,${base64Data}`
  }

  if (trimmed.startsWith('data:')) {
    return trimmed
  }

  if (isPureBase64(trimmed)) {
    const formatInfo = detectImageFormatFromMagicBytes(trimmed) || { type: 'png', mime: 'image/png' }
    return `data:${formatInfo.mime};base64,${trimmed}`
  }

  return trimmed
}

export interface ImageHistoryItem {
  src: string
  timestamp?: string | number
  label?: string
  alt?: string
}

export interface ImageHistoryProps {
  dataSource?: DataSource
  images?: ImageHistoryItem[] | string[]
  title?: string
  size?: 'sm' | 'md' | 'lg'

  // Display options
  fit?: 'contain' | 'cover' | 'fill' | 'none' | 'scale-down'
  rounded?: boolean
  showTitle?: boolean

  // Data options for telemetry
  limit?: number
  timeRange?: number  // hours

  className?: string
}

type ImageLoadState = 'loading' | 'loaded' | 'error'

/**
 * Helper to extract timestamp from an object
 * Returns timestamp in milliseconds for consistency with JavaScript Date
 */
function extractTimestamp(obj: Record<string, unknown>): number | undefined {
  const ts = obj.timestamp ?? obj.time ?? obj.t
  if (ts === undefined || ts === null) return undefined
  if (typeof ts === 'number') {
    // Assume milliseconds if > 10000000000 (year 2286+), otherwise seconds
    return ts > 10000000000 ? ts : ts * 1000
  }
  if (typeof ts === 'string') {
    const parsed = Date.parse(ts)
    if (!isNaN(parsed)) return parsed  // Date.parse returns milliseconds
  }
  return undefined
}

/**
 * Transform telemetry points to ImageHistoryItem array
 * Sorts by timestamp descending (newest first)
 */
function transformTelemetryToImages(data: unknown): ImageHistoryItem[] {
  if (!Array.isArray(data)) {
    return []
  }

  const result: ImageHistoryItem[] = []

  for (let index = 0; index < data.length; index++) {
    const point = data[index]

    if (typeof point === 'string') {
      const normalizedSrc = normalizeImageUrl(point)
      if (normalizedSrc) {
        result.push({ src: normalizedSrc, alt: `Image ${index + 1}` })
      }
    } else if (typeof point === 'object' && point !== null) {
      const obj = point as Record<string, unknown>

      // Try various value fields
      const rawSrc = String(
        obj.src ??
        obj.url ??
        obj.image ??
        obj.value ??
        obj.v ??
        ''
      )

      const normalizedSrc = normalizeImageUrl(rawSrc)
      if (!normalizedSrc) {
        continue
      }

      // Extract timestamp
      const timestamp = extractTimestamp(obj)

      // Extract label
      const label = typeof obj.label === 'string' ? obj.label :
                    typeof obj.name === 'string' ? obj.name : undefined

      result.push({
        src: normalizedSrc,
        timestamp,
        label,
        alt: `Image ${index + 1}`,
      })
    }
  }

  // Sort by timestamp descending (newest first), items without timestamp go to the end
  result.sort((a, b) => {
    if (a.timestamp === undefined && b.timestamp === undefined) return 0
    if (a.timestamp === undefined) return 1
    if (b.timestamp === undefined) return -1
    return (b.timestamp as number) - (a.timestamp as number)
  })

  return result
}

function normalizeImageData(data: unknown): ImageHistoryItem[] {
  if (data === null || data === undefined) {
    return []
  }

  if (Array.isArray(data)) {
    const result: ImageHistoryItem[] = []

    for (let index = 0; index < data.length; index++) {
      const item = data[index]

      if (typeof item === 'string') {
        const normalizedSrc = normalizeImageUrl(item)
        if (normalizedSrc) {
          result.push({
            src: normalizedSrc,
            alt: `Image ${index + 1}`,
          })
        }
      } else if (typeof item === 'object' && item !== null) {
        const obj = item as Record<string, unknown>
        const timestamp = extractTimestamp(obj)
        const label = obj.label ?? obj.name
        const rawSrc = String(obj.src || obj.url || obj.image || obj.value || '')
        const normalizedSrc = normalizeImageUrl(rawSrc)

        if (normalizedSrc) {
          result.push({
            src: normalizedSrc,
            timestamp,
            label: (typeof label === 'string') ? label : undefined,
            alt: (typeof obj.alt === 'string') ? obj.alt : `Image ${index + 1}`,
          })
        }
      } else {
        const normalizedSrc = normalizeImageUrl(String(item))
        if (normalizedSrc) {
          result.push({
            src: normalizedSrc,
            alt: `Image ${index + 1}`,
          })
        }
      }
    }

    // Sort by timestamp descending (newest first), items without timestamp go to the end
    result.sort((a, b) => {
      if (a.timestamp === undefined && b.timestamp === undefined) return 0
      if (a.timestamp === undefined) return 1
      if (b.timestamp === undefined) return -1
      return (b.timestamp as number) - (a.timestamp as number)
    })

    return result
  }

  if (typeof data === 'string') {
    const normalizedSrc = normalizeImageUrl(data)
    if (normalizedSrc) {
      return [{ src: normalizedSrc, alt: 'Image 1' }]
    }
    return []
  }

  if (typeof data === 'object' && data !== null) {
    const obj = data as Record<string, unknown>
    const arrayProps = ['images', 'image', 'history', 'snapshots', 'frames', 'data', 'values', 'items', 'points']

    for (const prop of arrayProps) {
      if (prop in obj) {
        const value = obj[prop]
        if (Array.isArray(value)) {
          return normalizeImageData(value)
        }
      }
    }

    if ('src' in obj || 'url' in obj || 'image' in obj || 'value' in obj) {
      const rawSrc = String(obj.src || obj.url || obj.image || obj.value || '')
      const normalizedSrc = normalizeImageUrl(rawSrc)
      if (normalizedSrc) {
        return [{
          src: normalizedSrc,
          timestamp: (typeof obj.timestamp === 'string' || typeof obj.timestamp === 'number') ? obj.timestamp : undefined,
          label: (typeof obj.label === 'string') ? obj.label : undefined,
          alt: (typeof obj.alt === 'string') ? obj.alt : 'Image 1',
        }]
      }
    }
  }

  return []
}

function formatTimestamp(timestamp: string | number | undefined): string {
  if (!timestamp) return ''

  // timestamp is already in milliseconds from extractTimestamp
  const ts = typeof timestamp === 'number' ? timestamp : timestamp
  const date = new Date(ts)
  if (isNaN(date.getTime())) return String(timestamp)

  return date.toLocaleTimeString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

/**
 * Convert device data source to telemetry for historical data
 * Uses a larger time range (48 hours) for image data since images are sent infrequently
 */
function normalizeDataSourceForImages(
  ds: DataSource | undefined,
  limit: number = 200,
  timeRange: number = 1
): DataSource | undefined {
  if (!ds) return undefined

  // Use 48 hours as the default time range for images (override if explicitly set larger)
  const imageTimeRange = timeRange > 1 ? timeRange : 48

  // If it's already telemetry, return as-is with raw transform and custom limits
  if (ds.type === 'telemetry') {
    const originalLimit = ds.limit
    const newLimit = Math.max(ds.limit ?? 0, limit)
    const originalTimeRange = ds.timeRange
    const newTimeRange = ds.timeRange && ds.timeRange > 48 ? ds.timeRange : imageTimeRange

    const result = {
      ...ds,
      // Use the larger of: config limit or component default limit
      limit: newLimit,
      // Use image-specific time range (48 hours) unless data source has larger value
      timeRange: newTimeRange,
      params: {
        ...ds.params,
        includeRawPoints: true,
      },
      transform: 'raw',
    }

    return result
  }

  // If it's a device type, convert to telemetry for historical data
  if (ds.type === 'device') {
    return {
      type: 'telemetry',
      deviceId: ds.deviceId,
      metricId: ds.metricId ?? ds.property ?? 'image',
      timeRange: imageTimeRange,
      limit: limit,
      aggregate: 'raw',
      params: {
        includeRawPoints: true,
      },
      transform: 'raw',
    }
  }

  return ds
}

export function ImageHistory({
  dataSource,
  images: propImages,
  title,
  size = 'md',
  fit = 'fill',
  rounded = true,
  showTitle = true,
  limit = 200,
  timeRange = 1,
  className,
}: ImageHistoryProps) {
  // Get size configuration
  const sizeConfig = dashboardComponentSize[size]

  // Normalize data source for image history (convert device to telemetry)
  const normalizedDataSource = useMemo(() => {
    return normalizeDataSourceForImages(dataSource, limit, timeRange)
  }, [dataSource, limit, timeRange])

  const { data, loading } = useDataSource<ImageHistoryItem[] | string[]>(normalizedDataSource, {
    fallback: propImages,
  })

  // Memoize images to prevent unnecessary recalculation
  // Use a more reliable change detection that works for base64 images
  // Base64 images have the same prefix, so we use length + timestamp of last item
  const dataKey = useMemo(() => {
    if (!data) return 'no-data'
    if (Array.isArray(data)) {
      const lastItem = data.length > 0 ? data[data.length - 1] : null
      // For base64 images, use length + timestamp instead of string content
      // since all base64 images start with similar prefix (data:image/jpeg;base64,...)
      let lastItemKey = ''
      if (lastItem) {
        if (typeof lastItem === 'object' && lastItem !== null) {
          const obj = lastItem as unknown as Record<string, unknown>
          const ts = obj.timestamp ?? obj.time ?? obj.t ?? 0
          // Use value length (for base64) + timestamp for change detection
          const valueStr = String(obj.src ?? obj.url ?? obj.value ?? '')
          const valueLen = valueStr.length > 0 ? valueStr.length : 0
          lastItemKey = `ts:${ts}-len:${valueLen}`
        } else {
          // For strings, use length + first 100 chars (more than 50 to be safer)
          const str = String(lastItem)
          lastItemKey = `len:${str.length}-${str.slice(0, 100)}`
        }
      }
      return `array-${data.length}-${lastItemKey}`
    }
    return `object-${JSON.stringify(data).slice(0, 50)}`
  }, [data])

  // Transform data to images - handles telemetry raw points
  const images = useMemo(() => {
    const normalized = normalizeImageData(data ?? propImages ?? [])
    const result = normalized.length > 0 ? normalized : transformTelemetryToImages(data ?? propImages ?? [])
    return result
  }, [dataKey, propImages])

  // Track image SOURCES (not just indices) to detect real changes vs reordering
  const imageSourcesRef = useRef<string[]>([])

  const [currentIndex, setCurrentIndex] = useState(0)
  const [imageLoadState, setImageLoadState] = useState<ImageLoadState>('loading')

  // Cache loaded image states by SOURCE URL (not index) for better persistence
  const loadedImagesSrcRef = useRef<Set<string>>(new Set())

  // Track data update count for cache-busting (forces image reload when data changes)
  const [dataUpdateCount, setDataUpdateCount] = useState(0)
  const dataUpdateCountRef = useRef(0)
  dataUpdateCountRef.current = dataUpdateCount

  // Update data update count when images array changes
  useEffect(() => {
    if (images.length > 0) {
      setDataUpdateCount(c => c + 1)
    }
  }, [images.length, dataKey])  // Trigger when images array or dataKey changes

  const currentImage = images[currentIndex]
  const currentImageSrc = currentImage?.src
  const hasImages = images.length > 0
  const canNavigate = images.length > 1

  // Add cache-busting for base64/data URLs to force reload when data changes
  const displayImageSrc = useMemo(() => {
    if (!currentImageSrc) return currentImageSrc
    // Add cache buster for data URLs (base64 images)
    if (currentImageSrc.startsWith('data:') || currentImageSrc.startsWith('blob:')) {
      return `${currentImageSrc}#${dataUpdateCountRef.current}`
    }
    return currentImageSrc
  }, [currentImageSrc, dataUpdateCount])

  // Reset index and loading state only when actual images change (not just reordering)
  useEffect(() => {
    const currentSources = images.map(img => img.src)
    const prevSources = imageSourcesRef.current

    // Check if the actual images changed (not just order)
    const imagesChanged =
      currentSources.length !== prevSources.length ||
      currentSources.some((src, i) => src !== prevSources[i])

    if (imagesChanged) {
      imageSourcesRef.current = currentSources
      setCurrentIndex(0)
      // Only clear loaded cache if images actually changed
      loadedImagesSrcRef.current = new Set(currentSources.filter(src => prevSources.includes(src)))
      // Set loading state for current image
      if (currentSources.length > 0 && loadedImagesSrcRef.current.has(currentSources[0])) {
        setImageLoadState('loaded')
      } else {
        setImageLoadState('loading')
      }
    }
  }, [images])

  // Update image load state when image source changes
  const prevImageSrcRef = useRef<string | undefined>(undefined)
  const prevIndexRef = useRef<number>(-1)
  useEffect(() => {
    const imageChanged = currentImageSrc && currentImageSrc !== prevImageSrcRef.current
    const indexChanged = currentIndex !== prevIndexRef.current

    if (imageChanged) {
      if (currentImageSrc && loadedImagesSrcRef.current.has(currentImageSrc)) {
        setImageLoadState('loaded')
      } else {
        setImageLoadState('loading')
      }
      prevImageSrcRef.current = currentImageSrc
    } else if (indexChanged && currentImageSrc) {
      setImageLoadState(loadedImagesSrcRef.current.has(currentImageSrc) ? 'loaded' : 'loading')
    }

    prevIndexRef.current = currentIndex
  }, [currentImageSrc, currentIndex])

  const handleImageLoad = useCallback(() => {
    setImageLoadState('loaded')
    if (currentImageSrc) {
      loadedImagesSrcRef.current.add(currentImageSrc)
    }
  }, [currentImageSrc])

  const handleImageError = useCallback(() => {
    setImageLoadState('error')
  }, [])

  const handleSliderChange = useCallback((values: number[]) => {
    const index = values[0] ?? 0
    setCurrentIndex(index)
  }, [])

  // Loading state
  if (loading) {
    return (
      <div className={cn(dashboardCardBase, className)}>
        <div className={cn(
          'w-full flex items-center justify-center bg-muted/10',
          size === 'sm' ? 'h-[120px]' : size === 'md' ? 'h-[180px]' : 'h-[240px]'
        )}>
          <RefreshCw className="h-6 w-6 text-muted-foreground animate-spin" />
        </div>
      </div>
    )
  }

  // No images state
  if (!hasImages) {
    return (
      <div className={cn(
        dashboardCardBase,
        'flex flex-col items-center justify-center gap-3 bg-muted/30',
        sizeConfig.padding,
        className
      )}>
        <ImageOff className={cn(
          'text-muted-foreground/60',
          size === 'sm' ? 'h-8 w-8' : size === 'md' ? 'h-12 w-12' : 'h-16 w-16'
        )} />
        <div className="text-center">
          <p className="text-muted-foreground text-sm font-medium">No Images</p>
          <p className="text-muted-foreground/50 text-xs mt-1">Configure an image data source</p>
        </div>
      </div>
    )
  }

  return (
    <div className={cn(
      dashboardCardBase,
      'relative flex flex-col overflow-hidden',
      className
    )}>
      <div className={cn(
        'w-full flex-1 relative',
        size === 'sm' ? 'h-[120px]' : size === 'md' ? 'h-[180px]' : 'h-[240px]'
      )}>
      {/* Image fills entire container */}
      <img
        key={`img-${currentIndex}-${dataUpdateCount}`}
        src={displayImageSrc}
        alt={currentImage?.alt || `Image ${currentIndex + 1}`}
        className={cn(
          'w-full h-full',
          fit === 'contain' && 'object-contain',
          fit === 'cover' && 'object-cover',
          fit === 'fill' && 'object-fill',
          fit === 'none' && 'object-none',
          fit === 'scale-down' && 'object-scale-down'
        )}
        loading="lazy"
        onLoad={handleImageLoad}
        onError={handleImageError}
      />

      {/* Top-left title + index overlay */}
      <div className={cn(
        "absolute left-2 flex items-center gap-2 bg-black/60 text-white text-xs px-2 py-1 rounded z-10",
        title && showTitle || canNavigate ? "top-2" : "hidden"
      )}>
        {title && showTitle && (
          <>
            <Images className="h-3.5 w-3.5 text-white/90 shrink-0" />
            <span className="font-medium text-xs truncate max-w-[120px] text-white drop-shadow-md">{title}</span>
            {canNavigate && <span className="w-px h-3 bg-white/30" />}
          </>
        )}
        {canNavigate && (
          <span className="tabular-nums">{currentIndex + 1} / {images.length}</span>
        )}
      </div>

      {/* Top-right timestamp overlay */}
      {currentImage?.timestamp && (
        <div className="absolute top-2 right-2 bg-black/60 text-white text-xs px-2 py-1 rounded z-10">
          {formatTimestamp(currentImage.timestamp)}
        </div>
      )}

      {/* Label overlay - positioned below title/index */}
      {currentImage?.label && (
        <div className="absolute top-9 left-2 bg-black/60 text-white text-xs px-2 py-1 rounded max-w-[150px] truncate z-10">
          {currentImage.label}
        </div>
      )}

      {/* Loading indicator */}
      {imageLoadState === 'loading' && (
        <div className="absolute inset-0 flex items-center justify-center bg-background/50 z-20">
          <RefreshCw className="h-6 w-6 text-muted-foreground animate-spin" />
        </div>
      )}

      {/* Error indicator */}
      {imageLoadState === 'error' && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-background/80 z-20">
          <AlertTriangle className="h-8 w-8 text-destructive mb-2" />
          <span className="text-sm text-muted-foreground">Failed to load</span>
        </div>
      )}

      {/* Floating slider at bottom - always on top */}
      {canNavigate && (
        <div className="absolute bottom-0 left-0 right-0 px-3 py-3 bg-gradient-to-t from-black/70 via-black/40 to-transparent z-30">
          <div className="flex items-center gap-3">
            <Slider
              value={[currentIndex]}
              min={0}
              max={images.length - 1}
              step={1}
              onValueChange={handleSliderChange}
              className="flex-1"
            />
            <span className="text-xs text-white tabular-nums min-w-[50px] text-right shrink-0">
              {currentIndex + 1} / {images.length}
            </span>
          </div>
        </div>
      )}
      </div>
    </div>
  )
}
