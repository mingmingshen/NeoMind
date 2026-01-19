import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Badge } from '@/components/ui/badge'
import { Trash2, GripVertical, Zap, Bell, FileText, Plus, X } from 'lucide-react'
import type { RuleAction } from '@/types'

interface ActionBuilderProps {
  action: RuleAction
  onChange: (action: RuleAction) => void
  onRemove: () => void
  resources?: {
    devices: Array<{ id: string; name: string; type: string }>
    metrics: string[]
    alertChannels: Array<{ id: string; name: string }>
  }
  canRemove?: boolean
}

const COMMON_COMMANDS = [
  'turn_on',
  'turn_off',
  'toggle',
  'set_value',
  'set_brightness',
  'set_color',
  'set_temperature',
  'open',
  'close',
  'start',
  'stop',
]

interface ActionParameter {
  key: string
  value: string
}

export function ActionBuilder({
  action,
  onChange,
  onRemove,
  resources,
  canRemove = true,
}: ActionBuilderProps) {
  const { t } = useTranslation('automation')
  const [params, setParams] = useState<ActionParameter[]>(
    action.type === 'Execute' && action.params
      ? Object.entries(action.params).map(([key, value]) => ({ key, value: String(value) }))
      : []
  )

  const devices = resources?.devices || []

  const handleAddParam = () => {
    setParams([...params, { key: '', value: '' }])
  }

  const handleUpdateParam = (index: number, updates: Partial<ActionParameter>) => {
    const newParams = [...params]
    newParams[index] = { ...newParams[index], ...updates }
    setParams(newParams)

    // Update action params
    if (action.type === 'Execute') {
      const paramsObj: Record<string, unknown> = {}
      newParams.forEach((p) => {
        if (p.key) {
          paramsObj[p.key] = p.value
        }
      })
      onChange({ ...action, params: paramsObj } as RuleAction)
    }
  }

  const handleRemoveParam = (index: number) => {
    const newParams = params.filter((_, i) => i !== index)
    setParams(newParams)

    // Update action params
    if (action.type === 'Execute') {
      const paramsObj: Record<string, unknown> = {}
      newParams.forEach((p) => {
        if (p.key) {
          paramsObj[p.key] = p.value
        }
      })
      onChange({ ...action, params: paramsObj } as RuleAction)
    }
  }

  const getActionIcon = () => {
    switch (action.type) {
      case 'Execute':
        return <Zap className="h-4 w-4" />
      case 'Notify':
        return <Bell className="h-4 w-4" />
      case 'Log':
        return <FileText className="h-4 w-4" />
    }
  }

  const getActionTypeLabel = () => {
    switch (action.type) {
      case 'Execute':
        return t('actionTypeExecute')
      case 'Notify':
        return t('actionTypeNotify')
      case 'Log':
        return t('actionTypeLog')
    }
  }

  return (
    <Card className="p-3">
      <div className="flex items-start gap-3">
        {/* Drag Handle */}
        <div className="flex items-center gap-1 pt-1">
          <GripVertical className="h-4 w-4 text-muted-foreground cursor-grab" />
        </div>

        {/* Action Icon & Type */}
        <div className="flex items-center gap-2 pt-1">
          <div className="flex items-center justify-center w-8 h-8 rounded-md bg-primary/10 text-primary">
            {getActionIcon()}
          </div>
          <Badge variant="outline">{getActionTypeLabel()}</Badge>
        </div>

        {/* Action Configuration */}
        <div className="flex-1">
          {action.type === 'Execute' && (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {/* Device Selector */}
              <div>
                <Label className="text-xs text-muted-foreground">{t('automation:device')}</Label>
                <Select
                  value={action.device_id}
                  onValueChange={(value) => onChange({ ...action, device_id: value } as RuleAction)}
                >
                  <SelectTrigger className="h-9">
                    <SelectValue placeholder={t('automation:selectDevice')} />
                  </SelectTrigger>
                  <SelectContent>
                    {devices.map((device) => (
                      <SelectItem key={device.id} value={device.id}>
                        {device.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Command Selector */}
              <div>
                <Label className="text-xs text-muted-foreground">{t('automation:command')}</Label>
                <Select
                  value={action.command}
                  onValueChange={(value) => onChange({ ...action, command: value } as RuleAction)}
                >
                  <SelectTrigger className="h-9">
                    <SelectValue placeholder={t('automation:selectCommand')} />
                  </SelectTrigger>
                  <SelectContent>
                    {COMMON_COMMANDS.map((cmd) => (
                      <SelectItem key={cmd} value={cmd}>
                        {cmd}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Parameters Button */}
              <div className="flex items-end">
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={handleAddParam}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  {t('automation:parameters')} ({params.length})
                </Button>
              </div>
            </div>
          )}

          {(action.type === 'Notify' || action.type === 'Log') && (
            <div className="space-y-3">
              {action.type === 'Log' && (
                <div>
                  <Label className="text-xs text-muted-foreground">{t('automation:logLevel')}</Label>
                  <Select
                    value={(action as { level: string }).level || 'info'}
                    onValueChange={(value) => onChange({ ...action, level: value } as RuleAction)}
                  >
                    <SelectTrigger className="h-9 w-32">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="debug">Debug</SelectItem>
                      <SelectItem value="info">Info</SelectItem>
                      <SelectItem value="warn">Warning</SelectItem>
                      <SelectItem value="error">Error</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              )}
              <div className="flex-1">
                <Label className="text-xs text-muted-foreground">
                  {action.type === 'Notify' ? t('automation:message') : t('automation:logMessage')}
                </Label>
                <Textarea
                  value={action.message || ''}
                  onChange={(e) => onChange({ ...action, message: e.target.value } as RuleAction)}
                  placeholder={
                    action.type === 'Notify'
                      ? t('automation:notificationMessagePlaceholder')
                      : t('automation:logMessagePlaceholder')
                  }
                  className="min-h-[60px] text-sm"
                />
              </div>
            </div>
          )}

          {/* Parameters Display for Execute actions */}
          {action.type === 'Execute' && params.length > 0 && (
            <div className="mt-3 pt-3 border-t">
              <Label className="text-xs text-muted-foreground mb-2 block">{t('automation:parameters')}</Label>
              <div className="flex flex-wrap gap-2">
                {params.map(
                  (param, index) =>
                    param.key && (
                      <Badge key={index} variant="secondary" className="gap-1">
                        {param.key}: {param.value}
                        <X
                          className="h-3 w-3 cursor-pointer hover:text-destructive"
                          onClick={() => handleRemoveParam(index)}
                        />
                      </Badge>
                    )
                )}
              </div>
            </div>
          )}

          {/* Parameter Editor Panel */}
          {action.type === 'Execute' && (
            <div className="mt-3 pt-3 border-t">
              <div className="flex items-center justify-between mb-2">
                <Label className="text-xs text-muted-foreground">{t('automation:commandParameters')}</Label>
                <Button size="sm" variant="ghost" className="h-6 text-xs" onClick={handleAddParam}>
                  <Plus className="h-3 w-3 mr-1" />
                  {t('automation:addParameter')}
                </Button>
              </div>
              {params.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t('automation:noParameters')}</p>
              ) : (
                <div className="space-y-2">
                  {params.map((param, index) => (
                    <div key={index} className="flex items-center gap-2">
                      <Input
                        placeholder={t('automation:paramName')}
                        value={param.key}
                        onChange={(e) => handleUpdateParam(index, { key: e.target.value })}
                        className="h-8 flex-1 text-sm"
                      />
                      <span className="text-muted-foreground">=</span>
                      <Input
                        placeholder={t('automation:paramValue')}
                        value={param.value}
                        onChange={(e) => handleUpdateParam(index, { value: e.target.value })}
                        className="h-8 flex-1 text-sm"
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-destructive"
                        onClick={() => handleRemoveParam(index)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Remove Button */}
        {canRemove && (
          <div className="pt-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 text-destructive hover:text-destructive"
              onClick={onRemove}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        )}
      </div>
    </Card>
  )
}
