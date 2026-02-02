/**
 * NeoMind AI Agents Page
 *
 * User-defined AI Agents for autonomous IoT automation.
 * Card grid layout with detail dialog for viewing individual agent details.
 * Uses WebSocket events for real-time agent status updates.
 */

import { useState, useCallback, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { PageLayout } from "@/components/layout/PageLayout"
import { api } from "@/lib/api"
import { useToast } from "@/hooks/use-toast"
import { confirm } from "@/hooks/use-confirm"
import { useEvents } from "@/hooks/useEvents"
import { Loader2, Bot } from "lucide-react"
import { EmptyState } from "@/components/shared/EmptyState"
import type { AiAgent, AiAgentDetail } from "@/types"
import type { AgentExecutionStartedEvent, AgentExecutionCompletedEvent, AgentThinkingEvent } from "@/lib/events"

// Import components
import { AgentCard, CreateCard } from "./agents-components/AgentCard"
import { AgentEditorFullScreen } from "./agents-components/AgentEditorFullScreen"
import { ExecutionDetailDialog } from "./agents-components/ExecutionDetailDialog"
import { AgentDetailPanel } from "./agents-components/AgentDetailPanel"
import {
  Sheet,
  SheetContent,
} from "@/components/ui/sheet"

export function AgentsPage() {
  const { t: tCommon } = useTranslation('common')
  const { t: tAgent } = useTranslation('agents')
  const { toast } = useToast()

  // Dialog states
  const [showAgentDialog, setShowAgentDialog] = useState(false)
  const [executionDetailOpen, setExecutionDetailOpen] = useState(false)
  const [detailSheetOpen, setDetailSheetOpen] = useState(false)

  // Dialog data states
  const [detailAgentId, setDetailAgentId] = useState('')
  const [detailExecutionId, setDetailExecutionId] = useState('')

  // Editing states
  const [editingAgent, setEditingAgent] = useState<AiAgentDetail | undefined>(undefined)
  const [selectedAgent, setSelectedAgent] = useState<AiAgentDetail | null>(null)

  // Data state
  const [agents, setAgents] = useState<AiAgent[]>([])
  const [loading, setLoading] = useState(false)

  // Track executing agents for real-time updates with timestamps for timeout
  const [executingAgents, setExecutingAgents] = useState<Map<string, number>>(new Map())
  // Track current thinking state per agent
  const [agentThinking, setAgentThinking] = useState<Record<string, string>>({})

  // Resources for dialogs
  const [devices, setDevices] = useState<any[]>([])
  const [deviceTypes, setDeviceTypes] = useState<any[]>([])

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

      // Load agents
      const data = await api.listAgents()
      // Sort by created_at descending (newest first)
      setAgents((data.agents || []).sort((a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      ))
    } catch (error) {
      console.error('Failed to load agents:', error)
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
          setAgents(prev => prev.map(agent =>
            agent.id === completedData.agent_id
              ? { ...agent, status: completedData.success ? 'Active' : 'Error' }
              : agent
          ))

          // Update selected agent if it's the same one
          if (selectedAgent?.id === completedData.agent_id) {
            setSelectedAgent(prev => prev ? {
              ...prev,
              status: completedData.success ? 'Active' : 'Error'
            } : null)
          }

          // Reload agents to get updated stats (non-blocking)
          loadItems()
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
      console.error('Failed to load agent details:', error)
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
      console.error('Failed to delete agent:', error)
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
      console.error('Failed to toggle agent status:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleExecute = async (agent: AiAgent) => {
    try {
      const result = await api.executeAgent(agent.id)
      toast({
        title: tCommon('success'),
        description: tAgent('executionStarted', { id: result.execution_id }),
      })
      // Immediately mark as executing (WebSocket will also update this)
      setAgents(prev => prev.map(a =>
        a.id === agent.id ? { ...a, status: 'Executing' } : a
      ))
      setExecutingAgents(prev => new Map(prev).set(agent.id, Date.now()))
    } catch (error) {
      console.error('Failed to execute agent:', error)
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
      console.error('Failed to save agent:', error)
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
      setDetailSheetOpen(true)
    } catch (error) {
      console.error('Failed to load agent details:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  // Refresh detail when sheet is open
  useEffect(() => {
    if (detailSheetOpen && selectedAgent) {
      api.getAgent(selectedAgent.id).then(setSelectedAgent).catch(console.error)
    }
  }, [agents, detailSheetOpen, selectedAgent?.id])

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

  return (
    <PageLayout
      title={tAgent('title')}
      subtitle={tAgent('description')}
    >
      {loading ? (
        <div className="flex items-center justify-center py-20">
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
          <CreateCard onClick={handleCreate} />
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

      {/* Agent Editor Full Screen */}
      <AgentEditorFullScreen
        open={showAgentDialog}
        onOpenChange={setShowAgentDialog}
        agent={editingAgent}
        devices={devices}
        deviceTypes={deviceTypes}
        onSave={handleSave}
      />

      {/* Agent Detail Sheet */}
      <Sheet open={detailSheetOpen} onOpenChange={setDetailSheetOpen}>
        <SheetContent className="w-full sm:max-w-3xl p-0 gap-0">
          <div className="px-6 pt-6 pb-4 border-b flex items-center justify-between">
            <h2 className="text-lg font-semibold">{tAgent('detailTitle')}</h2>
          </div>
          <div className="h-[calc(100vh-100px)] overflow-y-auto">
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
        </SheetContent>
      </Sheet>

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
