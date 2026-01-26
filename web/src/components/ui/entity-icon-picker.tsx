/**
 * Entity Icon Picker Component
 *
 * Icon picker for dashboard entity icons using the project's icon system.
 * Shows icon previews and allows searching.
 */

import { useState, useMemo } from 'react'
import { Search, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Field } from '@/components/ui/field'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import { entityIcons, getIconForEntity } from '@/design-system/icons'
import type { EntityIcon } from '@/design-system/icons'

// Build icon list from entityIcons
const ALL_ENTITY_ICONS = [
  { id: '', name: '无图标' },
  ...Object.keys(entityIcons).map(key => ({
    id: key,
    name: key.charAt(0).toUpperCase() + key.slice(1).replace(/([A-Z])/g, ' $1').trim(),
  }))
]

export interface EntityIconPickerProps {
  value?: string
  onChange?: (iconName: string) => void
  label?: string
  disabled?: boolean
  className?: string
}

export function EntityIconPicker({
  value = '',
  onChange,
  label,
  disabled = false,
  className,
}: EntityIconPickerProps) {
  const [searchQuery, setSearchQuery] = useState('')

  // Filter icons by search query
  const filteredIcons = useMemo(() => {
    if (!searchQuery) return ALL_ENTITY_ICONS
    const query = searchQuery.toLowerCase()
    return ALL_ENTITY_ICONS.filter(icon =>
      icon.name.toLowerCase().includes(query) ||
      (icon.id && icon.id.toLowerCase().includes(query))
    )
  }, [searchQuery])

  // Get icon component for preview
  const IconPreview = ({ iconName, size = 16 }: { iconName: string; size?: number }) => {
    if (!iconName) {
      return (
        <div className="flex items-center justify-center text-muted-foreground" style={{ width: size, height: size }}>
          <span className="text-xs">—</span>
        </div>
      )
    }
    try {
      const IconComponent = getIconForEntity(iconName)
      return <IconComponent style={{ width: size, height: size }} />
    } catch {
      return null
    }
  }

  const handleSelectIcon = (iconName: string) => {
    onChange?.(iconName)
  }

  const handleClear = () => {
    onChange?.('')
  }

  // Get current icon name for display
  const currentIconName = useMemo(() => {
    if (!value) return '无图标'
    const icon = ALL_ENTITY_ICONS.find(i => i.id === value)
    return icon?.name || value
  }, [value])

  return (
    <Field className={className}>
      {label && <Label>{label}</Label>}
      <Popover>
        <PopoverTrigger asChild disabled={disabled}>
          <Button
            variant="outline"
            className="h-10 w-full justify-start font-normal"
          >
            <div className="flex items-center gap-2">
              <IconPreview iconName={value} size={16} />
              <span className="text-sm truncate">{currentIconName}</span>
            </div>
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-80 p-0" align="start">
          {/* Search */}
          <div className="p-3 border-b">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="搜索图标..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9 h-9"
              />
            </div>
          </div>

          {/* Icon grid */}
          <div className="p-3 max-h-80 overflow-y-auto scrollbar-thin scrollbar-thumb-muted-foreground/20 scrollbar-track-transparent hover:scrollbar-thumb-muted-foreground/40">
            {filteredIcons.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground text-sm">
                未找到匹配的图标
              </div>
            ) : (
              <div className="grid grid-cols-6 gap-1.5">
                {filteredIcons.map((icon) => {
                  const isSelected = value === icon.id
                  return (
                    <button
                      key={icon.id}
                      type="button"
                      onClick={() => handleSelectIcon(icon.id)}
                      disabled={disabled}
                      className={cn(
                        'flex flex-col items-center gap-1 p-2 rounded-md transition-colors',
                        'hover:bg-accent',
                        isSelected
                          ? 'bg-primary text-primary-foreground'
                          : 'hover:text-accent-foreground'
                      )}
                      title={icon.name}
                    >
                      <IconPreview iconName={icon.id} size={18} />
                      <span className="text-[10px] truncate w-full text-center">{icon.name}</span>
                    </button>
                  )
                })}
              </div>
            )}
          </div>

          {/* Footer */}
          {value && (
            <div className="p-2 border-t flex justify-between items-center">
              <span className="text-xs text-muted-foreground truncate max-w-[150px]">
                {currentIconName}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleClear}
                className="h-7 px-2 text-xs"
              >
                <X className="h-3 w-3 mr-1" />
                清除
              </Button>
            </div>
          )}
        </PopoverContent>
      </Popover>
    </Field>
  )
}
