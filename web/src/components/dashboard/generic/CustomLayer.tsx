/**
 * Custom Layer Component
 *
 * A free-form container component that can hold other components.
 * Supports placing devices, metrics, commands, and other content at custom positions.
 * Can be used as a background layer with overlaid interactive elements.
 */

import { useState, useCallback, useRef, useEffect } from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'
import { useDataSource } from '@/hooks/useDataSource'
import {
  Layers,
  Plus,
  Settings,
  Lock,
  Unlock,
  Eye,
  EyeOff,
  GripVertical,
  Maximize2,
  Minimize2,
  Image as ImageIcon,
  Square,
} from 'lucide-react'
import type { DataSource } from '@/types/dashboard'

// ============================================================================
// Types
// ============================================================================

export interface LayerItem {
  id: string
  type: 'device' | 'metric' | 'command' | 'text' | 'icon' | 'component'
  position: { x: number; y: number } // Relative position (0-100%)
  size?: { width: number; height: number }

  // Content
  label?: string
  value?: string | number
  icon?: string
  deviceId?: string
  metricId?: string
  command?: string
  commandParams?: Record<string, unknown>

  // Styling
  color?: string
  backgroundColor?: string
  borderColor?: string
  fontSize?: 'xs' | 'sm' | 'md' | 'lg'
  fontWeight?: 'normal' | 'medium' | 'semibold' | 'bold'

  // State
  visible?: boolean
  locked?: boolean
  draggable?: boolean

  // Actions
  onClick?: () => void
  onValueChange?: (value: unknown) => void
}

export interface CustomLayerProps {
  dataSource?: DataSource
  items?: LayerItem[]

  // Layer options
  backgroundType?: 'color' | 'image' | 'transparent' | 'grid'
  backgroundColor?: string
  backgroundImage?: string
  gridSize?: number

  // Display options
  size?: 'sm' | 'md' | 'lg'
  interactive?: boolean
  showControls?: boolean
  editable?: boolean
  showFullscreen?: boolean

  // Scale options
  maintainAspectRatio?: boolean
  aspectRatio?: number

  className?: string
}

// ============================================================================
// Layer Item Component
// ============================================================================

interface LayerItemComponentProps {
  item: LayerItem
  isEditing: boolean
  isSelected: boolean
  onSelect: () => void
  onDrag: (item: LayerItem, newPosition: { x: number; y: number }) => void
  onToggleVisibility: (id: string) => void
  onToggleLock: (id: string) => void
}

function LayerItemComponent({
  item,
  isEditing,
  isSelected,
  onSelect,
  onDrag,
  onToggleVisibility,
  onToggleLock,
}: LayerItemComponentProps) {
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const [position, setPosition] = useState(item.position)
  const itemRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setPosition(item.position)
  }, [item.position])

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!isEditing || item.locked || !item.draggable) return
    e.stopPropagation()
    setIsDragging(true)
    setDragStart({ x: e.clientX - position.x, y: e.clientY - position.y })
    onSelect()
  }, [isEditing, item.locked, item.draggable, position, onSelect])

  useEffect(() => {
    if (!isDragging) return

    const handleMouseMove = (e: MouseEvent) => {
      // Convert pixel delta to percentage
      if (itemRef.current?.parentElement) {
        const parent = itemRef.current.parentElement
        const newX = ((e.clientX - dragStart.x) / parent.offsetWidth) * 100
        const newY = ((e.clientY - dragStart.y) / parent.offsetHeight) * 100
        const clampedX = Math.max(0, Math.min(95, newX))
        const clampedY = Math.max(0, Math.min(95, newY))
        setPosition({ x: clampedX, y: clampedY })
      }
    }

    const handleMouseUp = () => {
      setIsDragging(false)
      onDrag(item, position)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [isDragging, dragStart, item, position, onDrag])

  const getFontSizeClass = () => {
    switch (item.fontSize) {
      case 'xs': return 'text-xs'
      case 'sm': return 'text-sm'
      case 'md': return 'text-base'
      case 'lg': return 'text-lg'
      default: return 'text-sm'
    }
  }

  const getFontWeightClass = () => {
    switch (item.fontWeight) {
      case 'normal': return 'font-normal'
      case 'medium': return 'font-medium'
      case 'semibold': return 'font-semibold'
      case 'bold': return 'font-bold'
      default: return 'font-medium'
    }
  }

  if (item.visible !== true) {
    return null
  }

  return (
    <div
      ref={itemRef}
      className={cn(
        'absolute transform -translate-x-1/2 -translate-y-1/2',
        isEditing && !item.locked && 'cursor-move',
        isDragging && 'cursor-grabbing',
        isSelected && 'ring-2 ring-primary ring-offset-2'
      )}
      style={{
        left: `${position.x}%`,
        top: `${position.y}%`,
        width: item.size?.width ? `${item.size.width}px` : 'auto',
        height: item.size?.height ? `${item.size.height}px` : 'auto',
      }}
      onMouseDown={handleMouseDown}
      onClick={(e) => {
        if (!isDragging && item.onClick) {
          e.stopPropagation()
          item.onClick()
        }
      }}
    >
      {/* Content based on type */}
      <div
        className={cn(
          'inline-flex items-center gap-1.5 px-2 py-1 rounded shadow-sm',
          getFontSizeClass(),
          getFontWeightClass()
        )}
        style={{
          backgroundColor: item.backgroundColor,
          borderColor: item.borderColor,
          color: item.color,
        }}
      >
        {item.icon && (
          <span className="flex-shrink-0">{item.icon}</span>
        )}
        {item.label && (
          <span>{item.label}</span>
        )}
        {item.value !== undefined && (
          <span className={cn(
            'tabular-nums',
            item.type === 'metric' && 'font-semibold'
          )}>
            {item.value}
          </span>
        )}
      </div>

      {/* Edit controls */}
      {isEditing && isSelected && (
        <div className="absolute -top-6 left-1/2 -translate-x-1/2 flex items-center gap-1">
          <Button
            variant="secondary"
            size="icon"
            className="h-5 w-5 bg-background/90"
            onClick={(e) => { e.stopPropagation(); onToggleVisibility(item.id); }}
          >
            {((item.visible as boolean | undefined) !== true) ? <EyeOff className="h-2.5 w-2.5" /> : <Eye className="h-2.5 w-2.5" />}
          </Button>
          <Button
            variant="secondary"
            size="icon"
            className="h-5 w-5 bg-background/90"
            onClick={(e) => { e.stopPropagation(); onToggleLock(item.id); }}
          >
            {item.locked ? <Lock className="h-2.5 w-2.5" /> : <Unlock className="h-2.5 w-2.5" />}
          </Button>
        </div>
      )}

      {/* Drag handle */}
      {isEditing && (item.locked !== true) && (
        <div className="absolute -left-6 top-1/2 -translate-y-1/2 opacity-0 hover:opacity-100 group-hover:opacity-100">
          <GripVertical className="h-4 w-4 text-muted-foreground" />
        </div>
      )}
    </div>
  )
}

// ============================================================================
// Background Component
// ============================================================================

interface LayerBackgroundProps {
  type: 'color' | 'image' | 'transparent' | 'grid'
  color?: string
  image?: string
  gridSize?: number
}

function LayerBackground({ type, color, image, gridSize = 20 }: LayerBackgroundProps) {
  const backgroundStyle: React.CSSProperties = {}

  switch (type) {
    case 'color':
      backgroundStyle.backgroundColor = color || 'hsl(var(--muted) / 0.3)'
      break
    case 'image':
      backgroundStyle.backgroundImage = image ? `url(${image})` : undefined
      backgroundStyle.backgroundSize = 'cover'
      backgroundStyle.backgroundPosition = 'center'
      break
    case 'transparent':
      // Chessboard pattern for transparency
      backgroundStyle.backgroundImage = `
        linear-gradient(45deg, hsl(var(--muted) / 0.1) 25%, transparent 25%),
        linear-gradient(-45deg, hsl(var(--muted) / 0.1) 25%, transparent 25%),
        linear-gradient(45deg, transparent 75%, hsl(var(--muted) / 0.1) 75%),
        linear-gradient(-45deg, transparent 75%, hsl(var(--muted) / 0.1) 75%)
      `
      backgroundStyle.backgroundSize = '20px 20px'
      backgroundStyle.backgroundPosition = '0 0, 0 10px, 10px -10px, -10px 0px'
      break
    case 'grid':
      backgroundStyle.backgroundColor = color || 'hsl(var(--background))'
      backgroundStyle.backgroundImage = `
        linear-gradient(to right, hsl(var(--border)) 1px, transparent 1px),
        linear-gradient(to bottom, hsl(var(--border)) 1px, transparent 1px)
      `
      backgroundStyle.backgroundSize = `${gridSize}px ${gridSize}px`
      break
  }

  return <div className="absolute inset-0 -z-10" style={backgroundStyle} />
}

// ============================================================================
// Main Component
// ============================================================================

export function CustomLayer({
  dataSource,
  items: propItems = [],
  backgroundType = 'grid',
  backgroundColor,
  backgroundImage,
  gridSize = 20,
  size = 'md',
  interactive = true,
  showControls = true,
  editable = false,
  showFullscreen = true,
  maintainAspectRatio,
  aspectRatio,
  className,
}: CustomLayerProps) {
  const { data, loading, error } = useDataSource<LayerItem[]>(dataSource, {
    fallback: propItems,
  })

  const items = error ? propItems : (data ?? propItems)
  const [isEditing, setIsEditing] = useState(editable)
  const [selectedItem, setSelectedItem] = useState<string | null>(null)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [internalItems, setInternalItems] = useState<LayerItem[]>(items)

  useEffect(() => {
    setInternalItems(items)
  }, [items])

  const sizeConfig = dashboardComponentSize[size]

  const handleItemDrag = useCallback((item: LayerItem, newPosition: { x: number; y: number }) => {
    setInternalItems(prev =>
      prev.map(i => i.id === item.id ? { ...i, position: newPosition } : i)
    )
  }, [])

  const handleToggleVisibility = useCallback((id: string) => {
    setInternalItems(prev =>
      prev.map(i => i.id === id ? { ...i, visible: i.visible === true ? false : true } : i)
    )
  }, [])

  const handleToggleLock = useCallback((id: string) => {
    setInternalItems(prev =>
      prev.map(i => i.id === id ? { ...i, locked: !i.locked } : i)
    )
  }, [])

  const handleAddItem = useCallback(() => {
    const newItem: LayerItem = {
      id: `item-${Date.now()}`,
      type: 'text',
      position: { x: 50, y: 50 },
      label: 'New Item',
      value: '0',
      visible: true,
      locked: false,
      draggable: true,
    }
    setInternalItems(prev => [...prev, newItem])
    setSelectedItem(newItem.id)
  }, [])

  // Loading state
  if (loading) {
    return (
      <div className={cn(dashboardCardBase, 'flex items-center justify-center', sizeConfig.padding, className)}>
        <Skeleton className={cn('w-full h-full rounded-lg')} />
      </div>
    )
  }

  const content = (
    <div className={cn(dashboardCardBase, 'relative overflow-hidden', className)}>
      {/* Header with controls */}
      {showControls && (
        <div className="flex items-center justify-between px-3 py-2 border-b bg-muted/20">
          <div className="flex items-center gap-2">
            <Layers className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Custom Layer</span>
            <span className="text-xs text-muted-foreground">({items.length} items)</span>
          </div>
          <div className="flex items-center gap-1">
            {editable && (
              <Button
                variant="ghost"
                size="icon"
                className={cn('h-7 w-7', isEditing && 'bg-accent')}
                onClick={() => setIsEditing(!isEditing)}
              >
                <Settings className="h-3.5 w-3.5" />
              </Button>
            )}
            {isEditing && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={handleAddItem}
              >
                <Plus className="h-3.5 w-3.5" />
              </Button>
            )}
            {showFullscreen && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => setIsFullscreen(!isFullscreen)}
              >
                {isFullscreen ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Layer canvas */}
      <div
        className={cn(
          'relative bg-background overflow-hidden',
          isFullscreen ? 'fixed inset-0 z-50' : 'min-h-[300px]'
        )}
        style={
          maintainAspectRatio && aspectRatio
            ? { aspectRatio: `${aspectRatio}` }
            : undefined
        }
        onClick={() => setSelectedItem(null)}
      >
        <LayerBackground
          type={backgroundType}
          color={backgroundColor}
          image={backgroundImage}
          gridSize={gridSize}
        />

        {/* Render items */}
        {internalItems.map((item) => (
          <LayerItemComponent
            key={item.id}
            item={item}
            isEditing={isEditing}
            isSelected={selectedItem === item.id}
            onSelect={() => setSelectedItem(item.id)}
            onDrag={handleItemDrag}
            onToggleVisibility={handleToggleVisibility}
            onToggleLock={handleToggleLock}
          />
        ))}

        {/* Empty state */}
        {internalItems.length === 0 && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-muted-foreground/60">
            <Square className="h-12 w-12" />
            <div className="text-center">
              <p className="text-sm">Empty Layer</p>
              <p className="text-xs mt-1">Add items to this layer</p>
            </div>
            {isEditing && (
              <Button variant="outline" size="sm" onClick={handleAddItem}>
                <Plus className="h-3.5 w-3.5 mr-1.5" />
                Add Item
              </Button>
            )}
          </div>
        )}

        {/* Edit mode indicator */}
        {isEditing && (
          <div className="absolute top-2 left-2 px-2 py-1 bg-accent text-accent-foreground rounded text-xs font-medium">
            Edit Mode
          </div>
        )}
      </div>
    </div>
  )

  return content
}
