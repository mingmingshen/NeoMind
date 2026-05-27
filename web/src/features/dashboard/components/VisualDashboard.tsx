/**
 * VisualDashboard — Main page orchestrator for the new dashboard feature module.
 *
 * Renders the dashboard grid, event bridge, config panel, and widget library sidebar.
 * Uses the new Zustand store and TanStack Query hooks.
 */

import { useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router-dom'
import { Pencil, Check, Plus, LayoutDashboard } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useDashboardStore } from '../store'
import { DashboardGrid } from './DashboardGrid'
import { WidgetConfigPanel } from './config/WidgetConfigPanel'
import { InstallWidgetDialog } from './InstallWidgetDialog'
import { WidgetShell } from './WidgetShell'
import { getWidgetComponent } from '../widgets/adapters'
import { useWidgetDataSource } from '../hooks/useWidgetDataSource'

export function VisualDashboard() {
  const { t } = useTranslation()
  const { dashboardId } = useParams<{ dashboardId?: string }>()

  // Store selectors
  const currentDashboard = useDashboardStore((s) => s.currentDashboard)
  const editMode = useDashboardStore((s) => s.editMode)
  const componentLibraryOpen = useDashboardStore((s) => s.componentLibraryOpen)
  const setEditMode = useDashboardStore((s) => s.setEditMode)
  const setComponentLibraryOpen = useDashboardStore((s) => s.setComponentLibraryOpen)
  const setCurrentDashboard = useDashboardStore((s) => s.setCurrentDashboard)
  const fetchDashboards = useDashboardStore((s) => s.fetchDashboards)
  const setConfigPanelOpen = useDashboardStore((s) => s.setConfigPanelOpen)

  // Load dashboards on mount
  useEffect(() => {
    fetchDashboards()
  }, [fetchDashboards])

  // Select dashboard when route changes
  useEffect(() => {
    if (dashboardId) {
      setCurrentDashboard(dashboardId)
    }
  }, [dashboardId, setCurrentDashboard])

  const handleToggleEdit = useCallback(() => {
    setEditMode(!editMode)
  }, [editMode, setEditMode])

  const handleOpenLibrary = useCallback(() => {
    setComponentLibraryOpen(true)
  }, [setComponentLibraryOpen])

  // Empty state
  if (!currentDashboard) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-3">
        <LayoutDashboard className="h-10 w-10 opacity-40" />
        <p className="text-sm">{t('dashboard.noDashboard', 'No dashboard selected')}</p>
      </div>
    )
  }

  return (
    <div className="relative flex h-full overflow-hidden">
      {/* Main content area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Header bar */}
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border shrink-0">
          <h2 className="text-sm font-medium truncate flex-1">
            {currentDashboard.name}
          </h2>

          {editMode && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs gap-1"
              onClick={handleOpenLibrary}
            >
              <Plus className="h-3.5 w-3.5" />
              {t('dashboard.addWidget', 'Add Widget')}
            </Button>
          )}

          <Button
            variant={editMode ? 'default' : 'outline'}
            size="sm"
            className="h-7 text-xs gap-1"
            onClick={handleToggleEdit}
          >
            {editMode ? (
              <>
                <Check className="h-3.5 w-3.5" />
                {t('dashboard.done', 'Done')}
              </>
            ) : (
              <>
                <Pencil className="h-3.5 w-3.5" />
                {t('dashboard.editDashboard', 'Edit Dashboard')}
              </>
            )}
          </Button>
        </div>

        {/* Dashboard Grid */}
        <div className="flex-1 overflow-auto p-2">
          <DashboardGrid
            components={currentDashboard.components}
            editMode={editMode}
          >
            {currentDashboard.components.map((component) => {
              const WidgetComponent = getWidgetComponent(component.type)
              return (
                <div key={component.id}>
                  <WidgetShell
                    widgetId={component.id}
                    title={component.title}
                    isEditing={editMode}
                    onOpenConfig={() => setConfigPanelOpen(true, component.id)}
                  >
                    {WidgetComponent ? (
                      <WidgetComponent
                        widgetId={component.id}
                        dataSource={null}
                        isEditing={editMode}
                        title={component.title}
                      />
                    ) : (
                      <div className="flex items-center justify-center h-full text-muted-foreground text-xs">
                        {component.type}
                      </div>
                    )}
                  </WidgetShell>
                </div>
              )
            })}
          </DashboardGrid>
        </div>
      </div>

      {/* Widget Library Sidebar (left) */}
      {componentLibraryOpen && (
        <div
          className={cn(
            'absolute top-0 left-0 h-full w-72 bg-card border-r border-border',
            'flex flex-col z-30 shadow-lg',
            'animate-in slide-in-from-left duration-200'
          )}
        >
          <div className="flex items-center justify-between px-3 py-2 border-b border-border">
            <span className="text-sm font-medium">
              {t('dashboard.widgetLibrary', 'Widget Library')}
            </span>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 text-xs"
              onClick={() => setComponentLibraryOpen(false)}
            >
              {t('common.close', 'Close')}
            </Button>
          </div>
          <InstallWidgetDialog />
        </div>
      )}

      {/* Config Panel (right) */}
      <WidgetConfigPanel />
    </div>
  )
}
