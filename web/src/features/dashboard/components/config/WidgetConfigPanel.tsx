/**
 * WidgetConfigPanel — slide-out panel for configuring a widget
 */

import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { useDashboardStore } from '../../store'
import { DataSourceSelector } from './DataSourceSelector'
import { DisplayOptions } from './DisplayOptions'
import { ActionConfigEditor } from './ActionConfig'
import type { DashboardComponent, DataSourceOrList, DisplayConfig, ActionConfig, GenericComponent } from '@/types/dashboard'
import { isGenericComponent } from '@/types/dashboard'

export function WidgetConfigPanel() {
  const { t } = useTranslation()
  const configPanelOpen = useDashboardStore((s) => s.configPanelOpen)
  const configComponentId = useDashboardStore((s) => s.configComponentId)
  const currentDashboard = useDashboardStore((s) => s.currentDashboard)
  const setConfigPanelOpen = useDashboardStore((s) => s.setConfigPanelOpen)
  const updateComponent = useDashboardStore((s) => s.updateComponent)

  const component: DashboardComponent | undefined = useMemo(() => {
    if (!configComponentId || !currentDashboard?.components) return undefined
    return currentDashboard.components.find((c) => c.id === configComponentId)
  }, [configComponentId, currentDashboard?.components])

  const handleClose = useCallback(() => {
    setConfigPanelOpen(false)
  }, [setConfigPanelOpen])

  const handleTitleChange = useCallback(
    (title: string) => {
      if (!configComponentId) return
      updateComponent(configComponentId, { title })
    },
    [configComponentId, updateComponent]
  )

  const handleDataSourceChange = useCallback(
    (dataSource: DataSourceOrList) => {
      if (!configComponentId) return
      updateComponent(configComponentId, { dataSource })
    },
    [configComponentId, updateComponent]
  )

  const handleDisplayChange = useCallback(
    (display: DisplayConfig) => {
      if (!configComponentId) return
      updateComponent(configComponentId, { display })
    },
    [configComponentId, updateComponent]
  )

  const handleActionsChange = useCallback(
    (actions: ActionConfig[]) => {
      if (!configComponentId) return
      updateComponent(configComponentId, { actions })
    },
    [configComponentId, updateComponent]
  )

  if (!configPanelOpen || !component) return null

  const isGeneric = isGenericComponent(component)

  return (
    <div
      className={cn(
        'absolute top-0 right-0 h-full w-80 bg-card border-l border-border',
        'flex flex-col z-30 shadow-lg',
        'animate-in slide-in-from-right duration-200'
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <span className="text-sm font-medium truncate">
          {t('dashboard.configureWidget', 'Configure Widget')}
        </span>
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleClose}>
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto p-3 space-y-4">
        {/* Title */}
        <div className="space-y-1.5">
          <Label className="text-xs">{t('dashboard.title', 'Title')}</Label>
          <Input
            value={component.title ?? ''}
            onChange={(e) => handleTitleChange(e.target.value)}
            placeholder="Widget title"
          />
        </div>

        {/* Data Source */}
        <div className="space-y-1.5">
          <Label className="text-xs font-medium">
            {t('dashboard.dataSource', 'Data Source')}
          </Label>
          <DataSourceSelector
            value={Array.isArray(component.dataSource) ? component.dataSource[0] : component.dataSource}
            onChange={handleDataSourceChange}
          />
        </div>

        {/* Display Options (generic components only) */}
        {isGeneric && (
          <div className="space-y-1.5">
            <Label className="text-xs font-medium">
              {t('dashboard.display', 'Display')}
            </Label>
            <DisplayOptions
              value={(component as GenericComponent).display}
              onChange={handleDisplayChange}
            />
          </div>
        )}

        {/* Actions (generic components only) */}
        {isGeneric && (
          <div className="space-y-1.5">
            <Label className="text-xs font-medium">
              {t('dashboard.actions', 'Actions')}
            </Label>
            <ActionConfigEditor
              value={(component as GenericComponent).actions}
              onChange={handleActionsChange}
            />
          </div>
        )}
      </div>
    </div>
  )
}
