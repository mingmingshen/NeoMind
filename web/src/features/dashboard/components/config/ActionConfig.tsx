/**
 * ActionConfig — widget action configuration
 */

import { useCallback } from 'react'
import { Plus, Trash2 } from 'lucide-react'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { ActionConfig, ActionType } from '../../types'

interface ActionConfigProps {
  value: ActionConfig[] | undefined
  onChange: (actions: ActionConfig[]) => void
}

export function ActionConfigEditor({ value, onChange }: ActionConfigProps) {
  const actions = value ?? []

  const addAction = useCallback(() => {
    onChange([...actions, { type: 'api-call' }])
  }, [actions, onChange])

  const removeAction = useCallback((index: number) => {
    onChange(actions.filter((_, i) => i !== index))
  }, [actions, onChange])

  const updateAction = useCallback((index: number, updates: Partial<ActionConfig>) => {
    onChange(actions.map((a, i) => i === index ? { ...a, ...updates } : a))
  }, [actions, onChange])

  return (
    <div className="space-y-2">
      {actions.map((action, i) => (
        <div key={i} className="flex gap-2 items-start p-2 rounded-md border border-border bg-muted/30">
          <div className="flex-1 space-y-2">
            <Select
              value={action.type}
              onValueChange={(v) => updateAction(i, { type: v as ActionType })}
            >
              <SelectTrigger className="h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="api-call">API Call</SelectItem>
                <SelectItem value="navigate">Navigate</SelectItem>
                <SelectItem value="dialog">Dialog</SelectItem>
              </SelectContent>
            </Select>
            {action.type === 'api-call' && (
              <Input
                className="h-7 text-xs"
                value={action.endpoint ?? ''}
                onChange={(e) => updateAction(i, { endpoint: e.target.value })}
                placeholder="Endpoint"
              />
            )}
            {action.type === 'navigate' && (
              <Input
                className="h-7 text-xs"
                value={action.path ?? ''}
                onChange={(e) => updateAction(i, { path: e.target.value })}
                placeholder="Path"
              />
            )}
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
            onClick={() => removeAction(i)}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>
      ))}
      <Button variant="outline" size="sm" className="w-full h-7 text-xs" onClick={addAction}>
        <Plus className="h-3.5 w-3.5 mr-1" />
        Add Action
      </Button>
    </div>
  )
}
