/**
 * DisplayOptions — widget display configuration
 */

import { useCallback } from 'react'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { DisplayConfig, Size } from '../../types'

interface DisplayOptionsProps {
  value: DisplayConfig | undefined
  onChange: (display: DisplayConfig) => void
}

export function DisplayOptions({ value, onChange }: DisplayOptionsProps) {
  const update = useCallback(
    (field: keyof DisplayConfig, v: unknown) => {
      onChange({ ...value, [field]: v } as DisplayConfig)
    },
    [value, onChange]
  )

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-2">
        <div className="space-y-1.5">
          <Label className="text-xs">Unit</Label>
          <Input
            value={value?.unit ?? ''}
            onChange={(e) => update('unit', e.target.value)}
            placeholder="°C, %, etc."
          />
        </div>
        <div className="space-y-1.5">
          <Label className="text-xs">Format</Label>
          <Input
            value={value?.format ?? ''}
            onChange={(e) => update('format', e.target.value)}
            placeholder="e.g., 0.00"
          />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <div className="space-y-1.5">
          <Label className="text-xs">Min</Label>
          <Input
            type="number"
            value={value?.min ?? ''}
            onChange={(e) => update('min', e.target.value ? Number(e.target.value) : undefined)}
          />
        </div>
        <div className="space-y-1.5">
          <Label className="text-xs">Max</Label>
          <Input
            type="number"
            value={value?.max ?? ''}
            onChange={(e) => update('max', e.target.value ? Number(e.target.value) : undefined)}
          />
        </div>
      </div>

      <div className="space-y-1.5">
        <Label className="text-xs">Size</Label>
        <Select value={value?.size ?? 'md'} onValueChange={(v) => update('size', v as Size)}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="sm">Small</SelectItem>
            <SelectItem value="md">Medium</SelectItem>
            <SelectItem value="lg">Large</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-1.5">
        <Label className="text-xs">Color</Label>
        <Input
          value={value?.color ?? ''}
          onChange={(e) => update('color', e.target.value)}
          placeholder="CSS color"
        />
      </div>

      <div className="space-y-1.5">
        <Label className="text-xs">Prefix</Label>
        <Input
          value={value?.prefix ?? ''}
          onChange={(e) => update('prefix', e.target.value)}
        />
      </div>
    </div>
  )
}
