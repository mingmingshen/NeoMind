/**
 * Unified ColorPicker Component
 *
 * Color picker with preset palette, custom color input, and transparency support.
 * Follows system UI standards.
 */

import { useState } from 'react'
import { Check, ChevronDown, Palette } from 'lucide-react'
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

// Preset color palettes
export const COLOR_PRESETS = {
  primary: [
    '#000000', // Black
    '#ffffff', // White
    '#ef4444', // Red
    '#f97316', // Orange
    '#eab308', // Yellow
    '#22c55e', // Green
    '#06b6d4', // Cyan
    '#3b82f6', // Blue
    '#8b5cf6', // Purple
    '#ec4899', // Pink
  ],
  neutral: [
    '#000000', // Black
    '#171717', // Zinc 950
    '#404040', // Zinc 700
    '#737373', // Zinc 500
    '#a3a3a3', // Zinc 400
    '#d4d4d4', // Zinc 300
    '#e5e5e5', // Zinc 200
    '#f5f5f5', // Zinc 100
    '#ffffff', // White
  ],
  semantic: [
    '#22c55e', // Success
    '#eab308', // Warning
    '#ef4444', // Error/Danger
    '#3b82f6', // Info
    '#8b5cf6', // Purple
  ],
}

export interface ColorPickerProps {
  value?: string
  onChange?: (color: string) => void
  label?: string
  presets?: keyof typeof COLOR_PRESETS | 'all' | 'none'
  showAlpha?: boolean
  disabled?: boolean
  className?: string
}

function isValidHexColor(value: string): boolean {
  return /^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/.test(value)
}

function normalizeHexColor(value: string): string {
  if (!value.startsWith('#')) {
    return `#${value}`
  }
  return value
}

export function ColorPicker({
  value = '#3b82f6',
  onChange,
  label,
  presets = 'primary',
  showAlpha = false,
  disabled = false,
  className,
}: ColorPickerProps) {
  const [inputValue, setInputValue] = useState(value)

  const handleInputChange = (newValue: string) => {
    setInputValue(newValue)
    if (isValidHexColor(newValue)) {
      onChange?.(normalizeHexColor(newValue))
    }
  }

  const handlePresetClick = (color: string) => {
    setInputValue(color)
    onChange?.(color)
  }

  // Get presets to show
  const getPresetColors = (): string[] => {
    if (presets === 'all') {
      return [...COLOR_PRESETS.primary, ...COLOR_PRESETS.neutral, ...COLOR_PRESETS.semantic]
    }
    if (presets === 'none') return []
    return COLOR_PRESETS[presets] || COLOR_PRESETS.primary
  }

  const presetColors = getPresetColors()

  return (
    <Field className={className}>
      {label && <Label>{label}</Label>}
      <div className="flex items-center gap-2">
        {/* Color preview + trigger */}
        <Popover>
          <PopoverTrigger asChild disabled={disabled}>
            <Button
              variant="outline"
              className="h-10 w-12 p-0 shrink-0 border-input"
            >
              <div
                className="h-5 w-5 rounded-sm"
                style={{ backgroundColor: value }}
              />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-64 p-3" align="start">
            {/* Preset colors */}
            {presetColors.length > 0 && (
              <div className="mb-3">
                <div className="grid grid-cols-8 gap-1">
                  {presetColors.map((color) => (
                    <button
                      key={color}
                      type="button"
                      onClick={() => handlePresetClick(color)}
                      disabled={disabled}
                      className={cn(
                        "h-6 w-6 rounded-sm border transition-all hover:scale-110",
                        value.toLowerCase() === color.toLowerCase()
                          ? 'ring-2 ring-ring ring-offset-2'
                          : 'border-border'
                      )}
                      style={{ backgroundColor: color }}
                    >
                      {value.toLowerCase() === color.toLowerCase() && (
                        <Check className="h-3 w-3 text-white drop-shadow-md mx-auto" />
                      )}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Custom color input */}
            <div className="flex items-center gap-2 pt-2 border-t">
              <div className="relative flex-1">
                <div className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground">
                  <Palette className="h-4 w-4" />
                </div>
                <Input
                  value={inputValue}
                  onChange={(e) => handleInputChange(e.target.value)}
                  placeholder="#3b82f6"
                  className="pl-8 h-9 font-mono text-sm"
                  disabled={disabled}
                />
              </div>
              <Input
                type="color"
                value={value}
                onChange={(e) => {
                  const newColor = e.target.value
                  setInputValue(newColor)
                  onChange?.(newColor)
                }}
                className="h-9 w-12 p-1 cursor-pointer"
                disabled={disabled}
              />
            </div>
          </PopoverContent>
        </Popover>

        {/* Text input */}
        <Input
          type="text"
          value={inputValue}
          onChange={(e) => handleInputChange(e.target.value)}
          placeholder="#3b82f6"
          className="flex-1 h-10 font-mono text-sm"
          disabled={disabled}
        />
      </div>
    </Field>
  )
}

/**
 * Compact ColorPicker - inline version without popover
 */
export interface CompactColorPickerProps {
  value?: string
  onChange?: (color: string) => void
  presets?: string[]
  disabled?: boolean
  className?: string
}

export function CompactColorPicker({
  value = '#3b82f6',
  onChange,
  presets = COLOR_PRESETS.primary,
  disabled = false,
  className,
}: CompactColorPickerProps) {
  return (
    <div className={cn('flex items-center gap-2', className)}>
      {/* Color input */}
      <Input
        type="color"
        value={value}
        onChange={(e) => onChange?.(e.target.value)}
        className="h-9 w-12 p-1 cursor-pointer"
        disabled={disabled}
      />
      {/* Text input */}
      <Input
        type="text"
        value={value}
        onChange={(e) => {
          const newValue = e.target.value
          if (isValidHexColor(newValue)) {
            onChange?.(normalizeHexColor(newValue))
          }
        }}
        placeholder="#3b82f6"
        className="flex-1 h-9 font-mono text-sm"
        disabled={disabled}
      />
      {/* Presets */}
      <div className="flex gap-1">
        {presets.map((color) => (
          <button
            key={color}
            type="button"
            onClick={() => onChange?.(color)}
            disabled={disabled}
            className={cn(
              'h-7 w-7 rounded-sm border transition-all hover:scale-110 shrink-0',
              value.toLowerCase() === color.toLowerCase()
                ? 'ring-2 ring-ring ring-offset-2'
                : 'border-border'
            )}
            style={{ backgroundColor: color }}
          />
        ))}
      </div>
    </div>
  )
}
