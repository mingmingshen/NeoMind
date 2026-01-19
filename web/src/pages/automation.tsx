/**
 * NeoTalk Automation Page
 *
 * Unified automation interface with rules, workflows, and data transforms.
 * Uses PageLayout + PageTabs structure consistent with other pages.
 */

import { useState, useMemo, useCallback, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Search,
  RefreshCw,
  Zap,
  Workflow,
  GitBranch,
  Plus,
  Edit,
  Trash2,
  Play,
  MoreVertical,
} from "lucide-react"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Switch } from "@/components/ui/switch"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Card } from "@/components/ui/card"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import type { TransformAutomation } from "@/types"

// Import full-screen editor components
import { RuleFullScreenEditor } from "./automation/RuleFullScreenEditor"
import { WorkflowFullScreenEditor } from "./automation/WorkflowFullScreenEditor"
import { TransformFullScreenEditor } from "./automation/TransformFullScreenEditor"

type AutomationTab = 'rules' | 'workflows' | 'transforms'
type AutomationStatus = 'enabled' | 'disabled' | 'error' | 'running'

interface AutomationItem {
  id: string
  name: string
  description: string
  status: AutomationStatus
  type: 'rule' | 'workflow' | 'transform'
  triggerCount: number
  lastTriggered?: string
  icon: React.ReactNode
  category?: string
  complexity?: 'simple' | 'medium' | 'complex'
}

// Tab navigation items
const tabItems = [
  {
    id: 'rules' as const,
    label: 'automation:tabs.rules',
    icon: Zap,
  },
  {
    id: 'workflows' as const,
    icon: Workflow,
    label: 'automation:tabs.workflows',
  },
  {
    id: 'transforms' as const,
    icon: GitBranch,
    label: 'automation:tabs.transforms',
  },
]

export function AutomationPage() {
  const { t } = useTranslation(['common', 'automation'])
  const [activeTab, setActiveTab] = useState<AutomationTab>('rules')
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null)

  // Full screen editor state
  const [showEditor, setShowEditor] = useState(false)
  const [editingItem, setEditingItem] = useState<any>(null)

  // Data state
  const [rules, setRules] = useState<any[]>([])
  const [workflows, setWorkflows] = useState<any[]>([])
  const [transforms, setTransforms] = useState<any[]>([])
  const [loading, setLoading] = useState(false)

  // Fetch data
  const loadItems = useCallback(async () => {
    setLoading(true)
    try {
      if (activeTab === 'rules') {
        const data = await api.listRules()
        setRules(data.rules || [])
      } else if (activeTab === 'workflows') {
        const data = await api.listWorkflows()
        setWorkflows(data.workflows || [])
      } else if (activeTab === 'transforms') {
        const data = await api.listTransforms()
        setTransforms(data.transforms || [])
      }
    } catch (error) {
      console.error(`Failed to load ${activeTab}:`, error)
    } finally {
      setLoading(false)
    }
  }, [activeTab])

  // Load items when tab changes
  useEffect(() => {
    loadItems()
  }, [loadItems])

  // Convert API data to display items
  const displayItems = useMemo(() => {
    const sourceItems = activeTab === 'rules' ? rules : activeTab === 'workflows' ? workflows : transforms

    return sourceItems.map((item: any) => {
      let complexity: 'simple' | 'medium' | 'complex' | undefined = undefined
      if (item.complexity) {
        if (item.complexity < 3) complexity = 'simple'
        else if (item.complexity < 5) complexity = 'medium'
        else complexity = 'complex'
      }

      return {
        id: item.id,
        name: item.name,
        description: item.description || '',
        status: item.enabled ? 'enabled' : 'disabled',
        type: activeTab === 'rules' ? 'rule' : activeTab === 'workflows' ? 'workflow' : 'transform',
        triggerCount: item.trigger_count || item.execution_count || 0,
        lastTriggered: item.last_triggered || item.last_execution,
        icon: activeTab === 'rules' ? <Zap className="h-4 w-4" /> :
               activeTab === 'workflows' ? <Workflow className="h-4 w-4" /> :
               <GitBranch className="h-4 w-4" />,
        category: item.category,
        complexity,
      } as AutomationItem
    })
  }, [rules, workflows, transforms, activeTab])

  // Filter items
  const filteredItems = useMemo(() => {
    return displayItems.filter((item) => {
      const matchesSearch = !searchQuery ||
        item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        item.description.toLowerCase().includes(searchQuery.toLowerCase())
      const matchesCategory = !selectedCategory || item.category === selectedCategory
      return matchesSearch && matchesCategory
    })
  }, [displayItems, searchQuery, selectedCategory])

  // Categories
  const categories = useMemo(() => {
    const cats = new Set(displayItems.map(item => item.category).filter(Boolean))
    return Array.from(cats) as string[]
  }, [displayItems])

  // Handlers
  const handleCreate = () => {
    setEditingItem(null)
    setShowEditor(true)
  }

  const handleEdit = (item: AutomationItem) => {
    // Find original item from source data
    const sourceItems = activeTab === 'rules' ? rules : activeTab === 'workflows' ? workflows : transforms
    const originalItem = sourceItems.find((i: any) => i.id === item.id)
    setEditingItem(originalItem)
    setShowEditor(true)
  }

  const handleDelete = async (item: AutomationItem) => {
    if (!confirm(t('automation:deleteConfirm'))) return

    try {
      if (activeTab === 'rules') {
        await api.deleteRule(item.id)
      } else if (activeTab === 'workflows') {
        await api.deleteWorkflow(item.id)
      } else if (activeTab === 'transforms') {
        await api.deleteAutomation(item.id)
      }
      await loadItems()
    } catch (error) {
      console.error('Failed to delete item:', error)
    }
  }

  const handleToggleStatus = async (item: AutomationItem) => {
    try {
      const newStatus = !item.status || item.status !== 'enabled'
      if (activeTab === 'rules') {
        await api.enableRule(item.id)
      } else if (activeTab === 'workflows') {
        await api.setAutomationStatus(item.id, newStatus)
      } else if (activeTab === 'transforms') {
        await api.setAutomationStatus(item.id, newStatus)
      }
      await loadItems()
    } catch (error) {
      console.error('Failed to toggle status:', error)
    }
  }

  const handleExecute = async (item: AutomationItem) => {
    try {
      if (activeTab === 'rules') {
        await api.testRule(item.id)
      } else if (activeTab === 'workflows') {
        await api.executeWorkflow(item.id)
      }
    } catch (error) {
      console.error('Failed to execute:', error)
    }
  }

  const handleSave = async () => {
    setShowEditor(false)
    setEditingItem(null)
    await loadItems()
  }

  const getStatusBadge = (status: AutomationStatus) => {
    const variants: Record<AutomationStatus, { variant: string; label: string }> = {
      enabled: { variant: 'default', label: t('automation:statusEnabled') },
      disabled: { variant: 'secondary', label: t('automation:statusDisabled') },
      error: { variant: 'destructive', label: t('automation:statusError') },
      running: { variant: 'outline', label: t('automation:statusRunning') },
    }
    const { variant, label } = variants[status]
    return <Badge variant={variant as any}>{label}</Badge>
  }

  const getEmptyState = () => {
    const messages = {
      rules: { title: t('automation:noRules'), desc: t('automation:noRulesDesc') },
      workflows: { title: t('automation:noWorkflows'), desc: t('automation:noWorkflowsDesc') },
      transforms: { title: t('automation:noTransforms'), desc: t('automation:noTransformsDesc') },
    }
    const msg = messages[activeTab]
    return (
      <Card className="p-12 text-center">
        <div className="flex flex-col items-center gap-4">
          <div className="p-4 rounded-full bg-muted">
            {activeTab === 'rules' && <Zap className="h-8 w-8 text-muted-foreground" />}
            {activeTab === 'workflows' && <Workflow className="h-8 w-8 text-muted-foreground" />}
            {activeTab === 'transforms' && <GitBranch className="h-8 w-8 text-muted-foreground" />}
          </div>
          <h3 className="text-lg font-semibold">{msg.title}</h3>
          <p className="text-muted-foreground">{msg.desc}</p>
        </div>
      </Card>
    )
  }

  return (
    <PageLayout maxWidth="full">
      {/* Page Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold">{t('automation:title')}</h1>
        <p className="text-muted-foreground">{t('automation:description')}</p>
      </div>

      {/* Tabs with Actions */}
      <PageTabs
        tabs={tabItems.map(tab => ({
          value: tab.id,
          label: t(tab.label as any),
          icon: <tab.icon className="h-4 w-4" />,
        }))}
        activeTab={activeTab}
        onTabChange={(v) => setActiveTab(v as AutomationTab)}
        actions={[
          {
            label: t('automation:create', { tab: t(`automation:tabs.${activeTab}`) }),
            icon: <Plus className="h-4 w-4" />,
            variant: 'default',
            onClick: handleCreate,
          },
          {
            label: t('common:search'),
            icon: <Search className="h-4 w-4" />,
            variant: 'outline',
            onClick: () => {}, // Search is handled by the input below
            disabled: true,
          },
          {
            label: t('common:refresh'),
            icon: <RefreshCw className={cn('h-4 w-4', loading && 'animate-spin')} />,
            variant: 'outline',
            onClick: loadItems,
            loading: loading,
          },
        ]}
      >
        {/* Search and Filter Bar */}
        <div className="flex items-center gap-4 mb-4">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder={t('automation:searchPlaceholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
          {categories.length > 0 && (
            <div className="flex gap-2">
              <Button
                variant={!selectedCategory ? 'default' : 'outline'}
                size="sm"
                onClick={() => setSelectedCategory(null)}
              >
                {t('automation:allCategories')}
              </Button>
              {categories.map(cat => (
                <Button
                  key={cat}
                  variant={selectedCategory === cat ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setSelectedCategory(cat)}
                >
                  {cat}
                </Button>
              ))}
            </div>
          )}
        </div>

        {/* Content for each tab */}
        <PageTabsContent value="rules" activeTab={activeTab}>
          {filteredItems.length === 0 ? getEmptyState() : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12"></TableHead>
                  <TableHead>{t('automation:ruleName')}</TableHead>
                  <TableHead>{t('automation:description')}</TableHead>
                  <TableHead>{t('automation:status')}</TableHead>
                  <TableHead>{t('automation:triggerCount')}</TableHead>
                  <TableHead>{t('automation:lastTriggered')}</TableHead>
                  <TableHead className="text-right">{t('common:actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredItems.map((item, index) => (
                  <TableRow key={item.id}>
                    <TableCell>{index + 1}</TableCell>
                    <TableCell className="font-medium">{item.name}</TableCell>
                    <TableCell className="text-muted-foreground max-w-md truncate">
                      {item.description || '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={item.status === 'enabled'}
                          onCheckedChange={() => handleToggleStatus(item)}
                        />
                        {getStatusBadge(item.status)}
                      </div>
                    </TableCell>
                    <TableCell>{item.triggerCount}</TableCell>
                    <TableCell>{item.lastTriggered || '-'}</TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEdit(item)}>
                            <Edit className="mr-2 h-4 w-4" />
                            {t('common:edit')}
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => handleExecute(item)}>
                            <Play className="mr-2 h-4 w-4" />
                            {t('automation:execute')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem onClick={() => handleDelete(item)} className="text-destructive">
                            <Trash2 className="mr-2 h-4 w-4" />
                            {t('common:delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </PageTabsContent>

        <PageTabsContent value="workflows" activeTab={activeTab}>
          {filteredItems.length === 0 ? getEmptyState() : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12"></TableHead>
                  <TableHead>{t('automation:workflowName')}</TableHead>
                  <TableHead>{t('automation:description')}</TableHead>
                  <TableHead>{t('automation:status')}</TableHead>
                  <TableHead>{t('automation:executionCount')}</TableHead>
                  <TableHead>{t('automation:updatedAt')}</TableHead>
                  <TableHead className="text-right">{t('common:actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredItems.map((item, index) => (
                  <TableRow key={item.id}>
                    <TableCell>{index + 1}</TableCell>
                    <TableCell className="font-medium">{item.name}</TableCell>
                    <TableCell className="text-muted-foreground max-w-md truncate">
                      {item.description || '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={item.status === 'enabled'}
                          onCheckedChange={() => handleToggleStatus(item)}
                        />
                        {getStatusBadge(item.status)}
                      </div>
                    </TableCell>
                    <TableCell>{item.triggerCount}</TableCell>
                    <TableCell>{item.lastTriggered || '-'}</TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEdit(item)}>
                            <Edit className="mr-2 h-4 w-4" />
                            {t('common:edit')}
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => handleExecute(item)}>
                            <Play className="mr-2 h-4 w-4" />
                            {t('automation:execute')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem onClick={() => handleDelete(item)} className="text-destructive">
                            <Trash2 className="mr-2 h-4 w-4" />
                            {t('common:delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </PageTabsContent>

        <PageTabsContent value="transforms" activeTab={activeTab}>
          {filteredItems.length === 0 ? getEmptyState() : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12"></TableHead>
                  <TableHead>{t('automation:name')}</TableHead>
                  <TableHead>{t('automation:scope')}</TableHead>
                  <TableHead>{t('automation:description')}</TableHead>
                  <TableHead>{t('automation:status')}</TableHead>
                  <TableHead className="text-right">{t('common:actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredItems.map((item, index) => (
                  <TableRow key={item.id}>
                    <TableCell>{index + 1}</TableCell>
                    <TableCell className="font-medium">{item.name}</TableCell>
                    <TableCell>
                      <Badge variant="outline">{item.category || 'global'}</Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground max-w-md truncate">
                      {item.description || '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={item.status === 'enabled'}
                          onCheckedChange={() => handleToggleStatus(item)}
                        />
                        {getStatusBadge(item.status)}
                      </div>
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEdit(item)}>
                            <Edit className="mr-2 h-4 w-4" />
                            {t('common:edit')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem onClick={() => handleDelete(item)} className="text-destructive">
                            <Trash2 className="mr-2 h-4 w-4" />
                            {t('common:delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </PageTabsContent>
      </PageTabs>

      {/* Full Screen Editors */}
      {activeTab === 'rules' && (
        <RuleFullScreenEditor
          open={showEditor}
          rule={editingItem}
          onClose={() => setShowEditor(false)}
          onSave={handleSave}
        />
      )}

      {activeTab === 'workflows' && (
        <WorkflowFullScreenEditor
          open={showEditor}
          workflow={editingItem}
          onClose={() => setShowEditor(false)}
          onSave={handleSave}
        />
      )}

      {activeTab === 'transforms' && (
        <TransformFullScreenEditor
          open={showEditor}
          transform={editingItem as TransformAutomation | null}
          onClose={() => setShowEditor(false)}
          onSave={handleSave}
        />
      )}
    </PageLayout>
  )
}

// Export as default for the route
export default AutomationPage
