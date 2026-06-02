/**
 * DashboardListSidebar Component
 *
 * Left sidebar for managing multiple dashboards.
 * - Desktop: Fixed sidebar with collapse toggle
 * - Mobile: Slide-out drawer with backdrop
 */

import { useState } from 'react'
import {
  LayoutDashboard,
  Plus,
  Trash2,
  Edit2,
  Check,
  X,
  ChevronLeft,
  MoreHorizontal,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import type { Dashboard } from '@/types/dashboard'
import { confirm } from '@/hooks/use-confirm'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

export interface DashboardListSidebarProps {
  dashboards: Dashboard[]
  currentDashboardId: string | null
  onSwitch: (id: string) => void
  onCreate: (name: string) => void
  onRename: (id: string, name: string) => void
  onDelete: (id: string) => void
  /** Open state: false = collapsed (desktop) or closed (mobile), true = expanded (desktop) or open drawer (mobile) */
  open?: boolean
  /** Open/close handler */
  onOpenChange?: (open: boolean) => void
  /** Is desktop mode (fixed sidebar) vs mobile (drawer) */
  isDesktop?: boolean
  className?: string
}

/** Shared content for both desktop and mobile sidebar layouts */
function DashboardSidebarContent({
  dashboards,
  currentDashboardId,
  onSwitch,
  onCreate,
  onRename,
  onDelete,
  onOpenChange,
  isDesktop,
}: Omit<DashboardListSidebarProps, 'open' | 'className'>) {
  const { t } = useTranslation('dashboardComponents')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editingName, setEditingName] = useState('')
  const [showCreateInput, setShowCreateInput] = useState(false)
  const [newDashboardName, setNewDashboardName] = useState('')

  const handleStartEdit = (dashboard: Dashboard) => {
    setEditingId(dashboard.id)
    setEditingName(dashboard.name)
  }

  const handleSaveEdit = () => {
    if (editingId && editingName.trim()) {
      onRename(editingId, editingName.trim())
    }
    setEditingId(null)
    setEditingName('')
  }

  const handleCancelEdit = () => {
    setEditingId(null)
    setEditingName('')
  }

  const handleCreate = () => {
    if (newDashboardName.trim()) {
      onCreate(newDashboardName.trim())
      setNewDashboardName('')
      setShowCreateInput(false)
    }
  }

  const handleDelete = async (id: string) => {
    const dashboard = dashboards.find(d => d.id === id)
    const name = dashboard?.name || ''
    const confirmed = await confirm({
      title: t('sidebar.deleteTitle'),
      description: t('sidebar.deleteDescription', { name }),
      confirmText: t('sidebar.delete'),
      cancelText: t('common.cancel'),
      variant: 'destructive'
    })
    if (confirmed) {
      onDelete(id)
    }
  }

  const handleSwitch = (id: string) => {
    onSwitch(id)
    if (!isDesktop) {
      onOpenChange?.(false)
    }
  }

  return (
    <>
      {/* Header */}
      <div className="flex items-center justify-between px-3 h-11 border-b border-border">
        <span className="font-medium text-xs text-muted-foreground uppercase tracking-wider">{t('sidebar.title')}</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => onOpenChange?.(false)}
        >
          {isDesktop ? <ChevronLeft className="h-4 w-4" /> : <X className="h-4 w-4" />}
        </Button>
      </div>

      {/* Dashboard List */}
      <div className="flex-1 overflow-y-auto px-2 py-1.5 space-y-0.5">
        {dashboards.map((dashboard) => {
          const isEditing = editingId === dashboard.id
          const isActive = dashboard.id === currentDashboardId

          return isEditing ? (
            <div key={dashboard.id} className="flex items-center gap-1 px-1 py-0.5">
              <Input
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleSaveEdit()
                  if (e.key === 'Escape') handleCancelEdit()
                }}
                className="h-7 text-sm flex-1"
                autoFocus
              />
              <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={handleSaveEdit}>
                <Check className="h-3.5 w-3.5 text-success" />
              </Button>
              <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={handleCancelEdit}>
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          ) : (
            <div
              key={dashboard.id}
              role="button"
              tabIndex={0}
              onClick={() => handleSwitch(dashboard.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  handleSwitch(dashboard.id)
                }
              }}
              className={cn(
                'group flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-colors',
                isActive
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:bg-muted-50 hover:text-foreground'
              )}
            >
              <LayoutDashboard className={cn(
                'h-4 w-4 shrink-0',
                isActive ? 'text-foreground' : 'text-muted-foreground'
              )} />
              <span className={cn(
                'text-sm flex-1 truncate',
                isActive && 'font-medium'
              )}>
                {dashboard.name}
              </span>

              {/* Actions dropdown - visible on hover */}
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <button
                    className={cn(
                      'h-5 w-5 shrink-0 flex items-center justify-center rounded hover:bg-muted-50 transition-opacity',
                      isDesktop ? 'opacity-0 group-hover:opacity-100' : 'opacity-60'
                    )}
                    onClick={(e) => e.stopPropagation()}
                  >
                    <MoreHorizontal className="h-3.5 w-3.5" />
                  </button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-32">
                  <DropdownMenuItem onClick={() => handleStartEdit(dashboard)}>
                    <Edit2 className="h-3.5 w-3.5 mr-2" />
                    {t('sidebar.rename')}
                  </DropdownMenuItem>
                  {dashboards.length > 1 && (
                    <DropdownMenuItem
                      className="text-destructive focus:text-destructive"
                      onClick={() => handleDelete(dashboard.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5 mr-2" />
                      {t('sidebar.delete')}
                    </DropdownMenuItem>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          )
        })}

        {/* Create */}
        {showCreateInput ? (
          <div className="flex items-center gap-1 px-1 py-0.5">
            <Input
              value={newDashboardName}
              onChange={(e) => setNewDashboardName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleCreate()
                if (e.key === 'Escape') {
                  setShowCreateInput(false)
                  setNewDashboardName('')
                }
              }}
              placeholder={t('sidebar.namePlaceholder')}
              className="h-7 text-sm flex-1"
              autoFocus
            />
            <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={handleCreate}>
              <Check className="h-3.5 w-3.5 text-success" />
            </Button>
            <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={() => { setShowCreateInput(false); setNewDashboardName('') }}>
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        ) : (
          <button
            onClick={() => setShowCreateInput(true)}
            className="flex items-center gap-2 px-2 py-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted-50 transition-colors w-full"
          >
            <Plus className="h-4 w-4 shrink-0" />
            <span className="text-sm">{t('sidebar.newDashboard')}</span>
          </button>
        )}
      </div>
    </>
  )
}

export function DashboardListSidebar({
  dashboards,
  currentDashboardId,
  onSwitch,
  onCreate,
  onRename,
  onDelete,
  open = true,
  onOpenChange,
  isDesktop = true,
  className,
}: DashboardListSidebarProps) {
  const contentProps = {
    dashboards,
    currentDashboardId,
    onSwitch,
    onCreate,
    onRename,
    onDelete,
    onOpenChange,
    isDesktop,
  }

  // Desktop mode: fixed sidebar with collapse
  if (isDesktop) {
    return (
      <div
        className={cn(
          'flex-shrink-0 flex flex-col bg-card transition-[width] duration-300 ease-out',
          !open ? 'w-0 overflow-hidden border-r-0' : 'w-64 border-r border-border',
          className
        )}
      >
        <DashboardSidebarContent {...contentProps} />
      </div>
    )
  }

  // Mobile mode: drawer with backdrop
  // Position below the fixed TopNav using --topnav-height CSS variable,
  // and z-[60] to render above TopNav (z-20) and its dropdowns.
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 bg-black/30 backdrop-blur-sm z-[55] transition-opacity lg:hidden"
          style={{ top: 'var(--topnav-height, 56px)' }}
          onClick={() => onOpenChange?.(false)}
        />
      )}

      {/* Sidebar Drawer */}
      <div
        className={cn(
          'fixed left-0 bottom-0 w-72 z-[60] lg:hidden',
          'bg-background shadow-xl flex flex-col',
          'transform transition-transform duration-300 ease-out',
          open ? 'translate-x-0' : '-translate-x-full',
          className
        )}
        style={{ top: 'var(--topnav-height, 56px)' }}
      >
        <DashboardSidebarContent {...contentProps} />
      </div>
    </>
  )
}
