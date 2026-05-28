/**
 * Image History Component
 *
 * Displays historical image data with manual slider navigation.
 * Supports base64 images, URLs, and data URLs.
 * Uses shared image utilities from @/lib/imageUtils.
 */

import { useState, useCallback, useRef, useEffect, useMemo, memo } from 'react'
import { useTranslation } from 'react-i18next'
import { Slider } from '@/components/ui/slider'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'
import { ImageOff, AlertTriangle, RefreshCw, Images } from 'lucide-react'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import { LoadingState } from '../shared'
import {
  normalizeImageUrl,
  extractTimestamp,
} from '@/lib/imageUtils'

// ============================================================================
// Types
// ============================================================================

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

// ============================================================================
// Data normalization
// ============================================================================

/**
 * Unified data → ImageHistoryItem[] transformation.
 * Replaces the old normalizeImageData + transformTelemetryToImages pair.
 * Uses the cached normalizeImageUrl from shared utils.
 */
function toImageHistoryItems(data: unknown): ImageHistoryItem[] {
  if (data === null || data === undefined) return []

  // Array: process each item
  if (Array.isArray(data)) {
    const result: ImageHistoryItem[] = []
    for (let i = 0; i < data.length; i++) {
      const item = data[i]

      if (typeof item === 'string') {
        const norm = normalizeImageUrl(item)
        if (norm) result.push({ src: norm.src, alt: `Image ${i + 1}` })
      } else if (typeof item === 'number' || typeof item === 'boolean') {
        // skip — can't be an image
      } else if (item !== null && typeof item === 'object') {
        const obj = item as Record<string, unknown>
        const rawSrc = String(obj.src ?? obj.url ?? obj.image ?? obj.imageUrl ?? obj.value ?? obj.v ?? '')
        const norm = normalizeImageUrl(rawSrc)
        if (norm) {
          result.push({
            src: norm.src,
            timestamp: extractTimestamp(obj),
            label: typeof obj.label === 'string' ? obj.label : typeof obj.name === 'string' ? obj.name as string : undefined,
            alt: typeof obj.alt === 'string' ? obj.alt as string : `Image ${i + 1}`,
          })
        }
      }
    }

    // Stable sort by timestamp descending (newest first), undefined timestamps go to end
    const indexed = result.map((item, i) => ({ item, i }))
    indexed.sort((a, b) => {
      if (a.item.timestamp === undefined && b.item.timestamp === undefined) return a.i - b.i
      if (a.item.timestamp === undefined) return 1
      if (b.item.timestamp === undefined) return -1
      const diff = (b.item.timestamp as number) - (a.item.timestamp as number)
      return diff !== 0 ? diff : a.i - b.i
    })
    indexed.forEach(({ item }, i) => { result[i] = item })
    return result
  }

  // Single string
  if (typeof data === 'string') {
    const norm = normalizeImageUrl(data)
    return norm ? [{ src: norm.src, alt: 'Image 1' }] : []
  }

  // Object: dig into common array properties, or extract single image
  if (typeof data === 'object') {
    const obj = data as Record<string, unknown>
    const arrayProps = ['images', 'image', 'history', 'snapshots', 'frames', 'data', 'values', 'items', 'points']
    for (const prop of arrayProps) {
      if (prop in obj && Array.isArray(obj[prop])) {
        return toImageHistoryItems(obj[prop])
      }
    }
    // Single image object
    const rawSrc = String(obj.src ?? obj.url ?? obj.image ?? obj.value ?? '')
    const norm = normalizeImageUrl(rawSrc)
    if (norm) {
      return [{
        src: norm.src,
        timestamp: extractTimestamp(obj),
        label: typeof obj.label === 'string' ? obj.label as string : undefined,
        alt: typeof obj.alt === 'string' ? obj.alt as string : 'Image 1',
      }]
    }
  }

  return []
}

// ============================================================================
// Helpers
// ============================================================================

function formatTimestamp(timestamp: string | number | undefined): string {
  if (!timestamp) return ''
  const date = new Date(timestamp)
  if (isNaN(date.getTime())) return String(timestamp)
  return date.toLocaleTimeString('zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
  })
}

function normalizeDataSourceForImages(
  ds: DataSource | undefined,
  limit: number = 200,
  timeRange: number = 48,
): DataSource | undefined {
  if (!ds) return undefined

  if (ds.type === 'telemetry') {
    return {
      ...ds,
      limit,
      timeRange,
      params: { ...ds.params, includeRawPoints: true, isImage: true },
      transform: 'raw',
    }
  }

  if (ds.type === 'device' || ds.type === 'metric') {
    const sourceId = getSourceId(ds)
    return {
      type: 'telemetry',
      sourceId,
      metricId: ds.metricId ?? ds.property ?? 'image',
      timeRange,
      limit,
      aggregate: 'raw',
      params: { includeRawPoints: true, isImage: true },
      transform: 'raw',
      refresh: ds.refresh ?? 60,
    }
  }

  return ds
}

// ============================================================================
// Component
// ============================================================================

export const ImageHistory = memo(function ImageHistory({
  dataSource,
  images: propImages,
  title,
  size = 'md',
  fit = 'fill',
  rounded = true,
  showTitle = true,
  limit = 200,
  timeRange = 48,
  className,
}: ImageHistoryProps) {
  const { t } = useTranslation('dashboardComponents')
  const sizeConfig = dashboardComponentSize[size]

  // Normalized data source — memoized
  const normalizedDataSource = useMemo(
    () => normalizeDataSourceForImages(dataSource, limit, timeRange),
    [dataSource, limit, timeRange],
  )

  const { data, loading, lastUpdate: dataSourceLastUpdate } = useDataSource(normalizedDataSource, {
    fallback: propImages,
  })

  // Transform data to images — uses shared cached normalizeImageUrl
  const images = useMemo(
    () => toImageHistoryItems(data ?? propImages ?? []),
    [data, propImages],
  )

  // Track sources to detect real changes (not just reordering)
  const imageSourcesRef = useRef<string[]>([])
  const [currentIndex, setCurrentIndex] = useState(0)
  const [imageLoadState, setImageLoadState] = useState<ImageLoadState>('loading')
  const loadedImagesSrcRef = useRef<Set<string>>(new Set())

  // Cache-bust timestamp — derived from dataSourceLastUpdate
  const cacheBustTimestamp = dataSourceLastUpdate ?? 0

  const currentImage = images[currentIndex]
  const currentImageSrc = currentImage?.src
  const hasImages = images.length > 0
  const canNavigate = images.length > 1

  // Display src — only add cache buster for data URLs
  const displayImageSrc = useMemo(() => {
    if (!currentImageSrc) return currentImageSrc
    if ((currentImageSrc.startsWith('data:') || currentImageSrc.startsWith('blob:')) && cacheBustTimestamp) {
      return `${currentImageSrc}#${cacheBustTimestamp}`
    }
    return currentImageSrc
  }, [currentImageSrc, cacheBustTimestamp])

  // Reset index when actual image sources change (not just order)
  useEffect(() => {
    const currentSources = images.map(img => img.src)
    const prevSources = imageSourcesRef.current
    const changed = currentSources.length !== prevSources.length ||
      currentSources.some((src, i) => src !== prevSources[i])

    if (changed) {
      imageSourcesRef.current = currentSources
      setCurrentIndex(0)
      loadedImagesSrcRef.current = new Set(currentSources.filter(src => prevSources.includes(src)))
      setImageLoadState(
        currentSources.length > 0 && loadedImagesSrcRef.current.has(currentSources[0])
          ? 'loaded' : 'loading',
      )
    }
  }, [images])

  // Update load state on src/index change
  const prevImageSrcRef = useRef<string | undefined>()
  const prevIndexRef = useRef(-1)
  useEffect(() => {
    const srcChanged = currentImageSrc && currentImageSrc !== prevImageSrcRef.current
    const idxChanged = currentIndex !== prevIndexRef.current

    if (srcChanged || idxChanged) {
      if (currentImageSrc) {
        setImageLoadState(loadedImagesSrcRef.current.has(currentImageSrc) ? 'loaded' : 'loading')
      }
      prevImageSrcRef.current = currentImageSrc
    }
    prevIndexRef.current = currentIndex
  }, [currentImageSrc, currentIndex])

  // Callbacks
  const handleImageLoad = useCallback(() => {
    setImageLoadState('loaded')
    if (currentImageSrc) loadedImagesSrcRef.current.add(currentImageSrc)
  }, [currentImageSrc])

  const handleImageError = useCallback(() => setImageLoadState('error'), [])
  const handleSliderChange = useCallback((values: number[]) => setCurrentIndex(values[0] ?? 0), [])

  // --- Loading ---
  if (loading && !hasImages) {
    return <LoadingState size={size} className={className} />
  }

  // --- No images ---
  if (!loading && !hasImages) {
    return (
      <div className={cn(dashboardCardBase, 'h-full flex flex-col items-center justify-center gap-3 bg-muted-30', sizeConfig.padding, className)}>
        <ImageOff className={cn('text-muted-foreground', size === 'sm' ? 'h-8 w-8' : size === 'md' ? 'h-12 w-12' : 'h-16 w-16')} />
        <div className="text-center">
          <p className="text-muted-foreground text-sm font-medium">{t('imageHistory.noImages')}</p>
          <p className="text-muted-foreground text-xs mt-1">{t('imageHistory.configureSource')}</p>
        </div>
      </div>
    )
  }

  // --- Main render ---
  return (
    <div className={cn(dashboardCardBase, 'relative flex flex-col overflow-hidden', className)}>
      <div className={cn('w-full flex-1 relative', size === 'sm' ? 'h-[120px]' : size === 'md' ? 'h-[180px]' : 'h-[240px]')}>
        <img
          src={displayImageSrc}
          alt={currentImage?.alt || `Image ${currentIndex + 1}`}
          className={cn(
            'w-full h-full',
            fit === 'contain' && 'object-contain',
            fit === 'cover' && 'object-cover',
            fit === 'fill' && 'object-fill',
            fit === 'none' && 'object-none',
            fit === 'scale-down' && 'object-scale-down',
          )}
          loading="lazy"
          onLoad={handleImageLoad}
          onError={handleImageError}
        />

        {/* Title + index overlay */}
        <div className={cn(
          "absolute left-2 flex items-center gap-2 bg-black/60 text-white text-xs px-2 py-1 rounded z-10",
          (title && showTitle) || canNavigate ? "top-2" : "hidden",
        )}>
          {title && showTitle && (
            <>
              <Images className="h-4 w-4 text-white/90 shrink-0" />
              <span className="font-medium text-xs truncate max-w-[120px] text-white drop-shadow-md">{title}</span>
              {canNavigate && <span className="w-px h-3 bg-white/30" />}
            </>
          )}
          {canNavigate && (
            <span className="tabular-nums">{currentIndex + 1} / {images.length}</span>
          )}
        </div>

        {/* Timestamp overlay */}
        {currentImage?.timestamp && (
          <div className="absolute top-2 right-2 bg-black/60 text-white text-xs px-2 py-1 rounded z-10">
            {formatTimestamp(currentImage.timestamp)}
          </div>
        )}

        {/* Label overlay */}
        {currentImage?.label && (
          <div className="absolute top-9 left-2 bg-black/60 text-white text-xs px-2 py-1 rounded max-w-[150px] truncate z-10">
            {currentImage.label}
          </div>
        )}

        {/* Loading overlay */}
        {imageLoadState === 'loading' && (
          <div className="absolute inset-0 flex items-center justify-center bg-bg-50 z-20">
            <RefreshCw className="h-6 w-6 text-muted-foreground animate-spin" />
          </div>
        )}

        {/* Error overlay */}
        {imageLoadState === 'error' && (
          <div className="absolute inset-0 flex flex-col items-center justify-center bg-bg-80 z-20">
            <AlertTriangle className="h-8 w-8 text-destructive mb-2" />
            <span className="text-sm text-muted-foreground">{t('imageHistory.failedToLoad')}</span>
          </div>
        )}

        {/* Slider */}
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
})
