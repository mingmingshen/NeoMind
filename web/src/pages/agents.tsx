/**
 * NeoMind AI Agents Page
 *
 * User-defined AI Agents for autonomous IoT automation.
 * Card grid layout with detail dialog for viewing individual agent details.
 * Uses WebSocket events for real-time agent status updates.
 * Also includes System Memory tab for viewing aggregated memory.
 */

import { useState, useCallback, useEffect, useRef } from "react"
import { useLocation, useNavigate } from "react-router-dom"
import { useTranslation } from "react-i18next"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabsBar, PageTabsContent, PageTabsBottomNav } from "@/components/shared"
import { api } from "@/lib/api"
import { useToast } from "@/hooks/use-toast"
import { confirm } from "@/hooks/use-confirm"
import { useEvents } from "@/hooks/useEvents"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { useIsMobile } from "@/hooks/useMobile"
import { Loader2, Bot, Plus, Brain, Cpu, RefreshCw, Settings, Sparkles, Zap, BookOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { EmptyState } from "@/components/shared/EmptyState"
import type { AiAgent, AiAgentDetail, Extension, ExtensionDataSourceInfo, TransformDataSourceInfo } from "@/types"
import type { AgentExecutionStartedEvent, AgentExecutionCompletedEvent, AgentThinkingEvent } from "@/lib/events"

// Import components
import { AgentCard } from "./agents-components/AgentCard"
import { AgentEditorFullScreen } from "./agents-components/AgentEditorFullScreen"
import { ExecutionDetailDialog } from "./agents-components/ExecutionDetailDialog"
import { AgentDetailPanel } from "./agents-components/AgentDetailPanel"
import { MemoryPanel } from "./agents-components/MemoryPanel"
import { SkillsPanel, type SkillsPanelHandle } from "./agents-components/SkillsPanel"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function AgentsPage() {
  const { t: tCommon } = useTranslation('common')
  const { t: tAgent } = useTranslation('agents')
  const { toast } = useToast()
  const { handleError } = useErrorHandler()
  const location = useLocation()
  const navigate = useNavigate()

  // Determine active tab from URL path
  const getTabFromPath = () => {
    if (location.pathname.includes('/agents/memory')) {
      return 'memory'
    }
    if (location.pathname.includes('/agents/skills')) {
      return 'skills'
    }
    return 'agents'
  }
  const activeTab = getTabFromPath()

  // Handle tab change by navigating to the appropriate URL
  const handleTabChange = (tab: string) => {
    if (tab === 'memory') {
      navigate('/agents/memory')
    } else if (tab === 'skills') {
      navigate('/agents/skills')
    } else {
      navigate('/agents')
    }
  }

  // Dialog states
  const [showAgentDialog, setShowAgentDialog] = useState(false)
  const [executionDetailOpen, setExecutionDetailOpen] = useState(false)
  const [detailDialogOpen, setDetailDialogOpen] = useState(false)

  // Dialog data states
  const [detailAgentId, setDetailAgentId] = useState('')
  const [detailExecutionId, setDetailExecutionId] = useState('')

  // Editing states
  const [editingAgent, setEditingAgent] = useState<AiAgentDetail | undefined>(undefined)
  const [selectedAgent, setSelectedAgent] = useState<AiAgentDetail | null>(null)

  // Data state
  const [agents, setAgents] = useState<AiAgent[]>([])
  const [loading, setLoading] = useState(false)
  const [memoryRefreshKey, setMemoryRefreshKey] = useState(0)
  const [isExtracting, setIsExtracting] = useState(false)
  const memoryPanelRef = useRef<{
    openConfig: () => void
    triggerExtract: () => void
    isExtracting: boolean
  }>(null)
  const skillsPanelRef = useRef<SkillsPanelHandle>(null)

  // Track executing agents for real-time updates with timestamps for timeout
  const [executingAgents, setExecutingAgents] = useState<Map<string, number>>(new Map())
  // Track current thinking state per agent
  const [agentThinking, setAgentThinking] = useState<Record<string, string>>({})

  // Resources for dialogs
  const [devices, setDevices] = useState<any[]>([])
  const [deviceTypes, setDeviceTypes] = useState<any[]>([])
  const [extensions, setExtensions] = useState<Extension[]>([])
  const [extensionDataSources, setExtensionDataSources] = useState<ExtensionDataSourceInfo[]>([])

  // Fetch data
  const loadItems = useCallback(async () => {
    setLoading(true)
    try {
      // Load devices for dialogs
      const devicesData = await api.getDevices()
      setDevices(devicesData.devices || [])

      // Load device types
      try {
        const typesData = await api.getDeviceTypes()
        setDeviceTypes(typesData.device_types || [])
      } catch {
        setDeviceTypes([])
      }

      // Load extensions for agent resources
      try {
        const [extData, dsData] = await Promise.all([
          api.listExtensions().catch((): Extension[] => []),
          api.listAllDataSources().catch((): (ExtensionDataSourceInfo | TransformDataSourceInfo)[] => []),
        ])
        setExtensions(extData)
        // Filter only extension data sources (exclude transform data sources)
        setExtensionDataSources(dsData.filter((source): source is ExtensionDataSourceInfo => 'extension_id' in source))
      } catch {
        setExtensions([])
        setExtensionDataSources([])
      }

      // Load agents
      const data = await api.listAgents()
      // Sort by created_at descending (newest first)
      setAgents((data.agents || []).sort((a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      ))
    } catch (error) {
      handleError(error, { operation: 'Load agents', showToast: false })
    } finally {
      setLoading(false)
    }
  }, [])

  // Load items on mount
  useEffect(() => {
    loadItems()
  }, [loadItems])

  // Listen to WebSocket events for real-time agent status updates
  useEvents({
    enabled: true,
    eventTypes: ['AgentExecutionStarted', 'AgentExecutionCompleted', 'AgentThinking'],
    onConnected: (connected) => {
      if (!connected) {
        // Connection lost - clear all executing states and reset agent statuses
        setExecutingAgents(new Map())
        setAgentThinking({})
        // Reset executing status in agents list - will be refreshed when reconnecting
        setAgents(prev => prev.map(agent =>
          agent.status === 'Executing'
            ? { ...agent, status: 'Active' as const, currentThinking: null }
            : { ...agent, currentThinking: null }
        ))
        // Also reset selected agent if present
        setSelectedAgent(prev => prev?.status === 'Executing'
          ? { ...prev, status: 'Active', currentThinking: null }
          : prev
        )
      } else {
        // Connection restored - refresh agent status from server
        loadItems()
      }
    },
    onEvent: (event) => {
      const eventData = event.data as { agent_id?: string }

      switch (event.type) {
        case 'AgentExecutionStarted': {
          const startedData = (event as AgentExecutionStartedEvent).data
          // Add to executing map with timestamp
          setExecutingAgents(prev => new Map(prev).set(startedData.agent_id, Date.now()))

          // Update the specific agent's status in the list
          setAgents(prev => prev.map(agent =>
            agent.id === startedData.agent_id
              ? { ...agent, status: 'Executing' as const }
              : agent
          ))

          // Update selected agent if it's the same one
          if (selectedAgent?.id === startedData.agent_id) {
            setSelectedAgent(prev => prev ? { ...prev, status: 'Executing' } : null)
          }
          break
        }

        case 'AgentExecutionCompleted': {
          const completedData = (event as AgentExecutionCompletedEvent).data

          // Remove from executing map immediately
          setExecutingAgents(prev => {
            const next = new Map(prev)
            next.delete(completedData.agent_id)
            return next
          })

          // Clear thinking state immediately
          setAgentThinking(prev => {
            const next = { ...prev }
            delete next[completedData.agent_id]
            return next
          })

          // Immediately update the agent's status in the list to Active or Error
          const newStatus: AiAgent['status'] = completedData.success ? 'Active' : 'Error'
          setAgents(prev => prev.map(agent =>
            agent.id === completedData.agent_id
              ? { ...agent, status: newStatus }
              : agent
          ))

          // Update selected agent if it's the same one
          if (selectedAgent?.id === completedData.agent_id) {
            setSelectedAgent(prev => prev ? {
              ...prev,
              status: newStatus
            } : null)
          }
          break
        }

        case 'AgentThinking': {
          const thinkingData = (event as AgentThinkingEvent).data
          // Update current thinking for this agent
          setAgentThinking(prev => ({
            ...prev,
            [thinkingData.agent_id]: thinkingData.description
          }))
          break
        }
      }
    },
  })

  // Auto-cleanup: Remove agents from executing state after timeout (5 minutes)
  useEffect(() => {
    const EXECUTION_TIMEOUT_MS = 5 * 60 * 1000 // 5 minutes

    const interval = setInterval(() => {
      const now = Date.now()
      setExecutingAgents(prev => {
        const next = new Map<string, number>()
        let hasChanges = false

        prev.forEach((timestamp, agentId) => {
          if (now - timestamp > EXECUTION_TIMEOUT_MS) {
            // Agent has been executing too long, remove it
            hasChanges = true
            // Also clear thinking state
            setAgentThinking(prevThinking => {
              const nextThinking = { ...prevThinking }
              delete nextThinking[agentId]
              return nextThinking
            })
            // Reload agents to get actual status from server
            loadItems()
          } else {
            next.set(agentId, timestamp)
          }
        })

        return hasChanges ? next : prev
      })
    }, 30000) // Check every 30 seconds

    return () => clearInterval(interval)
  }, [loadItems])

  // Handlers
  const handleCreate = () => {
    setEditingAgent(undefined)
    setShowAgentDialog(true)
  }

  const handleEdit = async (agent: AiAgent) => {
    try {
      const detail = await api.getAgent(agent.id)
      setEditingAgent(detail)
      setShowAgentDialog(true)
    } catch (error) {
      handleError(error, { operation: 'Load agent details', showToast: false })
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleDelete = async (agent: AiAgent) => {
    const confirmed = await confirm({
      title: tCommon('delete'),
      description: tAgent('deleteConfirm'),
      confirmText: tCommon('delete'),
      cancelText: tCommon('cancel'),
      variant: "destructive"
    })
    if (!confirmed) return

    try {
      await api.deleteAgent(agent.id)
      await loadItems()
      if (selectedAgent?.id === agent.id) {
        setSelectedAgent(null)
      }
      toast({
        title: tCommon('success'),
        description: tAgent('agentDeleted'),
      })
    } catch (error) {
      handleError(error, { operation: 'Delete agent', showToast: true })
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleToggleStatus = async (agent: AiAgent) => {
    try {
      const newStatus = agent.status === 'Active' ? 'paused' : 'active'
      await api.setAgentStatus(agent.id, newStatus)
      await loadItems()
    } catch (error) {
      handleError(error, { operation: 'Toggle agent status', showToast: true })
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleExecute = async (agent: AiAgent) => {
    // Immediately mark as executing for responsive UI
    setAgents(prev => prev.map(a =>
      a.id === agent.id ? { ...a, status: 'Executing' } : a
    ))
    setExecutingAgents(prev => new Map(prev).set(agent.id, Date.now()))

    try {
      // API returns immediately — execution runs in background
      await api.executeAgent(agent.id)
      toast({
        title: tCommon('success'),
        description: tAgent('executionStarted', { id: agent.name }),
      })
    } catch (error) {
      // Revert status on error
      setAgents(prev => prev.map(a =>
        a.id === agent.id ? { ...a, status: 'Active' } : a
      ))
      setExecutingAgents(prev => {
        const next = new Map(prev)
        next.delete(agent.id)
        return next
      })
      handleError(error, { operation: 'Execute agent', showToast: true })
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleSave = async (data: any) => {
    try {
      if (editingAgent) {
        await api.updateAgent(editingAgent.id, data)
      } else {
        await api.createAgent(data)
      }
      setShowAgentDialog(false)
      setEditingAgent(undefined)
      await loadItems()
      toast({
        title: tCommon('success'),
        description: editingAgent ? tAgent('agentUpdated') : tAgent('agentCreated'),
      })
    } catch (error) {
      handleError(error, { operation: 'Save agent', showToast: false })
      throw error
    }
  }

  // Open execution detail dialog
  const handleViewExecutionDetail = (agentId: string, executionId: string) => {
    setDetailAgentId(agentId)
    setDetailExecutionId(executionId)
    setExecutionDetailOpen(true)
  }

  // Open detail dialog for an agent
  const handleViewDetail = async (agent: AiAgent) => {
    try {
      const detail = await api.getAgent(agent.id)
      setSelectedAgent(detail)
      setDetailDialogOpen(true)
    } catch (error) {
      handleError(error, { operation: 'Load agent details for panel', showToast: false })
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  // Refresh detail when sheet is open
  useEffect(() => {
    if (detailDialogOpen && selectedAgent) {
      api.getAgent(selectedAgent.id).then(setSelectedAgent).catch(err =>
        handleError(err, { operation: 'Refresh agent details', showToast: false })
      )
    }
  }, [agents, detailDialogOpen, selectedAgent?.id, handleError])

  // Merge executing state from WebSocket with agent data
  // Only show Executing if agent is currently executing AND not paused/error in database
  const agentsWithExecutingStatus = agents.map(agent => {
    // If agent is paused or error in database, don't override with executing state
    if (agent.status === 'Paused' || agent.status === 'Error') {
      return {
        ...agent,
        currentThinking: null
      };
    }
    // Show Executing only if in executing set and currently Active/Executing in database
    return {
      ...agent,
      status: executingAgents.has(agent.id) ? 'Executing' : agent.status,
      currentThinking: executingAgents.has(agent.id) ? (agentThinking[agent.id] || null) : null
    };
  })

  const isMobile = useIsMobile()

  // Wrapper functions for memory operations with loading state
  const handleTriggerExtract = useCallback(async () => {
    if (isExtracting) return
    setIsExtracting(true)
    try {
      await memoryPanelRef.current?.triggerExtract()
    } finally {
      setIsExtracting(false)
    }
  }, [isExtracting])

  const tabs = [
    { value: 'agents', label: tAgent('tabs.agents'), icon: <Cpu className="h-4 w-4" /> },
    { value: 'memory', label: tAgent('tabs.memory'), icon: <Brain className="h-4 w-4" /> },
    { value: 'skills', label: tAgent('tabs.skills'), icon: <BookOpen className="h-4 w-4" /> },
  ]

  const tabActions = activeTab === 'agents' && agents.length > 0
    ? [{ label: tAgent('createAgent'), icon: <Plus className="h-4 w-4" />, onClick: handleCreate }]
    : activeTab === 'memory'
    ? [
        { label: tAgent('systemMemory.config.title', 'Config'), icon: <Settings className="h-4 w-4" />, onClick: () => memoryPanelRef.current?.openConfig() },
        { label: tAgent('systemMemory.extract', 'Extract'), icon: isExtracting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Sparkles className="h-4 w-4" />, onClick: handleTriggerExtract, loading: isExtracting, disabled: isExtracting },
        { label: tCommon('refresh'), icon: <RefreshCw className="h-4 w-4" />, onClick: () => setMemoryRefreshKey(k => k + 1), disabled: isExtracting },
      ]
    : activeTab === 'skills'
    ? [
        { label: tAgent('skills.createSkill', 'Add Skill'), icon: <Plus className="h-4 w-4" />, onClick: () => skillsPanelRef.current?.openCreate() },
      ]
    : []

  const skillsActionsExtra = undefined

  return (
    <PageLayout
      title={tAgent('title')}
      subtitle={tAgent('description')}
      headerContent={
        <PageTabsBar
          tabs={tabs}
          activeTab={activeTab}
          onTabChange={handleTabChange}
          actions={tabActions}
          actionsExtra={skillsActionsExtra}
        />
      }
      hideFooterOnMobile
    >
      <PageTabsContent value="agents" activeTab={activeTab}>
        {loading ? (
          <div className="h-full flex items-center justify-center">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : agents.length === 0 ? (
          <div className="flex min-h-[500px] items-center justify-center">
            <EmptyState
              icon={<Bot className="h-12 w-12" />}
              title={tAgent('noAgents')}
              description={tAgent('noAgentsDesc')}
              action={{
                label: tAgent('createAgent'),
                onClick: handleCreate,
              }}
            />
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {agentsWithExecutingStatus.map((agent) => (
              <AgentCard
                key={agent.id}
                agent={agent}
                onToggleStatus={handleToggleStatus}
                onExecute={handleExecute}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onClick={() => handleViewDetail(agent)}
              />
            ))}
          </div>
        )}
      </PageTabsContent>

      <PageTabsContent value="memory" activeTab={activeTab}>
        <MemoryPanel ref={memoryPanelRef} refreshKey={memoryRefreshKey} />
      </PageTabsContent>

      <PageTabsContent value="skills" activeTab={activeTab}>
        <SkillsPanel ref={skillsPanelRef} />
      </PageTabsContent>

      {/* Mobile: Bottom navigation bar */}
      <PageTabsBottomNav
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={handleTabChange}
      />

      {/* Agent Editor Full Screen */}
      <AgentEditorFullScreen
        open={showAgentDialog}
        onOpenChange={setShowAgentDialog}
        agent={editingAgent}
        devices={devices}
        deviceTypes={deviceTypes}
        extensions={extensions}
        extensionDataSources={extensionDataSources}
        onSave={handleSave}
      />

      {/* Agent Detail Dialog */}
      <Dialog open={detailDialogOpen} onOpenChange={setDetailDialogOpen}>
        <DialogContent className="sm:max-w-5xl sm:max-h-[85vh] flex flex-col overflow-hidden">
          <DialogHeader>
            <DialogTitle>{tAgent('detailTitle')}</DialogTitle>
          </DialogHeader>
          <div className="flex-1 overflow-y-auto">
            {selectedAgent && (
              <AgentDetailPanel
                agent={selectedAgent}
                onEdit={handleEdit}
                onExecute={handleExecute}
                onViewExecutionDetail={handleViewExecutionDetail}
                onRefresh={loadItems}
                inlineMode
              />
            )}
          </div>
        </DialogContent>
      </Dialog>

      {/* Execution Detail Dialog */}
      <ExecutionDetailDialog
        open={executionDetailOpen}
        onOpenChange={setExecutionDetailOpen}
        agentId={detailAgentId}
        executionId={detailExecutionId}
      />
    </PageLayout>
  )
}

// Export as default for the route
export default AgentsPage
