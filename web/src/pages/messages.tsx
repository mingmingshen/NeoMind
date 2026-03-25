// Messages Page
// Unified notification/message system for NeoMind

import { useState, useCallback, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useLocation } from 'react-router-dom'
import { PageLayout } from '@/components/layout/PageLayout'
import { PageTabsBar, PageTabsContent, PageTabsBottomNav, EmptyStateInline, Pagination, ResponsiveTable } from '@/components/shared'
import { MessageSquare, Network, Settings } from 'lucide-react'
import { api, getApiBase } from '@/lib/api'
import { useToast } from '@/hooks/use-toast'
import { confirm } from '@/hooks/use-confirm'
import { useErrorHandler } from '@/hooks/useErrorHandler'
import { useIsMobile } from '@/hooks/useMobile'
import type { NotificationMessage, MessageSeverity, MessageStatus, MessageCategory, MessageChannel, MessageType, DeliveryLog } from '@/types'
import type { StandardError } from '@/lib/errors'

// Raw API response types
interface RawNotificationMessage {
  id?: string
  category?: string
  severity?: MessageSeverity
  title?: string
  message?: string
  source?: string
  source_type?: string
  timestamp?: string
  created_at?: string
  status?: MessageStatus
  tags?: string[]
  metadata?: Record<string, unknown>
}

interface MessagesApiResponse {
  messages?: RawNotificationMessage[]
  data?: {
    messages?: RawNotificationMessage[]
  } | RawNotificationMessage[]
}
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
import { Checkbox } from '@/components/ui/checkbox'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { UnifiedFormDialog } from '@/components/dialog/UnifiedFormDialog'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { FormField } from '@/components/ui/field'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  DropdownMenuCheckboxItem,
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
  Filter,
  X,
  CheckCircle2,
  TestTube,
  Mail,
  UserPlus,
  Send,
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

// Category config - supports any category from backend with fallback
const CATEGORY_CONFIG: Record<string, { label: string; icon: typeof Bell }> = {
  alert: { label: 'messages.category.alert', icon: AlertCircle },
  system: { label: 'messages.category.system', icon: Bell },
  business: { label: 'messages.category.business', icon: Megaphone },
  notification: { label: 'messages.category.notification', icon: Bell },
}

// Helper to get category config with fallback for unknown categories
const getCategoryConfig = (category: string) => {
  return CATEGORY_CONFIG[category] || {
    label: category, // Use raw category name as label for unknown categories
    icon: Bell,
  }
}

export default function MessagesPage() {
  const { t } = useTranslation()
  const { handleError } = useErrorHandler()
  const navigate = useNavigate()
  const location = useLocation()
  const isMobile = useIsMobile()

  // Helper to get API base URL for Tauri environment
  const getApiUrl = (path: string) => {
    const apiBase = getApiBase()
    return `${apiBase}${path}`
  }

  // Tab state - sync with URL
  const [activeTab, setActiveTab] = useState<TabValue>(() => getTabFromPath(location.pathname))

  // Messages state
  const [messages, setMessages] = useState<NotificationMessage[]>([])
  const [loading, setLoading] = useState(false)

  // Pagination
  const [messagePage, setMessagePage] = useState(1)
  const messagesPerPage = 10

  // Filters - support multiple selections
  const [selectedSeverities, setSelectedSeverities] = useState<Set<MessageSeverity>>(new Set())
  const [selectedStatuses, setSelectedStatuses] = useState<Set<MessageStatus>>(new Set())
  const [selectedCategories, setSelectedCategories] = useState<Set<string>>(new Set())
  const [selectedMessageTypes, setSelectedMessageTypes] = useState<Set<string>>(new Set())

  // Delivery logs for DataPush messages
  const [deliveryLogs, setDeliveryLogs] = useState<DeliveryLog[]>([])

  // Dynamic categories from API
  const [availableCategories, setAvailableCategories] = useState<string[]>([])

  // Check if any filters are active
  const hasActiveFilters = selectedSeverities.size > 0 || selectedStatuses.size > 0 || selectedCategories.size > 0 || selectedMessageTypes.size > 0

  // Clear all filters
  const clearAllFilters = () => {
    setSelectedMessageTypes(new Set())
    setSelectedSeverities(new Set())
    setSelectedStatuses(new Set())
    setSelectedCategories(new Set())
  }

  // Toggle filter
  const toggleSeverity = (severity: MessageSeverity) => {
    const newSet = new Set(selectedSeverities)
    if (newSet.has(severity)) {
      newSet.delete(severity)
    } else {
      newSet.add(severity)
    }
    setSelectedSeverities(newSet)
  }

  const toggleStatus = (status: MessageStatus) => {
    const newSet = new Set(selectedStatuses)
    if (newSet.has(status)) {
      newSet.delete(status)
    } else {
      newSet.add(status)
    }
    setSelectedStatuses(newSet)
  }

  const toggleCategory = (category: string) => {
    const newSet = new Set(selectedCategories)
    if (newSet.has(category)) {
      newSet.delete(category)
    } else {
      newSet.add(category)
    }
    setSelectedCategories(newSet)
  }

  const toggleMessageType = (type: string) => {
    const newSet = new Set(selectedMessageTypes)
    if (newSet.has(type)) {
      newSet.delete(type)
    } else {
      newSet.add(type)
    }
    setSelectedMessageTypes(newSet)
  }

  // Get active filter count
  const getActiveFilterCount = () => selectedSeverities.size + selectedStatuses.size + selectedCategories.size + selectedMessageTypes.size

  // Test channel state
  const [testingChannel, setTestingChannel] = useState<string | null>(null)
  const [testResults, setTestResults] = useState<Record<string, { success: boolean; message: string }>>({})

  // View channel dialog state
  const [viewChannel, setViewChannel] = useState<MessageChannel | null>(null)

  // Recipients management dialog state
  const [recipientsDialogChannel, setRecipientsDialogChannel] = useState<MessageChannel | null>(null)
  const [recipients, setRecipients] = useState<string[]>([])
  const [newRecipientEmail, setNewRecipientEmail] = useState('')
  const [loadingRecipients, setLoadingRecipients] = useState(false)
  const [addingRecipient, setAddingRecipient] = useState(false)
  const [recipientError, setRecipientError] = useState<string | null>(null)

  // Fetch recipients for a channel
  const fetchRecipients = async (channelName: string) => {
    setLoadingRecipients(true)
    try {
      const response = await fetch(getApiUrl(`/messages/channels/${encodeURIComponent(channelName)}/recipients`), {
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
        },
      })
      if (response.ok) {
        const result = await response.json()
        // Handle wrapped response format: { success: true, data: { recipients: [...] } }
        const recipients = result.data?.recipients || result.recipients || []
        setRecipients(recipients)
      }
    } catch (error) {
      handleError(error, { operation: 'Fetch recipients' })
    } finally {
      setLoadingRecipients(false)
    }
  }

  // Open recipients dialog
  const handleManageRecipients = async (channel: MessageChannel) => {
    setRecipientsDialogChannel(channel)
    setNewRecipientEmail('')
    await fetchRecipients(channel.name)
  }

  // Add recipient
  const handleAddRecipient = async () => {
    if (!recipientsDialogChannel || !newRecipientEmail.trim()) return

    setAddingRecipient(true)
    setRecipientError(null)
    try {
      const response = await fetch(
        getApiUrl(`/messages/channels/${encodeURIComponent(recipientsDialogChannel.name)}/recipients`),
        {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ email: newRecipientEmail.trim() }),
        }
      )
      if (response.ok) {
        const result = await response.json()
        // Handle wrapped response format: { success: true, data: { recipients: [...] } }
        const recipients = result.data?.recipients || result.recipients || []
        setRecipients(recipients)
        setNewRecipientEmail('')
        toast({
          title: t('success'),
          description: t('messages.channels.recipientAdded', 'Recipient added successfully'),
        })
        fetchChannels() // Refresh channel list
      } else {
        const text = await response.text()
        let errorMessage = t('messages.channels.addRecipientError', 'Failed to add recipient')
        try {
          if (text) {
            const result = JSON.parse(text)
            // Handle nested error format: { success: false, error: { message: "..." } }
            errorMessage = result.error?.message || result.message || result.error || errorMessage
          }
        } catch {
          // Ignore JSON parse errors
        }
        setRecipientError(errorMessage)
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setRecipientError(message)
    } finally {
      setAddingRecipient(false)
    }
  }

  // Remove recipient
  const handleRemoveRecipient = async (email: string) => {
    if (!recipientsDialogChannel) return

    try {
      const response = await fetch(
        getApiUrl(`/messages/channels/${encodeURIComponent(recipientsDialogChannel.name)}/recipients/${encodeURIComponent(email)}`),
        {
          method: 'DELETE',
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
          },
        }
      )
      if (response.ok) {
        const result = await response.json()
        // Handle wrapped response format: { success: true, data: { recipients: [...] } }
        const recipients = result.data?.recipients || result.recipients || []
        setRecipients(recipients)
        toast({
          title: t('success'),
          description: t('messages.channels.recipientRemoved', 'Recipient removed successfully'),
        })
        fetchChannels() // Refresh channel list
      } else {
        const text = await response.text()
        let errorMessage = t('messages.channels.removeRecipientError', 'Failed to remove recipient')
        try {
          if (text) {
            const result = JSON.parse(text)
            errorMessage = result.message || result.error || errorMessage
          }
        } catch {
          // Ignore JSON parse errors
        }
        throw new Error(errorMessage)
      }
    } catch (error) {
      handleError(error, { operation: 'Remove recipient' })
    }
  }

  // Test a channel
  const handleTestChannel = async (channelName: string) => {
    setTestingChannel(channelName)
    try {
      const response = await fetch(getApiUrl(`/messages/channels/${encodeURIComponent(channelName)}/test`), {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          message: 'Test message from NeoMind',
          title: 'Channel Test',
        }),
      })
      const result = await response.json()
      if (result.success) {
        setTestResults(prev => ({ ...prev, [channelName]: { success: true, message: result.message || 'Test sent successfully' } }))
        toast({ title: t('common.success'), description: `Channel "${channelName}" test successful` })
      } else {
        throw new Error(result.message || 'Test failed')
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error)
      setTestResults(prev => ({ ...prev, [channelName]: { success: false, message: errorMsg } }))
      toast({ title: t('common.failed'), description: errorMsg, variant: 'destructive' })
    } finally {
      setTestingChannel(null)
    }
  }

  // Toggle channel enabled state
  const handleToggleEnabled = async (channelName: string, enabled: boolean) => {
    try {
      const response = await fetch(getApiUrl(`/messages/channels/${encodeURIComponent(channelName)}/enabled`), {
        method: 'PUT',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ enabled }),
      })
      if (response.ok) {
        // Update local state
        setChannels(prev => prev.map(c =>
          c.name === channelName ? { ...c, enabled } : c
        ))
        toast({
          title: t('common.success'),
          description: enabled
            ? t('messages.channels.enableSuccess', 'Channel enabled')
            : t('messages.channels.disableSuccess', 'Channel disabled')
        })
      } else {
        // Try to parse error response, handle empty responses gracefully
        let errorMessage = 'Failed to update channel'
        try {
          const text = await response.text()
          if (text) {
            const result = JSON.parse(text)
            errorMessage = result.message || result.error || errorMessage
          }
        } catch {
          // Ignore JSON parse errors
        }
        throw new Error(errorMessage)
      }
    } catch (error) {
      handleError(error, { operation: 'Toggle channel', showToast: true })
    }
  }

  // View channel details
  const handleViewChannel = async (channelName: string) => {
    const channel = channels.find(c => c.name === channelName)
    if (channel) {
      setViewChannel(channel)
    }
  }

  // Delete channel
  const handleDeleteChannel = async (channelName: string) => {
    const confirmed = await confirm({
      title: t('messages.channels.deleteChannel', 'Delete Channel'),
      description: t('messages.channels.confirmDelete', 'Are you sure you want to delete the channel "{{name}}"?', { name: channelName }),
      variant: 'destructive',
    })
    if (!confirmed) return

    try {
      const response = await fetch(getApiUrl(`/messages/channels/${encodeURIComponent(channelName)}`), {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
        },
      })
      if (response.ok) {
        toast({
          title: t('success'),
          description: t('messages.channels.deleteSuccess', 'Channel deleted successfully'),
        })
        fetchChannels()
      } else {
        const text = await response.text()
        let errorMessage = t('messages.channels.deleteError', 'Failed to delete channel')
        try {
          if (text) {
            const result = JSON.parse(text)
            errorMessage = result.message || result.error || errorMessage
          }
        } catch {
          // Ignore JSON parse errors
        }
        throw new Error(errorMessage)
      }
    } catch (error) {
      handleError(error, { operation: 'Delete channel', showToast: true })
    }
  }

  // Calculate paginated messages
  // On mobile: show cumulative data (all pages up to current)
  // On desktop: show only current page
  const paginatedMessages = useMemo(() => {
    if (isMobile) {
      // Mobile: show all data from page 1 to current page (cumulative)
      return messages.slice(0, messagePage * messagesPerPage)
    } else {
      // Desktop: show only current page
      return messages.slice(
        (messagePage - 1) * messagesPerPage,
        messagePage * messagesPerPage
      )
    }
  }, [messages, messagePage, messagesPerPage, isMobile])

  // Channels state
  const [channels, setChannels] = useState<MessageChannel[]>([])

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
      // Fetch both notifications and delivery logs in parallel
      const [messagesResponse, deliveryLogsResponse] = await Promise.all([
        fetch(getApiUrl('/messages'), {
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
          },
        }),
        fetch(getApiUrl('/messages/delivery-logs?hours=24'), {
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
          },
        }).catch(() => null) // Gracefully handle if delivery logs endpoint is not available
      ])

      const rawData: unknown = await messagesResponse.json()

      // Handle different response formats from messages endpoint
      let messagesArray: RawNotificationMessage[] = []
      if (Array.isArray(rawData)) {
        messagesArray = rawData as RawNotificationMessage[]
      } else {
        // Try to extract messages from various response formats
        const apiResponse = rawData as MessagesApiResponse & { data?: RawNotificationMessage[] | { messages?: RawNotificationMessage[] } }
        if (apiResponse.messages && Array.isArray(apiResponse.messages)) {
          messagesArray = apiResponse.messages
        } else if (apiResponse.data) {
          if (Array.isArray(apiResponse.data)) {
            messagesArray = apiResponse.data
          } else if (apiResponse.data.messages && Array.isArray(apiResponse.data.messages)) {
            messagesArray = apiResponse.data.messages
          }
        }
      }

      // Convert to NotificationMessage format
      let messages: NotificationMessage[] = messagesArray.map((msg: RawNotificationMessage) => ({
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
        message_type: (msg.metadata?.message_type as MessageType) || 'notification',
      }))

      // Fetch and process delivery logs
      let allDeliveryLogs: DeliveryLog[] = []
      if (deliveryLogsResponse && deliveryLogsResponse.ok) {
        const logsData = await deliveryLogsResponse.json()
        allDeliveryLogs = logsData.logs || logsData || []
        setDeliveryLogs(allDeliveryLogs)

        // Convert delivery logs to message format for display
        const deliveryLogMessages: NotificationMessage[] = allDeliveryLogs.map((log: DeliveryLog) => ({
          id: log.id,
          category: 'data_push',
          severity: log.status === 'success' ? 'info' : log.status === 'failed' ? 'critical' : 'warning',
          title: `DataPush: ${log.channel_name}`,
          message: log.error_message || log.payload_summary || 'Data pushed',
          source: log.channel_name,
          source_type: 'data_push',
          timestamp: log.created_at,
          status: log.status === 'success' ? 'resolved' : log.status === 'failed' ? 'active' : 'active',
          tags: [],
          metadata: { delivery_log: true, retry_count: log.retry_count },
          message_type: 'data_push' as MessageType,
        }))

        // Merge messages with delivery logs
        messages = [...messages, ...deliveryLogMessages]
      }

      // Extract unique categories from the data
      const categories = [...new Set(messages.map(m => m.category))].sort()
      setAvailableCategories(categories)

      // Apply filters using Sets
      if (selectedSeverities.size > 0) {
        messages = messages.filter((m: NotificationMessage) => selectedSeverities.has(m.severity as MessageSeverity))
      }
      if (selectedStatuses.size > 0) {
        messages = messages.filter((m: NotificationMessage) => selectedStatuses.has(m.status as MessageStatus))
      }
      if (selectedCategories.size > 0) {
        messages = messages.filter((m: NotificationMessage) => selectedCategories.has(m.category))
      }
      // Filter by message type
      if (selectedMessageTypes.size > 0) {
        messages = messages.filter((m: NotificationMessage) => {
          const msgType = m.message_type || 'notification'
          return selectedMessageTypes.has(msgType)
        })
      }

      // Sort by timestamp descending (handle invalid timestamps)
      messages.sort((a: NotificationMessage, b: NotificationMessage) => {
        const aTime = new Date(a.timestamp).getTime()
        const bTime = new Date(b.timestamp).getTime()
        // If either timestamp is invalid, treat it as oldest
        if (isNaN(aTime)) return 1
        if (isNaN(bTime)) return -1
        return bTime - aTime
      })

      setMessages(messages)
      setMessagePage(1) // Reset to first page when data changes
    } catch (error) {
      handleError(error, { operation: 'Fetch messages', showToast: false })
    } finally {
      setLoading(false)
    }
  }, [selectedSeverities, selectedStatuses, selectedCategories, selectedMessageTypes])

  // Fetch channels
  const fetchChannels = useCallback(async () => {
    setLoading(true)
    try {
      const response = await api.listMessageChannels()
      setChannels(response.channels || [])
    } catch (error) {
      handleError(error, { operation: 'Fetch channels', showToast: false })
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
  }, [activeTab, fetchMessages, fetchChannels, selectedSeverities, selectedStatuses, selectedCategories])

  // Message actions - using messages API endpoints
  const handleAcknowledge = async (id: string) => {
    try {
      const response = await fetch(getApiUrl(`/messages/${id}/acknowledge`), {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
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
      handleError(error, { operation: 'Acknowledge message', showToast: true })
      toast({ title: t('messages.acknowledgeError', 'Failed'), variant: 'destructive' })
    }
  }

  const handleResolve = async (id: string) => {
    try {
      const response = await fetch(getApiUrl(`/messages/${id}/resolve`), {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
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
      handleError(error, { operation: 'Resolve message', showToast: true })
      toast({ title: t('messages.resolveError', 'Failed'), variant: 'destructive' })
    }
  }

  const handleArchive = async (id: string) => {
    try {
      const response = await fetch(getApiUrl(`/messages/${id}/archive`), {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
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
      handleError(error, { operation: 'Archive message', showToast: true })
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
      const response = await fetch(getApiUrl(`/messages/${id}`), {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
        },
      })
      if (response.ok) {
        setMessages(prev => prev.filter(m => m.id !== id))
        toast({ title: t('messages.deleteSuccess', 'Deleted') })
      } else {
        throw new Error('Failed to delete')
      }
    } catch (error) {
      handleError(error, { operation: 'Delete message', showToast: true })
      toast({ title: t('messages.deleteError', 'Failed to delete'), variant: 'destructive' })
    }
  }

  const tabs = [
    { value: 'messages' as TabValue, label: t('messages.tabs.messages'), icon: <MessageSquare className="h-4 w-4" /> },
    { value: 'channels' as TabValue, label: t('messages.tabs.channels'), icon: <Network className="h-4 w-4" /> },
  ]

  const actions = [
    ...(activeTab === 'messages' ? [
      { label: t('messages.create'), onClick: () => setCreateDialogOpen(true) },
    ] : []),
    ...(activeTab === 'channels' ? [
      { label: t('messages.channels.goToSettings', 'Settings'), icon: <Settings className="h-4 w-4" />, variant: 'outline' as const, onClick: () => navigate('/settings?tab=alert-channels') },
    ] : []),
    { label: t('refresh'), variant: 'outline' as const, onClick: activeTab === 'messages' ? fetchMessages : fetchChannels, disabled: loading },
  ]

  // Filter dropdown for actionsExtra
  const filterExtra = activeTab === 'messages' ? (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="h-9 gap-2">
          <Filter className="h-4 w-4" />
          {t('messages.filter.title')}
          {getActiveFilterCount() > 0 && (
            <Badge variant="secondary" className="h-5 px-1.5 text-xs">
              {getActiveFilterCount()}
            </Badge>
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56 max-h-[70vh] overflow-y-auto">
        {/* Severity Filter */}
        <div className="px-2 py-1.5">
          <p className="text-xs font-medium text-muted-foreground mb-1">{t('messages.severity.label')}</p>
          {(['info', 'warning', 'critical', 'emergency'] as MessageSeverity[]).map((sev) => (
            <DropdownMenuCheckboxItem
              key={sev}
              checked={selectedSeverities.has(sev)}
              onCheckedChange={() => toggleSeverity(sev)}
            >
              <div className="flex items-center gap-2">
                {sev === 'info' && <Info className="h-3.5 w-3.5 text-blue-500" />}
                {sev === 'warning' && <AlertTriangle className="h-3.5 w-3.5 text-yellow-500" />}
                {sev === 'critical' && <AlertCircle className="h-3.5 w-3.5 text-orange-500" />}
                {sev === 'emergency' && <ShieldAlert className="h-3.5 w-3.5 text-red-500" />}
                {t(`messages.severity.${sev}`)}
              </div>
            </DropdownMenuCheckboxItem>
          ))}
        </div>

        <DropdownMenuSeparator />

        {/* Status Filter */}
        <div className="px-2 py-1.5">
          <p className="text-xs font-medium text-muted-foreground mb-1">{t('messages.status.label')}</p>
          {(['active', 'acknowledged', 'resolved', 'archived'] as MessageStatus[]).map((stat) => (
            <DropdownMenuCheckboxItem
              key={stat}
              checked={selectedStatuses.has(stat)}
              onCheckedChange={() => toggleStatus(stat)}
            >
              {t(`messages.status.${stat}`)}
            </DropdownMenuCheckboxItem>
          ))}
        </div>

        <DropdownMenuSeparator />

        {/* Category Filter */}
        <div className="px-2 py-1.5">
          <p className="text-xs font-medium text-muted-foreground mb-1">{t('messages.category.label')}</p>
          {availableCategories.map((cat) => {
            const config = getCategoryConfig(cat)
            const Icon = config.icon
            return (
              <DropdownMenuCheckboxItem
                key={cat}
                checked={selectedCategories.has(cat)}
                onCheckedChange={() => toggleCategory(cat)}
              >
                <div className="flex items-center gap-2">
                  <Icon className="h-3.5 w-3.5" />
                  {t(config.label)}
                </div>
              </DropdownMenuCheckboxItem>
            )
          })}
        </div>

        <DropdownMenuSeparator />

        {/* Message Type Filter */}
        <div className="px-2 py-1.5">
          <p className="text-xs font-medium text-muted-foreground mb-1">{t('messages.type.label')}</p>
          {(['notification', 'data_push'] as const).map((type) => (
            <DropdownMenuCheckboxItem
              key={type}
              checked={selectedMessageTypes.has(type)}
              onCheckedChange={() => toggleMessageType(type)}
            >
              <div className="flex items-center gap-2">
                {type === 'notification' && <Bell className="h-3.5 w-3.5 text-blue-500" />}
                {type === 'data_push' && <Send className="h-3.5 w-3.5 text-purple-500" />}
                {t(`messages.type.${type}`)}
              </div>
            </DropdownMenuCheckboxItem>
          ))}
        </div>

        {hasActiveFilters && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={clearAllFilters}>
              <X className="h-4 w-4 mr-2" />
              {t('messages.filter.clear')}
            </DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  ) : null

  return (
    <>
      <PageLayout
        title={t('messages.title')}
        subtitle={t('messages.description')}
        hideFooterOnMobile
        headerContent={
          <PageTabsBar
            tabs={tabs}
            activeTab={activeTab}
            onTabChange={(v) => handleTabChange(v as TabValue)}
            actions={actions}
            actionsExtra={filterExtra}
          />
        }
        footer={
          activeTab === 'messages' && messages.length > messagesPerPage ? (
            <Pagination
              total={messages.length}
              pageSize={messagesPerPage}
              currentPage={messagePage}
              onPageChange={setMessagePage}
            />
          ) : undefined
        }
      >
        {/* Messages Tab */}
        <PageTabsContent value="messages" activeTab={activeTab} className="flex flex-col overflow-hidden">
          {/* Active Filter Chips */}
          {hasActiveFilters && (
            <div className="flex flex-wrap items-center gap-2 mb-4 shrink-0">
              {Array.from(selectedSeverities).map((sev) => (
                <Badge
                  key={`sev-${sev}`}
                  variant="secondary"
                  className="gap-1 pr-1 cursor-pointer hover:bg-secondary/80"
                  onClick={() => toggleSeverity(sev)}
                >
                  {sev === 'info' && <Info className="h-3 w-3 text-blue-500" />}
                  {sev === 'warning' && <AlertTriangle className="h-3 w-3 text-yellow-500" />}
                  {sev === 'critical' && <AlertCircle className="h-3 w-3 text-orange-500" />}
                  {sev === 'emergency' && <ShieldAlert className="h-3 w-3 text-red-500" />}
                  {t(`messages.severity.${sev}`)}
                  <X className="h-3 w-3 ml-1 text-muted-foreground" />
                </Badge>
              ))}

              {Array.from(selectedStatuses).map((stat) => (
                <Badge
                  key={`stat-${stat}`}
                  variant="secondary"
                  className="gap-1 pr-1 cursor-pointer hover:bg-secondary/80"
                  onClick={() => toggleStatus(stat)}
                >
                  {t(`messages.status.${stat}`)}
                  <X className="h-3 w-3 ml-1 text-muted-foreground" />
                </Badge>
              ))}

              {Array.from(selectedCategories).map((cat) => {
                const config = getCategoryConfig(cat)
                const Icon = config.icon
                return (
                  <Badge
                    key={`cat-${cat}`}
                    variant="secondary"
                    className="gap-1 pr-1 cursor-pointer hover:bg-secondary/80"
                    onClick={() => toggleCategory(cat)}
                  >
                    <Icon className="h-3 w-3" />
                    {t(config.label)}
                    <X className="h-3 w-3 ml-1 text-muted-foreground" />
                  </Badge>
                )
              })}

              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs"
                onClick={clearAllFilters}
              >
                {t('messages.filter.clearAll')}
              </Button>
            </div>
          )}

          {/* Messages Table - Responsive (Desktop: Table, Mobile: Cards) */}
          <div className="overflow-auto h-full">
            <ResponsiveTable
              columns={[
                {
                  key: 'severity',
                  label: '',
                  width: 'w-[50px]',
                  className: 'text-center',
                },
                {
                  key: 'title',
                  label: t('messages.formTitle.label'),
                },
                {
                  key: 'message',
                  label: t('messages.content.label'),
                },
                {
                  key: 'severityBadge',
                  label: t('messages.severity.label'),
                  width: 'w-[90px]',
                  align: 'center',
                },
                {
                  key: 'category',
                  label: t('messages.category.label'),
                  width: 'w-[90px]',
                },
                {
                  key: 'status',
                  label: t('messages.status.label'),
                  width: 'w-[90px]',
                  align: 'center',
                },
                {
                  key: 'timestamp',
                  label: t('common:createdAt'),
                  width: 'w-[130px]',
                },
              ]}
              data={paginatedMessages as unknown as Record<string, unknown>[]}
              rowKey={(msg) => (msg as unknown as NotificationMessage).id}
              renderCell={(columnKey, rowData) => {
                const message = rowData as unknown as NotificationMessage
                const severityConfig = SEVERITY_CONFIG[message.severity] || SEVERITY_CONFIG.info
                const categoryConfig = getCategoryConfig(message.category)
                const statusConfig = STATUS_CONFIG[message.status] || STATUS_CONFIG.active
                const SeverityIcon = severityConfig.icon
                const CategoryIcon = categoryConfig.icon

                switch (columnKey) {
                  case 'severity':
                    return <SeverityIcon className={cn("h-4 w-4", severityConfig.color)} />
                  case 'title':
                    return (
                      <div className="font-medium text-sm truncate" title={message.title}>
                        {message.title}
                      </div>
                    )
                  case 'message':
                    return (
                      <div className="text-sm">
                        <div className="text-muted-foreground line-clamp-2">{message.message}</div>
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
                      </div>
                    )
                  case 'severityBadge':
                    return (
                      <Badge variant="outline" className={cn("text-xs", severityConfig.bgColor, severityConfig.color)}>
                        {t(`messages.severity.${message.severity}`)}
                      </Badge>
                    )
                  case 'category':
                    return (
                      <div className="flex items-center gap-1.5">
                        <CategoryIcon className="h-3.5 w-3.5 text-muted-foreground" />
                        <span className="text-xs">{t(categoryConfig.label)}</span>
                      </div>
                    )
                  case 'status':
                    return (
                      <Badge variant={statusConfig.variant} className="text-xs">
                        {t(statusConfig.label)}
                      </Badge>
                    )
                  case 'timestamp':
                    return (
                      <span className="text-xs text-muted-foreground">
                        {formatTimestamp(message.timestamp, false)}
                      </span>
                    )
                  default:
                    return null
                }
              }}
              getRowClassName={(rowData) => {
                const message = rowData as unknown as NotificationMessage
                return (message.status === 'resolved' || message.status === 'archived') ? 'opacity-60' : ''
              }}
              actions={[
                {
                  label: t('messages.acknowledge'),
                  icon: <Eye className="h-4 w-4" />,
                  show: (rowData) => (rowData as unknown as NotificationMessage).status === 'active',
                  onClick: (rowData) => {
                    const message = rowData as unknown as NotificationMessage
                    handleAcknowledge(message.id)
                  },
                },
                {
                  label: t('messages.resolve'),
                  icon: <Eye className="h-4 w-4" />,
                  show: (rowData) => {
                    const status = (rowData as unknown as NotificationMessage).status
                    return status !== 'resolved' && status !== 'archived'
                  },
                  onClick: (rowData) => {
                    const message = rowData as unknown as NotificationMessage
                    handleResolve(message.id)
                  },
                },
                {
                  label: t('delete'),
                  icon: <Trash2 className="h-4 w-4" />,
                  variant: 'destructive',
                  onClick: (rowData) => {
                    const message = rowData as unknown as NotificationMessage
                    handleDelete(message.id)
                  },
                },
              ]}
              loading={loading}
              emptyState={
                !loading && messages.length === 0 ? (
                  <EmptyStateInline
                    title={t('messages.empty.title')}
                  />
                ) : undefined
              }
            />
            {/* Pagination - shows as footer on desktop, infinite scroll trigger on mobile */}
          </div>
        </PageTabsContent>

        {/* Channels Tab */}
        <PageTabsContent value="channels" activeTab={activeTab}>
          {/* Channels Responsive Table */}
          <ResponsiveTable
            columns={[
              {
                key: 'channel',
                label: (
                  <div className="flex items-center gap-2">
                    <Bell className="h-4 w-4" />
                    {t('messages.channels.name')}
                  </div>
                ),
              },
              {
                key: 'type',
                label: t('messages.channels.type'),
                align: 'center',
              },
              {
                key: 'status',
                label: t('status'),
                align: 'center',
              },
            ]}
            data={channels as unknown as Record<string, unknown>[]}
            rowKey={(ch) => (ch as unknown as MessageChannel).name}
            renderCell={(columnKey, rowData) => {
              const channel = rowData as unknown as MessageChannel
              const config: Record<string, { icon: typeof Bell; color: string }> = {
                console: { icon: Bell, color: 'bg-gray-500/10 text-gray-500' },
                memory: { icon: RefreshCw, color: 'bg-blue-500/10 text-blue-500' },
                webhook: { icon: Megaphone, color: 'bg-green-500/10 text-green-500' },
                email: { icon: Bell, color: 'bg-purple-500/10 text-purple-500' },
              }
              const channelConfig = config[channel.channel_type] || config.console
              const ChannelIcon = channelConfig.icon
              const testResult = testResults[channel.name]

              switch (columnKey) {
                case 'channel':
                  return (
                    <div className="flex items-center gap-3">
                      <div className={`w-9 h-9 rounded-lg flex items-center justify-center transition-colors ${channelConfig.color}`}>
                        <ChannelIcon className="h-4 w-4" />
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <span className="font-medium text-sm">{channel.name}</span>
                          {(channel.channel_type === 'webhook' || channel.channel_type === 'email') && (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-6 px-2 text-xs"
                              onClick={() => handleTestChannel(channel.name)}
                              disabled={testingChannel === channel.name}
                            >
                              <TestTube className="h-3 w-3 mr-1" />
                              {testingChannel === channel.name ? 'Testing...' : 'Test'}
                            </Button>
                          )}
                        </div>
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <span>{channel.channel_type}</span>
                          {channel.channel_type === 'email' && channel.recipients && channel.recipients.length > 0 && (
                            <>
                              <span>·</span>
                              <span className="flex items-center gap-1">
                                <Mail className="h-3 w-3" />
                                {channel.recipients.length} {t('messages.channels.recipients', 'recipients')}
                              </span>
                            </>
                          )}
                        </div>
                        {testResult && (
                          <div className={`text-xs mt-1 ${testResult.success ? 'text-green-500' : 'text-red-500'}`}>
                            {testResult.success ? '✓ ' : '✗ '}
                            {testResult.message}
                          </div>
                        )}
                      </div>
                    </div>
                  )
                case 'type':
                  return (
                    <Badge variant="outline" className="text-xs">
                      {channel.channel_type}
                    </Badge>
                  )
                case 'status':
                  return (
                    <Badge variant={channel.enabled ? 'default' : 'secondary'} className="text-xs">
                      {channel.enabled ? t('enabled') : t('disabled')}
                    </Badge>
                  )
                default:
                  return null
              }
            }}
            actions={[
              {
                label: t('view'),
                icon: <Eye className="h-4 w-4" />,
                onClick: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  handleViewChannel(channel.name)
                },
              },
              {
                label: t('messages.channels.manageRecipients', 'Manage Recipients'),
                icon: <UserPlus className="h-4 w-4" />,
                show: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  return channel.channel_type === 'email'
                },
                onClick: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  handleManageRecipients(channel)
                },
              },
              {
                label: t('enable'),
                icon: <CheckCircle2 className="h-4 w-4" />,
                show: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  return !channel.enabled && channel.channel_type !== 'console' && channel.channel_type !== 'memory'
                },
                onClick: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  handleToggleEnabled(channel.name, true)
                },
              },
              {
                label: t('disable'),
                icon: <X className="h-4 w-4" />,
                show: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  return channel.enabled && channel.channel_type !== 'console' && channel.channel_type !== 'memory'
                },
                onClick: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  handleToggleEnabled(channel.name, false)
                },
              },
              {
                label: t('delete'),
                icon: <Trash2 className="h-4 w-4" />,
                variant: 'destructive',
                show: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  return channel.channel_type !== 'console' && channel.channel_type !== 'memory'
                },
                onClick: (rowData) => {
                  const channel = rowData as unknown as MessageChannel
                  handleDeleteChannel(channel.name)
                },
              },
            ]}
            loading={loading}
            emptyState={
              !loading && channels.length === 0 ? (
                <EmptyStateInline title={t('messages.channels.empty.title')} />
              ) : undefined
            }
          />
        </PageTabsContent>
      </PageLayout>

      {/* Mobile Bottom Navigation */}
      <PageTabsBottomNav
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(v) => handleTabChange(v as TabValue)}
      />

      <CreateMessageDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        onCreate={async (req) => {
          // Use the messages API to create a new message
          const response = await fetch(getApiUrl('/messages'), {
            method: 'POST',
            headers: {
              'Authorization': `Bearer ${localStorage.getItem('neomind_token') || sessionStorage.getItem('neomind_token_session') || ''}`,
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

      {/* View Channel Dialog */}
      <Dialog open={!!viewChannel} onOpenChange={(open) => !open && setViewChannel(null)}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t('messages.channels.channelDetails', 'Channel Details')}</DialogTitle>
          </DialogHeader>
          {viewChannel && (
            <div className="space-y-4 py-4">
              <div className="grid grid-cols-3 gap-4">
                <div>
                  <Label className="text-muted-foreground">{t('messages.channels.name')}</Label>
                  <div className="font-medium">{viewChannel.name}</div>
                </div>
                <div>
                  <Label className="text-muted-foreground">{t('messages.channels.type')}</Label>
                  <div className="font-medium capitalize">{viewChannel.channel_type}</div>
                </div>
                <div>
                  <Label className="text-muted-foreground">{t('status')}</Label>
                  <Badge variant={viewChannel.enabled ? 'default' : 'secondary'}>
                    {viewChannel.enabled ? t('enabled') : t('disabled')}
                  </Badge>
                </div>
              </div>

              {/* Webhook config */}
              {viewChannel.channel_type === 'webhook' && viewChannel.config && (
                <div className="space-y-3">
                  <Label className="text-muted-foreground">{t('messages.channels.config')}</Label>
                  <div className="bg-muted/50 rounded-lg p-3 text-sm font-mono break-all">
                    {(() => {
                      const cfg = viewChannel.config as Record<string, unknown>
                      return typeof cfg === 'object' && cfg !== null && 'url' in cfg
                        ? String(cfg.url)
                        : JSON.stringify(viewChannel.config, null, 2)
                    })()}
                  </div>
                </div>
              )}

              {/* Email config */}
              {viewChannel.channel_type === 'email' && viewChannel.config && (
                <div className="space-y-3">
                  <Label className="text-muted-foreground">{t('messages.channels.config')}</Label>
                  <div className="bg-muted/50 rounded-lg p-3 text-sm space-y-1">
                    {(() => {
                      const cfg = viewChannel.config as Record<string, unknown>
                      return (
                        <>
                          {cfg && 'smtp_server' in cfg && (
                            <div>{t('messages.channels.smtpServer')}: {String(cfg.smtp_server)}</div>
                          )}
                          {cfg && 'smtp_port' in cfg && (
                            <div>{t('messages.channels.smtpPort')}: {String(cfg.smtp_port)}</div>
                          )}
                          {cfg && 'from_address' in cfg && (
                            <div>{t('messages.channels.emailFrom')}: {String(cfg.from_address)}</div>
                          )}
                        </>
                      )
                    })()}
                  </div>
                </div>
              )}

              <div className="flex justify-end gap-2 pt-4">
                {viewChannel.channel_type !== 'console' && viewChannel.channel_type !== 'memory' && (
                  <>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        handleTestChannel(viewChannel.name)
                        setViewChannel(null)
                      }}
                      disabled={testingChannel === viewChannel.name}
                    >
                      <TestTube className="h-4 w-4 mr-1" />
                      {t('messages.channels.test')}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        handleToggleEnabled(viewChannel.name, !viewChannel.enabled)
                        setViewChannel(null)
                      }}
                    >
                      {viewChannel.enabled ? t('disable') : t('enable')}
                    </Button>
                  </>
                )}
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

      {/* Recipients Management Dialog */}
      <UnifiedFormDialog
        open={!!recipientsDialogChannel}
        onOpenChange={(open) => {
          if (!open) {
            setRecipientsDialogChannel(null)
            setRecipientError(null)
            setNewRecipientEmail('')
          }
        }}
        title={t('messages.channels.manageRecipients', 'Manage Recipients')}
        description={recipientsDialogChannel?.name || ''}
        icon={<Mail className="h-5 w-5" />}
        width="sm"
        loading={loadingRecipients}
        submitError={recipientError}
        showCancelButton={true}
        cancelLabel={t('close')}
        onSubmit={async () => {
          // Dialog doesn't have a primary submit action - just close
          setRecipientsDialogChannel(null)
        }}
        submitLabel={t('done', 'Done')}
      >
        <div className="space-y-4">
          {/* Add new recipient */}
          <FormField label={t('messages.channels.addRecipient', 'Add Recipient')}>
            <div className="flex gap-2">
              <Input
                type="email"
                placeholder={t('messages.channels.emailPlaceholder', 'email@example.com')}
                value={newRecipientEmail}
                onChange={(e) => {
                  setNewRecipientEmail(e.target.value)
                  if (recipientError) setRecipientError(null)
                }}
                onKeyDown={(e) => e.key === 'Enter' && (e.preventDefault(), handleAddRecipient())}
                disabled={addingRecipient}
                className="flex-1"
              />
              <Button
                onClick={handleAddRecipient}
                disabled={addingRecipient || !newRecipientEmail.trim()}
                size="default"
              >
                {addingRecipient ? t('adding', 'Adding...') : t('add')}
              </Button>
            </div>
          </FormField>

          {/* Recipients list */}
          <div className="space-y-2">
            <Label className="text-muted-foreground text-xs">
              {t('messages.channels.currentRecipients', 'Current Recipients')} ({recipients.length})
            </Label>
            {loadingRecipients ? (
              <div className="text-center py-4 text-muted-foreground">
                {t('loading')}
              </div>
            ) : recipients.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground border border-dashed rounded-lg">
                <Mail className="h-8 w-8 mx-auto mb-2 opacity-50" />
                <p className="text-sm">{t('messages.channels.noRecipients', 'No recipients configured')}</p>
              </div>
            ) : (
              <div className="space-y-2 max-h-[300px] overflow-y-auto">
                {recipients.map((email) => (
                  <div
                    key={email}
                    className="flex items-center justify-between bg-muted/50 rounded-lg px-3 py-2"
                  >
                    <span className="text-sm truncate flex-1">{email}</span>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive shrink-0 ml-2"
                      onClick={() => handleRemoveRecipient(email)}
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </UnifiedFormDialog>
    </>
  )
}
