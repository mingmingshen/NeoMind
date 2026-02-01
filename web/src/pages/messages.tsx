// Messages Page
// Unified notification/message system for NeoTalk

import { useState, useCallback, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useLocation } from 'react-router-dom'
import { PageLayout } from '@/components/layout/PageLayout'
import { PageTabs, PageTabsContent, EmptyStateInline, Pagination } from '@/components/shared'
import { api } from '@/lib/api'
import { useToast } from '@/hooks/use-toast'
import { confirm } from '@/hooks/use-confirm'
import type { NotificationMessage, MessageSeverity, MessageStatus, MessageCategory } from '@/types'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  AlertCircle,
  Info,
  AlertTriangle,
  ShieldAlert,
  Trash2,
  Bell,
  Megaphone,
  MoreVertical,
  Eye,
  RefreshCw,
} from 'lucide-react'
import { CreateMessageDialog } from '@/components/messages/CreateMessageDialog'
import { formatTimestamp } from '@/lib/utils/format'
import { cn } from '@/lib/utils'

type TabValue = 'messages' | 'channels'

// Get tab from URL path
const getTabFromPath = (pathname: string): TabValue => {
  const pathSegments = pathname.split('/')
  const lastSegment = pathSegments[pathSegments.length - 1]
  if (lastSegment === 'channels') {
    return 'channels'
  }
  return 'messages'
}

// Severity config
const SEVERITY_CONFIG: Record<string, { icon: typeof Info; color: string; bgColor: string }> = {
  info: { icon: Info, color: 'text-blue-500', bgColor: 'bg-blue-500/10 border-blue-500/20' },
  warning: { icon: AlertTriangle, color: 'text-yellow-500', bgColor: 'bg-yellow-500/10 border-yellow-500/20' },
  critical: { icon: AlertCircle, color: 'text-orange-500', bgColor: 'bg-orange-500/10 border-orange-500/20' },
  emergency: { icon: ShieldAlert, color: 'text-red-500', bgColor: 'bg-red-500/10 border-red-500/20' },
}

// Status config
const STATUS_CONFIG: Record<string, { variant: 'default' | 'secondary' | 'outline'; label: string }> = {
  active: { variant: 'default', label: 'messages.status.active' },
  acknowledged: { variant: 'secondary', label: 'messages.status.acknowledged' },
  resolved: { variant: 'outline', label: 'messages.status.resolved' },
  archived: { variant: 'outline', label: 'messages.status.archived' },
  false_positive: { variant: 'outline', label: 'messages.status.false_positive' },
}

// Category config
const CATEGORY_CONFIG: Record<string, { label: string; icon: typeof Bell }> = {
  alert: { label: 'messages.category.alert', icon: AlertCircle },
  system: { label: 'messages.category.system', icon: Bell },
  business: { label: 'messages.category.business', icon: Megaphone },
}

export default function MessagesPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const location = useLocation()

  // Tab state - sync with URL
  const [activeTab, setActiveTab] = useState<TabValue>(() => getTabFromPath(location.pathname))

  // Messages state
  const [messages, setMessages] = useState<NotificationMessage[]>([])
  const [loading, setLoading] = useState(false)

  // Pagination
  const [messagePage, setMessagePage] = useState(1)
  const messagesPerPage = 10

  // Filters
  const [severityFilter, setSeverityFilter] = useState<MessageSeverity | 'all'>('all')
  const [statusFilter, setStatusFilter] = useState<MessageStatus | 'all'>('all')
  const [categoryFilter, setCategoryFilter] = useState<MessageCategory | 'all'>('all')

  // Calculate paginated messages
  const paginatedMessages = messages.slice(
    (messagePage - 1) * messagesPerPage,
    messagePage * messagesPerPage
  )

  // Channels state
  const [channels, setChannels] = useState<any[]>([])

  // Dialogs
  const [createDialogOpen, setCreateDialogOpen] = useState(false)

  const { toast } = useToast()

  // Update tab when URL changes
  useEffect(() => {
    setActiveTab(getTabFromPath(location.pathname))
  }, [location.pathname])

  // Update URL when tab changes
  const handleTabChange = (tab: TabValue) => {
    setActiveTab(tab)
    if (tab === 'channels') {
      navigate('/messages/channels')
    } else {
      navigate('/messages')
    }
  }

  // Fetch messages using the unified messages API
  const fetchMessages = useCallback(async () => {
    setLoading(true)
    try {
      const response = await fetch('/api/messages', {
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token') || ''}`,
        },
      })
      const rawData = await response.json() as any
      console.log('Messages API response:', rawData)

      // Handle different response formats from messages endpoint
      let messagesArray: any[] = []
      if (Array.isArray(rawData)) {
        messagesArray = rawData
      } else if (rawData?.messages && Array.isArray(rawData.messages)) {
        messagesArray = rawData.messages
      } else if (rawData?.data && Array.isArray(rawData.data)) {
        messagesArray = rawData.data
      }

      // Convert to NotificationMessage format
      let messages: NotificationMessage[] = messagesArray.map((msg: any) => ({
        id: msg.id || '',
        category: msg.category || 'alert',
        severity: msg.severity || 'info',
        title: msg.title || '',
        message: msg.message || '',
        source: msg.source || 'system',
        source_type: msg.source_type || '',
        timestamp: msg.timestamp || msg.created_at || new Date().toISOString(),
        status: msg.status || 'active',
        tags: msg.tags || [],
        metadata: msg.metadata,
      }))

      console.log('Messages before filter:', messages.length, messages)

      // Apply filters
      if (severityFilter !== 'all') {
        messages = messages.filter((m: NotificationMessage) => m.severity === severityFilter)
      }
      if (statusFilter !== 'all') {
        messages = messages.filter((m: NotificationMessage) => m.status === statusFilter)
      }
      if (categoryFilter !== 'all') {
        messages = messages.filter((m: NotificationMessage) => m.category === categoryFilter)
      }

      console.log('Messages after filter:', messages.length, 'Filters:', { severityFilter, statusFilter, categoryFilter })

      // Sort by timestamp descending
      messages.sort((a: NotificationMessage, b: NotificationMessage) =>
        new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
      )

      setMessages(messages)
      setMessagePage(1) // Reset to first page when data changes
    } catch (error) {
      console.error('Failed to fetch messages:', error)
    } finally {
      setLoading(false)
    }
  }, [severityFilter, statusFilter, categoryFilter])

  // Fetch channels
  const fetchChannels = useCallback(async () => {
    setLoading(true)
    try {
      const response = await api.listMessageChannels()
      setChannels(response.channels || [])
    } catch (error) {
      console.error('Failed to fetch channels:', error)
    } finally {
      setLoading(false)
    }
  }, [])

  // Initial load
  useEffect(() => {
    if (activeTab === 'messages') {
      fetchMessages()
    } else {
      fetchChannels()
    }
  }, [activeTab, fetchMessages, fetchChannels])

  // Message actions - using messages API endpoints
  const handleAcknowledge = async (id: string) => {
    try {
      const response = await fetch(`/api/messages/${id}/acknowledge`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token') || ''}`,
          'Content-Type': 'application/json',
        },
      })
      if (response.ok) {
        setMessages(prev => prev.map(m =>
          m.id === id ? { ...m, status: 'acknowledged' as MessageStatus, acknowledged: true } : m
        ))
        toast({ title: t('messages.acknowledgeSuccess', 'Acknowledged') })
      } else {
        throw new Error('Failed to acknowledge')
      }
    } catch (error) {
      console.error('Failed to acknowledge:', error)
      toast({ title: t('messages.acknowledgeError', 'Failed'), variant: 'destructive' })
    }
  }

  const handleResolve = async (id: string) => {
    try {
      const response = await fetch(`/api/messages/${id}/resolve`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token') || ''}`,
          'Content-Type': 'application/json',
        },
      })
      if (response.ok) {
        setMessages(prev => prev.map(m =>
          m.id === id ? { ...m, status: 'resolved' as MessageStatus, acknowledged: true } : m
        ))
        toast({ title: t('messages.resolveSuccess', 'Resolved') })
      } else {
        throw new Error('Failed to resolve')
      }
    } catch (error) {
      console.error('Failed to resolve:', error)
      toast({ title: t('messages.resolveError', 'Failed'), variant: 'destructive' })
    }
  }

  const handleArchive = async (id: string) => {
    try {
      const response = await fetch(`/api/messages/${id}/archive`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token') || ''}`,
          'Content-Type': 'application/json',
        },
      })
      if (response.ok) {
        setMessages(prev => prev.map(m =>
          m.id === id ? { ...m, status: 'archived' as MessageStatus } : m
        ))
        toast({ title: t('messages.archiveSuccess', 'Archived') })
      } else {
        throw new Error('Failed to archive')
      }
    } catch (error) {
      console.error('Failed to archive:', error)
      toast({ title: t('messages.archiveError', 'Failed'), variant: 'destructive' })
    }
  }

  const handleDelete = async (id: string) => {
    const confirmed = await confirm({
      title: t('delete'),
      description: t('messages.confirmBulkDeleteDesc', { count: 1 }),
      confirmText: t('delete'),
      cancelText: t('cancel'),
      variant: 'destructive',
    })
    if (!confirmed) return

    try {
      const response = await fetch(`/api/messages/${id}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token') || ''}`,
        },
      })
      if (response.ok) {
        setMessages(prev => prev.filter(m => m.id !== id))
        toast({ title: t('messages.deleteSuccess', 'Deleted') })
      } else {
        throw new Error('Failed to delete')
      }
    } catch (error) {
      console.error('Failed to delete:', error)
      toast({ title: t('messages.deleteError', 'Failed to delete'), variant: 'destructive' })
    }
  }

  const tabs = [
    { value: 'messages' as TabValue, label: t('messages.tabs.messages') },
    { value: 'channels' as TabValue, label: t('messages.tabs.channels') },
  ]

  return (
    <PageLayout
      title={t('messages.title')}
      subtitle={t('messages.description')}
    >
      <PageTabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(v) => handleTabChange(v as TabValue)}
        actions={[
          ...(activeTab === 'messages' ? [
            { label: t('messages.create'), onClick: () => setCreateDialogOpen(true) },
          ] : []),
          { label: t('refresh'), variant: 'outline' as const, onClick: activeTab === 'messages' ? fetchMessages : fetchChannels, disabled: loading },
        ]}
      >
        {/* Messages Tab */}
        <PageTabsContent value="messages" activeTab={activeTab}>
          {/* Filters */}
          <div className="flex flex-wrap items-center gap-3 mb-4">
            <Select value={severityFilter} onValueChange={(v) => setSeverityFilter(v as MessageSeverity | 'all')}>
              <SelectTrigger className="w-[140px]">
                <SelectValue placeholder={t('messages.filter.severity')} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('messages.filter.all')}</SelectItem>
                <SelectItem value="info">{t('messages.severity.info')}</SelectItem>
                <SelectItem value="warning">{t('messages.severity.warning')}</SelectItem>
                <SelectItem value="critical">{t('messages.severity.critical')}</SelectItem>
                <SelectItem value="emergency">{t('messages.severity.emergency')}</SelectItem>
              </SelectContent>
            </Select>

            <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as MessageStatus | 'all')}>
              <SelectTrigger className="w-[140px]">
                <SelectValue placeholder={t('messages.filter.status')} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('messages.filter.all')}</SelectItem>
                <SelectItem value="active">{t('messages.status.active')}</SelectItem>
                <SelectItem value="acknowledged">{t('messages.status.acknowledged')}</SelectItem>
                <SelectItem value="resolved">{t('messages.status.resolved')}</SelectItem>
                <SelectItem value="archived">{t('messages.status.archived')}</SelectItem>
              </SelectContent>
            </Select>

            <Select value={categoryFilter} onValueChange={(v) => setCategoryFilter(v as MessageCategory | 'all')}>
              <SelectTrigger className="w-[140px]">
                <SelectValue placeholder={t('messages.filter.category')} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('messages.filter.all')}</SelectItem>
                <SelectItem value="alert">{t('messages.category.alert')}</SelectItem>
                <SelectItem value="system">{t('messages.category.system')}</SelectItem>
                <SelectItem value="business">{t('messages.category.business')}</SelectItem>
              </SelectContent>
            </Select>

            {(severityFilter !== 'all' || statusFilter !== 'all' || categoryFilter !== 'all') && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setSeverityFilter('all')
                  setStatusFilter('all')
                  setCategoryFilter('all')
                }}
              >
                {t('messages.filter.clear')}
              </Button>
            )}
          </div>

          {/* Messages Table */}
          <Card className="overflow-hidden">
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent border-b bg-muted/30">
                  <TableHead className="w-[50px]"></TableHead>
                  <TableHead>
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('messages.formTitle.label')}
                    </div>
                  </TableHead>
                  <TableHead>
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('messages.content.label')}
                    </div>
                  </TableHead>
                  <TableHead className="w-[90px]">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('messages.severity.label')}
                    </div>
                  </TableHead>
                  <TableHead className="w-[90px]">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('messages.category.label')}
                    </div>
                  </TableHead>
                  <TableHead className="w-[90px]">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('messages.status.label')}
                    </div>
                  </TableHead>
                  <TableHead className="w-[130px]">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('common.createdAt')}
                    </div>
                  </TableHead>
                  <TableHead className="w-12"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loading ? (
                  <TableRow>
                    <TableCell colSpan={8} className="py-8">
                      <div className="flex items-center justify-center gap-2">
                        <RefreshCw className="h-4 w-4 animate-spin" />
                        <span className="text-sm text-muted-foreground">{t('loading')}</span>
                      </div>
                    </TableCell>
                  </TableRow>
                ) : messages.length === 0 ? (
                  <EmptyStateInline
                    title={t('messages.empty.title')}
                    colSpan={8}
                  />
                ) : (
                  paginatedMessages.map((message) => {
                    const severityConfig = SEVERITY_CONFIG[message.severity] || SEVERITY_CONFIG.info
                    const categoryConfig = CATEGORY_CONFIG[message.category] || CATEGORY_CONFIG.system
                    const statusConfig = STATUS_CONFIG[message.status] || STATUS_CONFIG.active
                    const SeverityIcon = severityConfig.icon
                    const CategoryIcon = categoryConfig.icon

                    return (
                      <TableRow
                        key={message.id}
                        className={cn(
                          "group transition-colors hover:bg-muted/50",
                          (message.status === 'resolved' || message.status === 'archived') && "opacity-60"
                        )}
                      >
                        <TableCell className="text-center">
                          <SeverityIcon className={cn("h-4 w-4 mx-auto", severityConfig.color)} />
                        </TableCell>
                        <TableCell>
                          <div className="font-medium text-sm truncate pr-4" title={message.title}>
                            {message.title}
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="text-sm text-muted-foreground line-clamp-2">
                            {message.message}
                          </div>
                          {message.tags.length > 0 && (
                            <div className="flex items-center gap-1 mt-1.5 flex-wrap">
                              {message.tags.slice(0, 4).map((tag, i) => (
                                <Badge key={i} variant="secondary" className="text-xs h-5 px-1.5">
                                  {tag}
                                </Badge>
                              ))}
                              {message.tags.length > 4 && (
                                <span className="text-xs text-muted-foreground">+{message.tags.length - 4}</span>
                              )}
                            </div>
                          )}
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className={cn("text-xs", severityConfig.bgColor, severityConfig.color)}>
                            {t(`messages.severity.${message.severity}`)}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1.5">
                            <CategoryIcon className="h-3.5 w-3.5 text-muted-foreground" />
                            <span className="text-xs">{t(categoryConfig.label)}</span>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant={statusConfig.variant} className="text-xs">
                            {t(statusConfig.label)}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <span className="text-xs text-muted-foreground">
                            {formatTimestamp(message.timestamp, false)}
                          </span>
                        </TableCell>
                        <TableCell>
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="icon" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end" className="w-40">
                              {message.status === 'active' && (
                                <DropdownMenuItem onClick={() => handleAcknowledge(message.id)}>
                                  <Eye className="mr-2 h-4 w-4" />
                                  {t('messages.acknowledge')}
                                </DropdownMenuItem>
                              )}
                              {message.status !== 'resolved' && message.status !== 'archived' && (
                                <DropdownMenuItem onClick={() => handleResolve(message.id)}>
                                  <Eye className="mr-2 h-4 w-4" />
                                  {t('messages.resolve')}
                                </DropdownMenuItem>
                              )}
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                onClick={() => handleDelete(message.id)}
                                className="text-destructive"
                              >
                                <Trash2 className="mr-2 h-4 w-4" />
                                {t('delete')}
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </TableCell>
                      </TableRow>
                    )
                  })
                )}
              </TableBody>
            </Table>
          </Card>

          {/* Pagination */}
          {messages.length > messagesPerPage && (
            <div className="sticky bottom-0 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 pt-4 pb-2">
              <Pagination
                total={messages.length}
                pageSize={messagesPerPage}
                currentPage={messagePage}
                onPageChange={setMessagePage}
              />
            </div>
          )}
        </PageTabsContent>

        {/* Channels Tab */}
        <PageTabsContent value="channels" activeTab={activeTab}>
          <Card className="overflow-hidden">
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent border-b bg-muted/30">
                  <TableHead>
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      <Bell className="h-4 w-4" />
                      {t('messages.channels.name')}
                    </div>
                  </TableHead>
                  <TableHead>
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('messages.channels.type')}
                    </div>
                  </TableHead>
                  <TableHead align="center">
                    <div className="flex items-center justify-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      {t('status')}
                    </div>
                  </TableHead>
                  <TableHead className="w-12"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loading ? (
                  <TableRow>
                    <TableCell colSpan={4} className="py-8">
                      <div className="flex items-center justify-center gap-2">
                        <RefreshCw className="h-4 w-4 animate-spin" />
                        <span className="text-sm text-muted-foreground">{t('loading')}</span>
                      </div>
                    </TableCell>
                  </TableRow>
                ) : channels.length === 0 ? (
                  <EmptyStateInline
                    title={t('messages.channels.empty.title')}
                    colSpan={4}
                  />
                ) : (
                  channels.map((channel) => {
                    const config: Record<string, { icon: any; color: string }> = {
                      console: { icon: Bell, color: 'bg-gray-500/10 text-gray-500' },
                      memory: { icon: RefreshCw, color: 'bg-blue-500/10 text-blue-500' },
                      webhook: { icon: Megaphone, color: 'bg-green-500/10 text-green-500' },
                      email: { icon: Bell, color: 'bg-purple-500/10 text-purple-500' },
                    }
                    const channelConfig = config[channel.channel_type] || config.console
                    const ChannelIcon = channelConfig.icon

                    return (
                      <TableRow key={channel.name} className="group transition-colors hover:bg-muted/50">
                        <TableCell>
                          <div className="flex items-center gap-3">
                            <div className={`w-9 h-9 rounded-lg flex items-center justify-center transition-colors ${channelConfig.color}`}>
                              <ChannelIcon className="h-4 w-4" />
                            </div>
                            <div>
                              <div className="font-medium text-sm">{channel.name}</div>
                              <div className="text-xs text-muted-foreground">{channel.channel_type}</div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {channel.channel_type}
                          </Badge>
                        </TableCell>
                        <TableCell align="center">
                          <Badge variant={channel.enabled ? 'default' : 'secondary'} className="text-xs">
                            {channel.enabled ? t('enabled') : t('disabled')}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="icon" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end" className="w-40">
                              <DropdownMenuItem>
                                <Eye className="mr-2 h-4 w-4" />
                                {t('common.view')}
                              </DropdownMenuItem>
                              <DropdownMenuItem>
                                {channel.enabled ? (
                                  <>
                                    <RefreshCw className="mr-2 h-4 w-4" />
                                    {t('common.disable')}
                                  </>
                                ) : (
                                  <>
                                    <RefreshCw className="mr-2 h-4 w-4" />
                                    {t('common.enable')}
                                  </>
                                )}
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </TableCell>
                      </TableRow>
                    )
                  })
                )}
              </TableBody>
            </Table>
          </Card>
        </PageTabsContent>
      </PageTabs>

      <CreateMessageDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        onCreate={async (req) => {
          // Use the messages API to create a new message
          const response = await fetch('/api/messages', {
            method: 'POST',
            headers: {
              'Authorization': `Bearer ${localStorage.getItem('token') || ''}`,
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({
              category: req.category || 'alert',
              severity: req.severity || 'info',
              title: req.title,
              message: req.message,
              source: req.source || 'manual',
              source_type: req.source_type || 'ui',
              tags: req.tags || [],
            }),
          })
          if (response.ok) {
            await fetchMessages()
            toast({ title: t('messages.createSuccess', 'Message created') })
          } else {
            toast({ title: t('messages.createError', 'Failed to create'), variant: 'destructive' })
          }
        }}
      />
    </PageLayout>
  )
}
