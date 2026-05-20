/**
 * InstallWidgetDialog — widget library browser for adding widgets to dashboard
 */

import { useState, useMemo, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, Plus } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'
import { useDashboardStore } from '../store'
import { getWidgetRegistry, groupComponentsByCategory, getCategoryInfo } from '../widgets/registry'
import type { WidgetCategory } from '../types'

export function InstallWidgetDialog() {
  const { t } = useTranslation()
  const [search, setSearch] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<WidgetCategory | 'all'>('all')
  const addComponent = useDashboardStore((s) => s.addComponent)
  const setComponentLibraryOpen = useDashboardStore((s) => s.setComponentLibraryOpen)

  const registry = useMemo(() => getWidgetRegistry(), [])

  const filteredGroups = useMemo(() => {
    let entries = Object.values(registry)

    if (selectedCategory !== 'all') {
      entries = entries.filter((e) => e.category === selectedCategory)
    }

    if (search.trim()) {
      const q = search.toLowerCase()
      entries = entries.filter(
        (e) =>
          e.displayName.toLowerCase().includes(q) ||
          e.type.toLowerCase().includes(q)
      )
    }

    return groupComponentsByCategory(entries)
  }, [registry, selectedCategory, search])

  const categories: Array<{ id: WidgetCategory | 'all'; label: string }> = [
    { id: 'all', label: t('dashboard.allWidgets', 'All') },
    { id: 'indicators', label: 'Indicators' },
    { id: 'charts', label: 'Charts' },
    { id: 'controls', label: 'Controls' },
    { id: 'display', label: 'Display' },
    { id: 'spatial', label: 'Spatial' },
    { id: 'business', label: 'Business' },
  ]

  const handleAdd = useCallback(
    (type: string) => {
      const entry = registry[type]
      if (!entry) return
      addComponent({
        type: entry.type as any,
        title: entry.displayName,
        position: { x: 0, y: Infinity, w: entry.defaultSize.w, h: entry.defaultSize.h },
      })
      setComponentLibraryOpen(false)
    },
    [registry, addComponent, setComponentLibraryOpen]
  )

  return (
    <div className="flex flex-col h-full">
      {/* Search */}
      <div className="px-3 py-2 border-b border-border">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('dashboard.searchWidgets', 'Search widgets...')}
            className="pl-7 h-8 text-xs"
          />
        </div>
      </div>

      {/* Category tabs */}
      <div className="flex gap-1 px-3 py-1.5 border-b border-border overflow-x-auto">
        {categories.map((cat) => (
          <button
            key={cat.id}
            onClick={() => setSelectedCategory(cat.id)}
            className={cn(
              'px-2 py-0.5 text-xs rounded-md whitespace-nowrap transition-colors',
              selectedCategory === cat.id
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-accent/50'
            )}
          >
            {cat.label}
          </button>
        ))}
      </div>

      {/* Widget list */}
      <ScrollArea className="flex-1">
        <div className="p-2 space-y-3">
          {filteredGroups.map(({ category, items }) => {
            const catInfo = getCategoryInfo(category)
            return (
              <div key={category}>
                <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider px-1 mb-1">
                  {catInfo.label}
                </p>
                <div className="grid grid-cols-2 gap-1.5">
                  {items.map((entry) => {
                    const Icon = entry.icon
                    return (
                      <button
                        key={entry.type}
                        onClick={() => handleAdd(entry.type)}
                        className="flex items-center gap-1.5 p-2 rounded-md border border-border hover:bg-accent/50 transition-colors text-left"
                      >
                        <Icon className="h-4 w-4 shrink-0 text-muted-foreground" />
                        <span className="text-xs truncate">{entry.displayName}</span>
                      </button>
                    )
                  })}
                </div>
              </div>
            )
          })}
          {filteredGroups.length === 0 && (
            <p className="text-xs text-muted-foreground text-center py-6">
              {t('dashboard.noWidgetsFound', 'No widgets found')}
            </p>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}
