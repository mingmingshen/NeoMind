/**
 * Visual Dashboard Page
 *
 * Main dashboard page with grid layout, drag-and-drop, and component library.
 * Supports both generic IoT components and business components.
 */

import { useEffect, useState, useCallback, useRef, useMemo, memo } from 'react'
import '@/lib/debug-scroll' // Auto-inits if DEBUG_SCROLL=true in localStorage
import { createPortal } from 'react-dom'
import { getPortalRoot } from '@/lib/portal'
import { useTranslation } from 'react-i18next'
import { useStore } from '@/store'
import { shallow } from 'zustand/shallow'
import { useErrorHandler } from '@/hooks/useErrorHandler'
import { useExtensionLifecycle } from '@/hooks/useExtensionLifecycle'
import { useCommunityComponentLifecycle } from '@/hooks/useCommunityComponentLifecycle'
import { useDashboardPrefetch } from '@/hooks/useDashboardPrefetch'
import { logError } from '@/lib/errors'
import { clearTelemetryCache } from '@/hooks/useDataSource/fetch'
import { fetchCache } from '@/lib/utils/async'
import { cn } from '@/lib/utils'
import { chartColorsHex } from '@/design-system/tokens/color'
import { textNano } from '@/design-system/tokens/typography'
import { createStableKey as createStableCacheKey } from '@/lib/stable-key'
import { useIsMobile, useTouchHover } from '@/hooks/useMobile'
import {
  LayoutDashboard,
  Plus,
  Check,
  Settings2,
  PanelsTopLeft,
  Copy,
  Trash2,
  Settings as SettingsIcon,
  ChevronRight,
  MoreVertical,
  Maximize,
  Minimize,
  // Indicator icons
  Hash,
  Circle,
  TrendingUp,
  Timer as TimerIcon,
  Hourglass,
  // Chart icons
  LineChart as LineChartIcon,
  BarChart3,
  PieChart as PieChartIcon,
  ScatterChart as ScatterChartIcon,
  Radar as RadarIcon,
  Filter,
  CandlestickChart,
  // Control icons
  ToggleLeft,
  Sliders as SliderIcon,
  RadioIcon,
  CheckSquare,
  ToggleLeft as SwitchIcon,
  Star,
  MapPin,
  Monitor,
  RotateCw,
  // Media icons
  Image as ImageIcon,
  Video as VideoIcon,
  Camera,
  Music,
  Globe,
  QrCode,
  Square as SquareIcon,
  Type,
  Code,
  Link,
  Share2,
  // Layout icons
  Layers,
  Container as ContainerIcon,
  MinusSquare,
  Grid,
  Minus,
  // Visualization icons
  Calendar as CalendarIcon,
  GitBranch,
  Network,
  Map as MapIcon,
  Zap,
  Box,
  Cloud,
  Sparkles,
  Database,
  // Agent icons
  Bot,
  ListTodo,
  Clock,
  Brain,
  // Device icons
  Workflow,
  Activity,
  SlidersHorizontal,
  HeartPulse,
  // More icons
  FileText,
  Table,
  Search,
  ChevronDown,
  LayoutGrid,
  List,
  Scroll,
  Play,
  Upload,
  Store as StoreIcon,
  Download,
  PackagePlus,
  Loader2,
} from 'lucide-react'
import { useParams, useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Checkbox } from '@/components/ui/checkbox'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@/components/ui/collapsible'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
} from '@/components/automation/dialog'
import { toast } from '@/components/ui/use-toast'

// Config system
import {
  createDataDisplayConfig,
  createProgressConfig,
  createControlConfig,
  createIndicatorConfig,
  createContentConfig,
  createChartConfig,
  ComponentConfigDialog,
  DualModeSourceField,
  UnifiedDataSourceConfig,
} from '@/components/dashboard/config'
import type { ComponentConfigSchema, ConfigSection } from '@/components/dashboard/config/ComponentConfigBuilder'
import { ValueMapEditor } from '@/components/dashboard/config/ValueMapEditor'
import { DataMappingConfig } from '@/components/dashboard/config/UIConfigSections'
import { LEDStateRulesConfig } from '@/components/dashboard/config/LEDStateRulesConfig'
import type { StateRule } from '@/components/dashboard/generic/LEDIndicator'
import type { SingleValueMappingConfig, TimeSeriesMappingConfig, CategoricalMappingConfig } from '@/lib/dataMapping'

// UI components
import { ColorPicker } from '@/components/ui/color-picker'
import { IconPicker } from '@/components/ui/icon-picker'
import { EntityIconPicker } from '@/components/ui/entity-icon-picker'

// Dashboard components
import { DashboardGrid } from '@/components/dashboard/DashboardGrid'
import { LayerEditorDialog } from '@/components/dashboard/generic/LayerEditorDialog'
import { MapEditorDialog, type MapBinding, type MapBindingType } from '@/components/dashboard/generic/MapEditorDialog'
import { CenterPickerDialog } from '@/components/dashboard/generic/CenterPickerDialog'
import type { LayerBinding, LayerBindingType } from '@/components/dashboard/generic/CustomLayer'
import { DashboardListSidebar } from '@/components/dashboard/DashboardListSidebar'
import { ShareManagerDialog } from '@/components/dashboard/ShareManagerDialog'
import { InstallComponentDialog } from '@/pages/dashboard-components/InstallComponentDialog'
import type { MarketComponentEntry } from '@/types/frontend-component'
import { MobileEditBar } from '@/components/dashboard/MobileEditBar'
import type { DashboardComponent, DataSourceOrList, DataSource, GenericComponent } from '@/types/dashboard'
import { getSourceId, normalizeDataSource } from '@/types/dashboard'
import type { Device } from '@/types'
import { COMPONENT_SIZE_CONSTRAINTS } from '@/types/dashboard'
import { dynamicRegistry, dtoToComponentMeta } from '@/components/dashboard/registry/DynamicRegistry'
import { communityRegistry } from '@/components/dashboard/registry/CommunityRegistry'
import { DeviceBindingConfig } from '@/components/dashboard/config/DeviceBindingConfig'
import { componentRegistry, groupComponentsByCategory, getCategoryInfo } from '@/components/dashboard/registry/registry'
import * as lucideReact from 'lucide-react'
import { api, fetchAPI } from '@/lib/api'
import { notifySuccess, notifyError } from '@/lib/notify'
import { confirm } from '@/hooks/use-confirm'

// Renderers extracted to Renderers.tsx
import { renderDashboardComponent, ComponentWrapper, scheduleDashboardIdleTask, builtInTypes } from './Renderers'

// Extracted sub-modules
import { getComponentLibrary, type ComponentCategory } from './componentLibraryUtils'
import { BindingDataSourceSelector } from './BindingDataSourceSelector'
import { SelectField, ImageSourceField, type SelectOption } from './ConfigFieldComponents'
import { generateConfigSchema as _generateConfigSchema } from './configSchemas'
import { ComponentLibrarySidebar } from './ComponentLibrarySidebar'




// ============================================================================
// Helper Functions
// ============================================================================

// All components show title in the display tab (unified standard)
function isTitleInDisplayComponent(_componentType?: string): boolean {
  // Unified: all components show title in Display tab
  return true
}

// ============================================================================
// Main Component
// ============================================================================

// Performance optimization: Memoize VisualDashboard to prevent unnecessary re-renders
// Only re-renders when dashboardId, editMode, or currentDashboard.id changes
const VisualDashboardMemo = memo(function VisualDashboard() {
  const { dashboardId } = useParams<{ dashboardId?: string }>()
  const navigate = useNavigate()
  const { t, i18n } = useTranslation('dashboardComponents')
  const { handleError } = useErrorHandler()

  // Dashboard state — split subscriptions to avoid cascade:
  // `devices` changes every 3s (batch polling) but rarely affects layout.
  // Keep it in a separate selector so the main block doesn't re-render on every batch.
  const {
    currentDashboard,
    currentDashboardId,
    dashboards,
    dashboardsLoading,
    editMode,
    componentLibraryOpen,
  } = useStore((s) => ({
    currentDashboard: s.currentDashboard,
    currentDashboardId: s.currentDashboardId,
    dashboards: s.dashboards,
    dashboardsLoading: s.dashboardsLoading,
    editMode: s.editMode,
    componentLibraryOpen: s.componentLibraryOpen,
  }), shallow)

  // Subscribe to devices LENGTH only — avoids re-rendering every 3s on batch polling
  // (component data comes from useDataSource, not this prop)
  const devicesLength = useStore((s) => s.devices.length)
  // Read devices directly from store when needed (not reactive)
  const devicesRef = useRef<Device[]>([])

  // Action selectors — single subscription for all actions (stable references)
  const {
    setEditMode, addComponent, updateComponent, batchUpdatePositions,
    removeComponent, duplicateComponent, createDashboard, updateDashboard,
    deleteDashboard, persistDashboard, setCurrentDashboard, setComponentLibraryOpen,
    fetchDashboards, fetchDevices, fetchDeviceTypes, fetchDevicesCurrentBatch,
    sendCommand,
  } = useStore((s) => ({
    setEditMode: s.setEditMode, addComponent: s.addComponent, updateComponent: s.updateComponent,
    batchUpdatePositions: s.batchUpdatePositions, removeComponent: s.removeComponent,
    duplicateComponent: s.duplicateComponent, createDashboard: s.createDashboard,
    updateDashboard: s.updateDashboard, deleteDashboard: s.deleteDashboard,
    persistDashboard: s.persistDashboard, setCurrentDashboard: s.setCurrentDashboard,
    setComponentLibraryOpen: s.setComponentLibraryOpen,
    fetchDashboards: s.fetchDashboards, fetchDevices: s.fetchDevices,
    fetchDeviceTypes: s.fetchDeviceTypes, fetchDevicesCurrentBatch: s.fetchDevicesCurrentBatch,
    sendCommand: s.sendCommand,
  }))

  // Marketplace store selectors — single subscription
  const {
    marketComponents, marketLoading, installed: installedComponents,
    fetchMarket, fetchInstalled, installFromMarket, uninstall: uninstallComponent,
  } = useStore((s) => ({
    marketComponents: s.marketComponents, marketLoading: s.marketLoading,
    installed: s.installed, fetchMarket: s.fetchMarket,
    fetchInstalled: s.fetchInstalled, installFromMarket: s.installFromMarket,
    uninstall: s.uninstall,
  }))

  // Extension lifecycle management for hot updates
  const { refreshVersion } = useExtensionLifecycle({
    autoSyncOnRegister: true,
    autoRemoveOnUnregister: true,
  })

  // Community component lifecycle
  useCommunityComponentLifecycle()

  // Pre-batch data loading: 1 batch request + telemetry cache warm-up
  useDashboardPrefetch(currentDashboard?.components ?? [])

  // Memoize component library with refreshVersion dependency to trigger re-renders
  const componentLibrary = useMemo(() => getComponentLibrary(t), [t, refreshVersion, installedComponents.length])

  // Component library search
  const [librarySearch, setLibrarySearch] = useState('')
  const [libraryTab, setLibraryTab] = useState<'components' | 'marketplace'>('components')
  const [importDialogOpen, setImportDialogOpen] = useState(false)
  const [installingId, setInstallingId] = useState<string | null>(null)

  // Fetch marketplace data when marketplace tab is active
  useEffect(() => {
    if (componentLibraryOpen && libraryTab === 'marketplace') {
      fetchMarket()
    }
  }, [componentLibraryOpen, libraryTab, fetchMarket])

  // Fetch installed components on mount (needed for community registry sync)
  // and when component library opens
  useEffect(() => {
    return scheduleDashboardIdleTask(() => {
      fetchInstalled()
    }, 2500)
  }, [fetchInstalled])

  const filteredLibrary = useMemo(() => {
    if (!librarySearch.trim()) return componentLibrary
    const q = librarySearch.toLowerCase()
    return componentLibrary
      .map(cat => ({
        ...cat,
        items: cat.items.filter(item =>
          item.name.toLowerCase().includes(q) ||
          item.description.toLowerCase().includes(q)
        ),
      }))
      .filter(cat => cat.items.length > 0)
  }, [componentLibrary, librarySearch])

  const [configOpen, setConfigOpen] = useState(false)
  const [selectedComponent, setSelectedComponent] = useState<DashboardComponent | null>(null)

  // Extension fullscreen dialog state
  const [extFullscreenContent, setExtFullscreenContent] = useState<React.ReactNode | null>(null)

  const openExtFullscreen = useCallback((content: React.ReactNode) => {
    setExtFullscreenContent(content)
  }, [])
  const closeExtFullscreen = useCallback(() => {
    setExtFullscreenContent(null)
  }, [])

  // Mobile editing state
  const isMobile = useIsMobile()
  const isDesktop = !isMobile
  const [mobileSelectedId, setMobileSelectedId] = useState<string | null>(null)
  const [mobileEditBarOpen, setMobileEditBarOpen] = useState(false)

  // Map editor dialog state
  const [mapEditorOpen, setMapEditorOpen] = useState(false)
  const [mapEditorBindings, setMapEditorBindings] = useState<MapBinding[]>([])

  // Center picker dialog state
  const [centerPickerOpen, setCenterPickerOpen] = useState(false)

  // Layer editor dialog state
  const [layerEditorOpen, setLayerEditorOpen] = useState(false)
  const [layerEditorBindings, setLayerEditorBindings] = useState<LayerBinding[]>([])

  // Agents for agent-monitor-widget config (summaries only)
  const [agents, setAgents] = useState<{ id: string; name: string; status: string }[]>([])
  const [agentsLoading, setAgentsLoading] = useState(false)

  // Vision models for ai-analyst config
  const [visionModels, setVisionModels] = useState<{ id: string; name: string; backendId: string; backendName: string }[]>([])
  const [visionModelsLoading, setVisionModelsLoading] = useState(false)

  // Fullscreen state
  const [isFullscreen, setIsFullscreen] = useState(false)

  // Share dialog state
  const [shareDialogOpen, setShareDialogOpen] = useState(false)

  // Persist sidebar state to localStorage (default to closed on mobile, open on desktop)
  const [sidebarOpen, setSidebarOpen] = useState(() => {
    const saved = localStorage.getItem('neomind_dashboard_sidebar_open')
    if (saved !== null) return saved !== 'false'
    // No saved value - default based on device type
    return window.innerWidth >= 1024 // Default to closed on mobile/tablet, open on desktop
  })

  // Update localStorage when sidebar state changes
  const handleSidebarOpenChange = useCallback((open: boolean) => {
    setSidebarOpen(open)
    localStorage.setItem('neomind_dashboard_sidebar_open', String(open))
  }, [])

  // Fullscreen toggle function - CSS-only fullscreen for dashboard content
  const toggleFullscreen = useCallback(() => {
    setIsFullscreen(prev => !prev)
  }, [])

  // Dashboard list handlers
  const handleDashboardSwitch = useCallback((id: string) => {
    setCurrentDashboard(id)
    // Reset mobile editing state when switching dashboards
    setMobileSelectedId(null)
    setMobileEditBarOpen(false)
    // URL will be updated automatically by the Store → URL sync effect
  }, [setCurrentDashboard])

  // Dashboard interaction handlers
  const handleDeviceClick = useCallback(async (deviceId: string) => {
    const device = devicesRef.current.find(d => d.id === deviceId || d.device_id === deviceId)
    if (device) {
      // Navigate to device detail page
      navigate(`/devices/${device.id}`)
    } else {
      toast({
        title: t('visualDashboard.deviceNotFound'),
        description: t('visualDashboard.deviceNotFoundDesc'),
        variant: 'destructive',
      })
    }
  }, [navigate, t])

  const handleMetricClick = useCallback(async (metricId: string, deviceId?: string) => {
    // Show metric info in toast
    toast({
      title: t('visualDashboard.metricInfo'),
      description: `${t('visualDashboard.metric')}: ${metricId}${deviceId ? `\n${t('visualDashboard.device')}: ${deviceId.slice(0, 8)}...` : ''}`,
    })
  }, [t])

  const handleCommandClick = useCallback(async (deviceId: string, command: string) => {
    try {
      const success = await sendCommand(deviceId, command)
      if (success) {
        toast({
          title: t('visualDashboard.commandSent'),
          description: `${t('visualDashboard.command')}: ${command}\n${t('visualDashboard.device')}: ${deviceId.slice(0, 8)}...`,
        })
      } else {
        toast({
          title: t('visualDashboard.commandFailed'),
          description: `${t('visualDashboard.command')}: ${command}`,
          variant: 'destructive',
        })
      }
    } catch (error) {
      handleError(error, t('visualDashboard.commandError'))
    }
  }, [sendCommand, t, handleError])

  const handleDashboardCreate = useCallback(async (name: string) => {
    const newId = await createDashboard({
      name,
      layout: {
        columns: 12,
        rows: 'auto' as const,
        breakpoints: { lg: 1200, md: 996, sm: 768, xs: 480 },
      },
      components: [],
    })
    // Navigate to the new dashboard
    if (newId) {
      navigate(`/visual-dashboard/${newId}`, { replace: true })
    }
  }, [createDashboard, navigate])

  const handleDashboardRename = useCallback((id: string, name: string) => {
    updateDashboard(id, { name })
  }, [updateDashboard])

  const handleDashboardDelete = useCallback(async (id: string) => {
    const confirmed = await confirm({
      title: 'Delete Dashboard',
      description: 'Delete this dashboard?',
      confirmText: 'Delete',
      cancelText: 'Cancel',
      variant: 'destructive'
    })
    if (confirmed) {
      deleteDashboard(id)
    }
  }, [deleteDashboard])

  // Config dialog state
  const [configTitle, setConfigTitle] = useState('')
  const [componentConfig, setComponentConfig] = useState<Record<string, any>>({})
  const [configSchema, setConfigSchema] = useState<ComponentConfigSchema | null>(null)

  // Store original config for revert on cancel
  const [originalComponentConfig, setOriginalComponentConfig] = useState<Record<string, any>>({})
  const [originalTitle, setOriginalTitle] = useState('')

  // Track if we've initialized to avoid duplicate calls
  const hasInitialized = useRef(false)

  // Track last synced dashboard ID to prevent URL↔Store ping-pong
  const lastSyncedIdRef = useRef<string | null>(null)

  // Track previous components to detect actual changes (not just reference changes)
  // Create a stable key for components to detect actual changes
  // This key only changes when component data actually changes, not on every render
  const componentsStableKey = useMemo(() => {
    const components = currentDashboard?.components ?? []
    // Lightweight content hash — must include dataSource/config changes
    // so the grid rebuilds when data binding or settings change.
    let hash = components.length.toString(36)
    for (const c of components) {
      const gc = c as GenericComponent
      // Hash dataSource identity (use _saveTs stamp if present for forced refresh)
      const ds = gc.dataSource
      const dsKey = ds ? (Array.isArray(ds) ? ds.map((d: any) => `${d.type}:${d.sourceId ?? d.extensionId ?? ''}:${d.metricId ?? ''}:${d._saveTs ?? ''}`).join(',') : `${(ds as any).type}:${(ds as any).sourceId ?? (ds as any).extensionId ?? ''}:${(ds as any).metricId ?? ''}:${(ds as any)._saveTs ?? ''}`) : ''
      // Hash config — use value hash to detect actual changes (not just key changes)
      const configHash = gc.config ? JSON.stringify(gc.config).length.toString(36) + ':' + Object.keys(gc.config).sort().join(',') : ''
      hash += `|${c.id}:${c.type}:${c.title}:${c.position?.x ?? 0},${c.position?.y ?? 0},${c.position?.w ?? 0},${c.position?.h ?? 0}:${dsKey}:${configHash}`
    }
    return hash
  }, [currentDashboard])

  // Initialize dashboards on mount
  useEffect(() => {
    if (hasInitialized.current) return
    hasInitialized.current = true

    // Fetch dashboards first so the shell and saved layout can paint quickly.
    fetchDashboards()

    // Fetch devices and types immediately (not delayed) so that dashboard
    // components with data bindings can read current_values synchronously
    // on first mount instead of waiting for async fetch chains.
    fetchDevices()
    fetchDeviceTypes()
  }, [fetchDashboards, fetchDevices, fetchDeviceTypes])

  // Retry device fetching when devices are empty (backend DB may still be loading)
  // Max 10 retries (30s) to avoid polling forever when no devices exist
  useEffect(() => {
    // Only retry if we have dashboard components that need device data
    if (!currentDashboard || currentDashboard.components.length === 0) return
    if (devicesLength > 0) return
    if (dashboardsLoading) return

    let attempts = 0
    const MAX_ATTEMPTS = 10
    const interval = setInterval(() => {
      if (attempts >= MAX_ATTEMPTS) {
        clearInterval(interval)
        return
      }
      attempts++
      fetchDevices()
    }, 5000)

    return () => clearInterval(interval)
  }, [devicesLength, currentDashboard, dashboardsLoading, fetchDevices])

  // Batch fetch current values for devices used in dashboard components
  // Only considers the CURRENT dashboard (not all dashboards) to avoid
  // re-fetching when switching between dashboards with different devices.
  const dashboardDeviceIdsKey = useMemo(() => {
    if (!currentDashboard) return ''
    const deviceIds = new Set<string>()
    for (const component of currentDashboard.components) {
      const genericComponent = component as GenericComponent
      const dataSource = genericComponent.dataSource
      if (dataSource) {
        const sources = Array.isArray(dataSource) ? dataSource : [dataSource]
        for (const ds of sources) {
          const sid = getSourceId(ds)
          if (sid) deviceIds.add(sid)
        }
      }
      // Device-bound community/extension components (e.g. NE101 camera)
      const deviceBindingId = (genericComponent.config as any)?.deviceBinding?.deviceId as string | undefined
      if (deviceBindingId) deviceIds.add(deviceBindingId)
      if (genericComponent.type === 'map-display') {
        const bindings = (genericComponent.config as any)?.bindings as MapBinding[] || []
        for (const binding of bindings) {
          const ds = binding.dataSource as DataSource | undefined
          if (ds && getSourceId(ds)) {
            deviceIds.add(getSourceId(ds)!)
          }
        }
      }
    }
    return Array.from(deviceIds).sort().join(',')
  }, [currentDashboard])

  // Fetch batch current values when device set changes.
  // v0.7.0 approach: single initial fetch + slow background refresh (120s).
  // NO 2-second fast retry polling — it blocks the main thread during scroll
  // in WKWebView (Tauri), causing white screen frames.
  const batchFetchControllerRef = useRef<{ deviceIds: string[]; interval: ReturnType<typeof setInterval> | null }>({ deviceIds: [], interval: null })
  const batchAbortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    if (!dashboardDeviceIdsKey) return

    const deviceIds = dashboardDeviceIdsKey.split(',').filter(Boolean)
    if (deviceIds.length === 0) return

    // Abort any in-flight requests from previous effect run
    batchAbortRef.current?.abort()
    const abortController = new AbortController()
    batchAbortRef.current = abortController

    // Clear previous polling if device set changed
    const ctrl = batchFetchControllerRef.current
    if (ctrl.interval) {
      clearInterval(ctrl.interval)
      ctrl.interval = null
    }
    ctrl.deviceIds = deviceIds

    // Initial fetch (like v0.7.0)
    fetchDevicesCurrentBatch(deviceIds, abortController.signal)

    // Slow background refresh only (120s) — no fast retry polling
    const SLOW_REFRESH_MS = 120_000
    ctrl.interval = setInterval(() => {
      if (!abortController.signal.aborted) {
        fetchDevicesCurrentBatch(deviceIds, abortController.signal)
      }
    }, SLOW_REFRESH_MS)

    return () => {
      abortController.abort()
      if (ctrl.interval) {
        clearInterval(ctrl.interval)
        ctrl.interval = null
      }
    }
  }, [devicesLength, dashboardDeviceIdsKey, fetchDevicesCurrentBatch])

  // Fix 3: Update devicesRef only when devices actually change (not on every render)
  useEffect(() => {
    devicesRef.current = useStore.getState().devices
  }, [devicesLength])

  // Fix 2: On dashboard switch, abort in-flight batch requests and clear polling
  useEffect(() => {
    // Abort any stale batch requests
    batchAbortRef.current?.abort()
    const ctrl = batchFetchControllerRef.current
    if (ctrl.interval) {
      clearInterval(ctrl.interval)
      ctrl.interval = null
    }
  }, [currentDashboardId])

  // Re-load dashboards if array becomes empty but we have a current ID
  useEffect(() => {
    if (dashboards.length === 0 && currentDashboardId) {
      // Try to recover by fetching again
      fetchDashboards()
    }
  }, [dashboards.length, currentDashboardId, fetchDashboards])

  // ==========================================================================
  // URL ↔ Store Sync
  // ==========================================================================

  // URL → Store: When URL dashboardId changes, load that dashboard into store.
  // Uses lastSyncedIdRef to skip when this direction already handled the ID.
  useEffect(() => {
    if (dashboards.length === 0) return

    if (dashboardId) {
      const exists = dashboards.some(d => d.id === dashboardId)
      if (exists && dashboardId !== currentDashboardId && dashboardId !== lastSyncedIdRef.current) {
        lastSyncedIdRef.current = dashboardId
        setCurrentDashboard(dashboardId)
      } else if (!exists && currentDashboardId) {
        navigate(`/visual-dashboard/${currentDashboardId}`, { replace: true })
      } else if (!exists && dashboards.length > 0) {
        const defaultDashboard = dashboards.find(d => d.isDefault) || dashboards[0]
        navigate(`/visual-dashboard/${defaultDashboard.id}`, { replace: true })
      }
    } else if (currentDashboardId) {
      // No dashboardId in URL but store has one — sync to URL (initial load)
      lastSyncedIdRef.current = currentDashboardId
      navigate(`/visual-dashboard/${currentDashboardId}`, { replace: true })
    }
  }, [dashboardId, dashboards])

  // Store → URL: When store currentDashboardId changes (e.g. sidebar click),
  // update URL. Skips when the ID was already synced from the URL direction.
  useEffect(() => {
    if (dashboards.length === 0 || !currentDashboardId) return
    if (currentDashboardId === dashboardId) return
    if (currentDashboardId === lastSyncedIdRef.current) return
    lastSyncedIdRef.current = currentDashboardId
    navigate(`/visual-dashboard/${currentDashboardId}`, { replace: true })
  }, [currentDashboardId])

  // Handle edge case: no dashboards exist yet
  useEffect(() => {
    if (dashboards.length === 0 && !dashboardsLoading && !dashboardId) {
      // Stay at /visual-dashboard without an ID - let user create first dashboard
      navigate('/visual-dashboard', { replace: true })
    }
  }, [dashboards.length, dashboardsLoading, dashboardId, navigate])

  // Load agents — preload on mount, refresh when config opens for agent-monitor-widget
  useEffect(() => {
    const loadAgents = async () => {
      if (!fetchCache.shouldFetch('agents-list')) return
      fetchCache.markFetching('agents-list')
      setAgentsLoading(true)
      try {
        const data = await api.listAgentSummaries()
        setAgents(data.agents || [])
        fetchCache.markFetched('agents-list')
      } catch (error) {
        handleError(error, { operation: 'Load agents for dashboard', showToast: false })
        setAgents([])
        fetchCache.invalidate('agents-list')
      } finally {
        setAgentsLoading(false)
      }
    }
    // Preload on mount, or reload when config opens for agent-monitor-widget
    if (!configOpen || selectedComponent?.type === 'agent-monitor-widget') {
      loadAgents()
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [configOpen, selectedComponent])

  // NOTE: agents are accessed directly via the `agents` state variable in
  // generateConfigSchema closure — no need to inject them into componentConfig.

  // Load vision models when config opens for ai-analyst
  useEffect(() => {
    const loadVisionModels = async () => {
      if (configOpen && selectedComponent?.type === 'ai-analyst') {
        setVisionModelsLoading(true)
        try {
          const resp = await api.listLlmBackends()
          const backends = resp?.backends || []
          const models: { id: string; name: string; backendId: string; backendName: string }[] = []
          for (const backend of backends) {
            if (backend.capabilities?.supports_multimodal) {
              models.push({
                id: backend.id,
                name: backend.name || backend.model,
                backendId: backend.id,
                backendName: backend.name || backend.id,
              })
            }
          }
          setVisionModels(models)
        } catch (error) {
          handleError(error, { operation: 'Load vision models for dashboard', showToast: false })
          setVisionModels([])
        } finally {
          setVisionModelsLoading(false)
        }
      }
    }
    loadVisionModels()
  }, [configOpen, selectedComponent?.type, selectedComponent?.id])

  // NOTE: visionModels are accessed directly via the `visionModels` state variable in
  // generateConfigSchema closure — no need to inject them into componentConfig.

  // Note: Removed auto-create dashboard logic
  // Users should explicitly create dashboards via the UI
  // This prevents creating duplicate dashboards on refresh

  // Handle adding a component
  const handleAddComponent = (componentType: string) => {
    const item = componentLibrary
      .flatMap(cat => cat.items)
      .find(i => i.id === componentType)

    // Get size constraints for this component type
    let constraints = COMPONENT_SIZE_CONSTRAINTS[componentType as keyof typeof COMPONENT_SIZE_CONSTRAINTS]

    // For extension components, get constraints from DTO
    if (!constraints) {
      const extensionDto = dynamicRegistry.getMeta(componentType)
      if (extensionDto?.size_constraints) {
        constraints = extensionDto.size_constraints as any
      }
    }

    // Build appropriate default config based on component type
    let defaultConfig: any = {}

    // For extension components, use their default config from DTO
    const extensionDto = dynamicRegistry.getMeta(componentType)
    if (extensionDto?.default_config) {
      defaultConfig = { ...extensionDto.default_config }
    }

    // For extension components, set up dataSource with extensionId
    let dataSource: any = undefined
    if (extensionDto?.extension_id) {
      dataSource = {
        type: 'extension' as const,
        extensionId: extensionDto.extension_id,
      }
      // Also add extensionMetric if available from data_binding
      if (extensionDto.data_binding?.extension_metric) {
        dataSource.extensionMetric = extensionDto.data_binding.extension_metric
      }
    }

    switch (componentType) {
      // Charts
      case 'line-chart':
      case 'area-chart':
        defaultConfig = {
          series: [{ name: 'Value', data: [10, 25, 15, 30, 28, 35, 20], color: chartColorsHex[0] }],
          labels: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
        }
        break
      case 'bar-chart':
        defaultConfig = {
          data: [{ name: 'A', value: 30 }, { name: 'B', value: 50 }, { name: 'C', value: 20 }]
        }
        break
      case 'pie-chart':
        defaultConfig = {
          data: [{ name: 'A', value: 30 }, { name: 'B', value: 50 }, { name: 'C', value: 20 }]
        }
        break
      // Indicators
      case 'sparkline':
        defaultConfig = {
          data: [12, 19, 15, 25, 22, 30, 28]
        }
        break
      case 'progress-bar':
        defaultConfig = {
          value: 65,
          min: 0,
          max: 100
        }
        break
      case 'led-indicator':
        defaultConfig = {
          rules: []
        }
        break
      // Controls
      case 'toggle-switch':
        defaultConfig = {
          size: 'md'
        }
        break
      // Display & Content
      case 'image-display':
        defaultConfig = {
          src: '',
          alt: 'Sample Image',
          fit: 'contain',
          rounded: true,
          zoomable: true,
        }
        break
      case 'image-history':
        defaultConfig = {
          dataSource: undefined,
          fit: 'fill',
          rounded: true,
          limit: 50,
          timeRange: 24,
        }
        break
      case 'web-display':
        defaultConfig = {
          src: 'https://example.com',
          title: 'Website',
          sandbox: true,
          showHeader: true,
        }
        break
      case 'markdown-display':
        defaultConfig = {
          content: '# Title\n\nThis is **markdown** content.\n\n- Item 1\n- Item 2\n\n`code example`',
          variant: 'default',
        }
        break
      case 'video-display':
        defaultConfig = {
          src: '',
          type: 'file',
          autoplay: false,
          muted: true,
          controls: true,
          loop: false,
          fit: 'contain',
          rounded: true,
          showFullscreen: true,
        }
        break
      case 'map-display':
        defaultConfig = {
          center: { lat: 39.9042, lng: 116.4074 }, // Beijing
          zoom: 10,
          minZoom: 2,
          maxZoom: 18,
          showControls: true,
          showLayers: true,
          showFullscreen: true,
          interactive: true,
          tileLayer: 'osm',
          markers: [
            { id: '1', latitude: 39.9042, longitude: 116.4074, label: 'Beijing', status: 'online' },
            { id: '2', latitude: 31.2304, longitude: 121.4737, label: 'Shanghai', status: 'online' },
            { id: '3', latitude: 23.1291, longitude: 113.2644, label: 'Guangzhou', status: 'warning' },
          ],
        }
        break
      case 'custom-layer':
        defaultConfig = {
          backgroundType: 'grid',
          gridSize: 20,
          showControls: true,
          showFullscreen: true,
          interactive: true,
          editable: false,
        }
        break
      // Business Components
      case 'agent-monitor-widget':
      case 'ai-analyst':
        defaultConfig = {}
        break
      default:
        // For extension components, keep the default_config from DTO (already set above)
        if (Object.keys(defaultConfig).length === 0) {
          defaultConfig = {}
        }
    }

    const w = constraints?.defaultW ?? 4
    const h = constraints?.defaultH ?? 3

    // Calculate y position based on existing components.
    // We use the bottom edge of the lowest existing component to place
    // the new component below everything currently visible.
    const existingComponents = currentDashboard?.components ?? []
    let maxY = 0
    for (const c of existingComponents) {
      const bottom = (c.position?.y ?? 0) + (c.position?.h ?? 1)
      if (bottom > maxY) maxY = bottom
    }

    const newComponent: Omit<DashboardComponent, 'id'> = {
      type: componentType as any,
      position: {
        x: 0,
        y: maxY,
        w,
        h,
        minW: constraints?.minW,
        minH: constraints?.minH,
        maxW: constraints?.maxW,
        maxH: constraints?.maxH,
      },
      title: item?.name || componentType,
      config: defaultConfig,
      ...(dataSource && { dataSource }),
    }

    addComponent(newComponent)
    setComponentLibraryOpen(false)
  }

  // Handle layout change - batch update all changed positions in a single store update
  // to prevent infinite re-render loops (store update → grid recalc → onLayoutChange → ...)
  const handleLayoutChange = useCallback((layout: readonly any[]) => {
    const components = currentDashboard?.components ?? []
    const changed: Array<{ id: string; position: { x: number; y: number; w: number; h: number } }> = []

    for (const item of layout) {
      const existing = components.find(c => c.id === item.i)
      if (!existing) continue
      if (
        existing.position.x !== item.x ||
        existing.position.y !== item.y ||
        existing.position.w !== item.w ||
        existing.position.h !== item.h
      ) {
        changed.push({
          id: item.i,
          position: { x: item.x, y: item.y, w: item.w, h: item.h },
        })
      }
    }

    if (changed.length > 0) {
      batchUpdatePositions(changed)
    }
  }, [currentDashboard?.components, batchUpdatePositions])

  // Handle opening config dialog
  const handleOpenConfig = useCallback((componentId: string) => {
    const component = currentDashboard?.components.find(c => c.id === componentId)
    if (!component) return

    setSelectedComponent(component)
    // Extract both config and dataSource (they are separate properties on GenericComponent)
    const config = { ...((component as any).config || {}) }
    const dataSource = (component as any).dataSource
    // Include title in config so style sections can access it
    const configWithTitle = { ...config, title: component.title }
    // Merge dataSource into config for unified state management
    const mergedConfig = dataSource ? { ...configWithTitle, dataSource } : configWithTitle

    // Store original config for revert on cancel
    setOriginalComponentConfig(mergedConfig)
    setOriginalTitle(component.title || '')

    setConfigTitle(component.title || '')
    setComponentConfig(mergedConfig)
    setConfigOpen(true)
  }, [currentDashboard?.components])

  // Memoize grid components to prevent infinite re-renders
  // Only recalculate when actual component data changes (detected via stableKey)
  // Note: handleOpenConfig, removeComponent, duplicateComponent are NOT dependencies
  // because they don't affect the rendered output structure, only event handlers
  // devices.length is deliberately excluded from deps — it changes every 3s due to
  // batch polling and is NOT used in this callback (component data comes via useDataSource).
  // IMPORTANT: Use currentDashboard from props (reactive) to ensure updates are reflected
  const gridComponents = useMemo(() => {
    return (currentDashboard?.components ?? []).map((component) => {
      // Get dataSource from component (it should be a separate property, not in config)
      const componentDataSource = (component as any).dataSource

      // Create callbacks for this component to persist configuration changes
      const handleDataSourceChange = (newDataSource: any) => {
        updateComponent(component.id, { dataSource: newDataSource as DataSource }, false)
      }

      const handleConfigChange = (newConfig: Record<string, any>) => {
        updateComponent(component.id, { config: newConfig }, false)
      }

      return {
        id: component.id,
        position: component.position,
        children: (
          <ComponentWrapper
            key={component.id}
            component={component}
            editMode={editMode}
            onOpenConfig={handleOpenConfig}
            onRemove={removeComponent}
            onDuplicate={duplicateComponent}
            onSelect={isMobile ? (comp) => {
              if (comp) {
                setMobileSelectedId(comp.id)
                setMobileEditBarOpen(true)
              }
            } : undefined}
            selectedComponentId={mobileSelectedId}
            isMobile={isMobile}
          >
            {renderDashboardComponent(component, editMode, handleDataSourceChange, handleConfigChange, openExtFullscreen, closeExtFullscreen)}
          </ComponentWrapper>
        ),
      }
    }) ?? []
  }, [componentsStableKey, editMode, isMobile])

  // Track initial config load to avoid unnecessary updates
  const initialConfigRef = useRef<any>(null)
  const isInitialLoad = useRef(false)
  const lastSyncedConfigRef = useRef<string>('')

  // Live preview: update component in real-time as config changes
  // Applies changes to the store immediately so the grid preview updates.
  // Schema is regenerated so the dialog's own inputs stay responsive.
  useEffect(() => {
    if (configOpen && selectedComponent) {
      // Skip initial load - don't update store with same config
      if (!isInitialLoad.current) {
        initialConfigRef.current = componentConfig
        isInitialLoad.current = true
        lastSyncedConfigRef.current = JSON.stringify(componentConfig)
        setConfigSchema(generateConfigSchema(selectedComponent.type, componentConfig))
        return
      }

      // Check if config actually changed since last sync
      const currentJSON = JSON.stringify(componentConfig)
      if (currentJSON !== lastSyncedConfigRef.current) {
        // Regenerate schema immediately so input values update
        setConfigSchema(generateConfigSchema(selectedComponent.type, componentConfig))

        // Update last synced config
        lastSyncedConfigRef.current = currentJSON

        // Apply to store immediately for live preview in the grid
        const { dataSource, ...configOnly } = componentConfig
        const currentDS = (selectedComponent as any).dataSource
        const updateData: any = { config: configOnly }
        if (dataSource !== undefined || currentDS !== undefined) {
          updateData.dataSource = dataSource
        }
        updateComponent(selectedComponent.id, updateData, false)
      }
    } else {
      // Reset when dialog closes
      isInitialLoad.current = false
      initialConfigRef.current = null
      lastSyncedConfigRef.current = ''
    }
  }, [componentConfig, configOpen, selectedComponent?.id, selectedComponent?.type, updateComponent, setConfigSchema])

  // Handle canceling component config - revert to original
  const handleCancelConfig = useCallback(() => {
    if (selectedComponent && originalComponentConfig) {
      // Revert to original config (no need to persist - reverting to saved state)
      const { dataSource, ...configOnly } = originalComponentConfig
      const currentDS = (selectedComponent as any).dataSource
      const updateData: any = { config: configOnly }
      // Include dataSource if:
      // 1. Original config had dataSource, OR
      // 2. Original config didn't have dataSource but current component does (need to clear it)
      if (dataSource !== undefined || currentDS !== undefined) {
        updateData.dataSource = dataSource
      }
      updateComponent(selectedComponent.id, updateData, false)

      // Revert title
      if (originalTitle !== selectedComponent.title) {
        updateComponent(selectedComponent.id, { title: originalTitle }, false)
      }
    }
    setConfigOpen(false)
  }, [selectedComponent, originalComponentConfig, originalTitle, updateComponent])

  // Handle saving component config - persist the dashboard to localStorage
  const handleSaveConfig = async () => {
    if (selectedComponent) {
      // Get the latest component from the store to merge with local changes
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      // Extract dataSource — only from authoritative locations:
      // 1. componentConfig.dataSource (newly selected/changed in config dialog)
      // 2. latestComponent.dataSource (existing on component as separate property)
      // Do NOT read from nested config.dataSource — the migration moved it to top-level,
      // and reading the nested one can restore a dataSource the user intentionally cleared.
      const configDataSource = componentConfig.dataSource
      const latestComponentDataSource = (latestComponent as any)?.dataSource

      // Use explicit null check: if user cleared dataSource (set to null/undefined), respect that.
      // Only fall back to the latest component dataSource if config didn't touch it at all.
      const finalDataSource = configDataSource !== undefined
        ? configDataSource
        : latestComponentDataSource

      // Merge local config changes with the latest component config
      // Local changes take precedence
      const mergedConfig = {
        ...(latestComponent as any)?.config || {},
        ...componentConfig,
      }

      // IMPORTANT: Remove dataSource from mergedConfig to avoid confusion
      // dataSource should be stored as a separate property, not inside config
      delete (mergedConfig as any).dataSource

      // Update the component in the store
      // CRITICAL: dataSource must be saved as a separate property, not inside config
      const updateData: any = {
        config: mergedConfig,
        title: configTitle,
      }
      if (finalDataSource !== undefined) {
        updateData.dataSource = finalDataSource
      }

      // 1. Save clean data (without _saveTs) to the store for persistence
      updateComponent(selectedComponent.id, updateData, false)

      // 2. Persist to storage — clean dataSource is saved
      await persistDashboard()

      // 3. Force telemetry cache refresh so dashboard components re-fetch with new settings
      clearTelemetryCache()

      // 4. Stamp dataSource with a unique timestamp to force re-render.
      //    This is done AFTER persist so _saveTs is not stored to backend.
      //    The stamp triggers: componentsStableKey change → gridComponents rebuild → useDataSource re-fetch.
      if (finalDataSource !== undefined) {
        const saveTs = Date.now()
        const stampedDataSource = Array.isArray(finalDataSource)
          ? finalDataSource.map((ds: any) => ({ ...ds, _saveTs: saveTs }))
          : { ...(finalDataSource as any), _saveTs: saveTs }
        updateComponent(selectedComponent.id, { dataSource: stampedDataSource }, false)
      }
    }
    setConfigOpen(false)
  }

  // Handle saving map editor bindings
  const handleMapEditorSave = async (bindings: MapBinding[]) => {
    // Fix any duplicate IDs in bindings before saving
    const idCount = new Map<string, number>() as Map<string, number>
    const fixedBindings = bindings.map((binding, index) => {
      const ds = binding.dataSource as any
      const currentId = binding.id
      idCount.set(currentId, (idCount.get(currentId) || 0) + 1)

      // If ID is duplicated, regenerate it
      if (idCount.get(currentId)! > 1) {
        let newId: string
        if (binding.type === 'metric' || ds?.type === 'telemetry') {
          newId = `metric-${getSourceId(ds)}-${ds?.metricId || ds?.property || index}`
        } else if (binding.type === 'command') {
          newId = `command-${getSourceId(ds)}-${ds?.command}`
        } else {
          newId = `device-${getSourceId(ds)}-${index}`
        }
        return { ...binding, id: newId }
      }
      return binding
    })

    if (selectedComponent) {
      // CRITICAL FIX: Get the latest component config from the store to avoid stale state
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      const latestConfig = (latestComponent as any)?.config || {}
      const latestDataSource = (latestComponent as any)?.dataSource

      // Merge the latest config with the new bindings, preserving dataSource
      const newConfig = { ...latestConfig, bindings: fixedBindings }
      const updateData: any = { config: newConfig }

      // CRITICAL: Preserve dataSource when updating
      if (latestDataSource) {
        updateData.dataSource = latestDataSource
      }

      // Update the store with both config and dataSource
      updateComponent(selectedComponent.id, updateData, false)

      // Update local config state
      setComponentConfig(prev => ({ ...prev, bindings: fixedBindings }))
    }

    // Persist to localStorage
    await persistDashboard()

    setMapEditorOpen(false)
  }

  // Handle saving layer editor bindings
  const handleLayerEditorSave = async (bindings: LayerBinding[]) => {
    if (selectedComponent) {
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      const latestConfig = (latestComponent as any)?.config || {}
      const latestDataSource = (latestComponent as any)?.dataSource

      // Merge the latest config with the new bindings, preserving dataSource
      const newConfig = { ...latestConfig, bindings }
      const updateData: any = { config: newConfig }

      // Preserve dataSource when updating
      if (latestDataSource) {
        updateData.dataSource = latestDataSource
      }

      // Update the store
      updateComponent(selectedComponent.id, updateData, false)

      // Force re-render

      // Update local config state
      setComponentConfig(prev => ({ ...prev, bindings }))
    }

    // Persist to localStorage
    await persistDashboard()

    setLayerEditorOpen(false)
  }

  // Handle saving center picker
  const handleCenterPickerSave = async (newCenter: { lat: number; lng: number }) => {
    if (selectedComponent) {
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      const latestConfig = (latestComponent as any)?.config || {}
      const latestDataSource = (latestComponent as any)?.dataSource

      // Merge the latest config with the new center, preserving dataSource
      const newConfig = { ...latestConfig, center: newCenter }
      const updateData: any = { config: newConfig }

      // Preserve dataSource when updating
      if (latestDataSource) {
        updateData.dataSource = latestDataSource
      }

      // Update the store
      updateComponent(selectedComponent.id, updateData, false)

      // Force re-render

      // Update local config state
      setComponentConfig(prev => ({ ...prev, center: newCenter }))
    }

    // Persist to localStorage
    await persistDashboard()

    setCenterPickerOpen(false)
  }

  // Handle title change (local state only — store updated via debounced live preview)
  const handleTitleChange = (newTitle: string) => {
    setConfigTitle(newTitle)
  }

  // Generate config schema based on component type
  const generateConfigSchema = (componentType: string, currentConfig: any): ComponentConfigSchema | null => {
    return _generateConfigSchema(componentType, currentConfig, {
      setConfigTitle,
      selectedComponent,
      updateComponent,
      setComponentConfig,
      t,
      agents,
      currentDashboard,
      setCenterPickerOpen,
      setMapEditorBindings,
      setMapEditorOpen,
      setLayerEditorBindings,
      setLayerEditorOpen,
      agentsLoading,
      visionModels,
      visionModelsLoading,
    })
  }

  if (!currentDashboard) {
    // Show loading state only if we're still loading
    if (dashboardsLoading) {
      return (
        <div className="flex items-center justify-center h-screen">
          <div className="text-center">
            <h2 className="text-lg font-medium mb-2">{t('visualDashboard.loadingDashboard')}</h2>
          </div>
        </div>
      )
    }

    // No dashboard found - show empty state with create button
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center space-y-4 px-4">
          <LayoutDashboard className="h-16 w-16 mx-auto text-muted-foreground" />
          <div>
            <h2 className="text-lg font-medium mb-1">{t('visualDashboard.noDashboardFound')}</h2>
            <p className="text-sm text-muted-foreground mb-4">
              {t('visualDashboard.createFirstDashboard')}
            </p>
            <Button
              onClick={() => {
                handleDashboardCreate('Overview').catch((err) => {
                  console.error('[VisualDashboard] Failed to create dashboard:', err)
                })
              }}
            >
              <Plus className="h-4 w-4 mr-1" />
              {t('visualDashboard.createDashboard')}
            </Button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full overflow-hidden bg-background">
      {/* Sidebar - Dashboard List - hidden in fullscreen */}
      {!isFullscreen && (
        <DashboardListSidebar
          dashboards={dashboards}
          currentDashboardId={currentDashboardId}
          onSwitch={handleDashboardSwitch}
          onCreate={handleDashboardCreate}
          onRename={handleDashboardRename}
          onDelete={handleDashboardDelete}
          open={sidebarOpen}
          onOpenChange={handleSidebarOpenChange}
          isDesktop={isDesktop}
        />
      )}

      {/* Main Content */}
      {isFullscreen && createPortal(
        <div className="fixed inset-0 z-[100] bg-background flex flex-col">
          <div className="flex-1 overflow-auto p-4 relative">
            <Button
              variant="outline"
              size="icon"
              onClick={toggleFullscreen}
              className="absolute top-4 right-4 z-50 shadow-lg bg-bg-90 backdrop-blur"
              title={t('visualDashboard.exitFullscreen')}
            >
              <Minimize className="h-4 w-4" />
            </Button>
            <DashboardGrid
              components={gridComponents}
              editMode={false}
              onLayoutChange={() => {}}
            />
          </div>
        </div>,
        getPortalRoot()
      )}
      <div className={cn(
        "flex-1 flex flex-col overflow-hidden",
        isFullscreen && "hidden"
      )}>
          <header className="shrink-0 flex items-center justify-between px-4 py-3 border-b border-border bg-background z-10">
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => isMobile ? handleSidebarOpenChange(true) : handleSidebarOpenChange(!sidebarOpen)}
                className={cn(
                  "h-6 w-6 active:scale-95",
                  !isDesktop && sidebarOpen && "bg-muted"
                )}
              >
                <PanelsTopLeft className="h-4 w-4" />
              </Button>
              <h1 className="text-sm font-semibold">
                {currentDashboard.name}
              </h1>
            </div>

            <div className="flex items-center gap-1.5">
              <Button
                variant={editMode ? "default" : "outline"}
                size="sm"
                onClick={() => {
                  const nextMode = !editMode
                  setEditMode(nextMode)
                  if (!nextMode && isMobile) {
                    setMobileSelectedId(null)
                    setMobileEditBarOpen(false)
                  }
                }}
                className={cn("h-7 text-xs rounded-md", editMode ? "shadow-sm" : "")}
              >
                {editMode ? (
                  <>
                    <Check className="h-4 w-4 mr-1" />
                    <span className="hidden sm:inline">{t('common.done')}</span>
                    <span className="sm:hidden">{t('common.done')}</span>
                  </>
                ) : (
                  <>
                    <Settings2 className="h-4 w-4 mr-1" />
                    <span className="hidden sm:inline">{t('common:editDashboard')}</span>
                    <span className="sm:hidden">{t('common:edit', 'Edit')}</span>
                  </>
                )}
              </Button>

              <Button
                variant="default"
                size="sm"
                className="h-7 text-xs rounded-md shadow-sm"
                disabled={!editMode}
                onClick={() => editMode && setComponentLibraryOpen(true)}
              >
                <Plus className="h-4 w-4 mr-1" />
                <span className="hidden sm:inline">{t('visualDashboard.addComponent')}</span>
                <span className="sm:hidden">{t('visualDashboard.addComponent')}</span>
              </Button>

              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs rounded-md"
                onClick={() => setShareDialogOpen(true)}
              >
                <Share2 className="h-4 w-4 mr-1" />
                <span className="hidden sm:inline">{t('visualDashboard.share.title')}</span>
              </Button>

              <ComponentLibrarySidebar
                open={componentLibraryOpen}
                onOpenChange={setComponentLibraryOpen}
                libraryTab={libraryTab}
                onLibraryTabChange={setLibraryTab}
                librarySearch={librarySearch}
                onLibrarySearchChange={setLibrarySearch}
                filteredLibrary={filteredLibrary}
                onAddComponent={handleAddComponent}
                marketComponents={marketComponents}
                marketLoading={marketLoading}
                installedComponents={installedComponents}
                installingId={installingId}
                onInstall={installFromMarket}
                onUninstall={uninstallComponent}
                onSetInstalling={setInstallingId}
                importDialogOpen={importDialogOpen}
                onImportDialogOpenChange={setImportDialogOpen}
              />

              {/* Fullscreen toggle button */}
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6"
                onClick={toggleFullscreen}
                title={t('visualDashboard.fullscreen')}
              >
                <Maximize className="h-4 w-4" />
              </Button>
            </div>
          </header>

        {/* Dashboard Grid */}
        <div className={cn("flex-1 overflow-auto p-4 relative")}>

          {(currentDashboard.components?.length ?? 0) === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
              <LayoutDashboard className="h-16 w-16 mb-4 opacity-50" />
              <p className="text-lg font-medium">{t('visualDashboard.emptyDashboard')}</p>
              <p className="text-sm mt-2">
                {editMode ? t('visualDashboard.addComponentsHint') : t('visualDashboard.enterEditModeHint')}
              </p>
              {editMode && (
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4"
                  onClick={() => setComponentLibraryOpen(true)}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  {t('visualDashboard.addComponent')}
                </Button>
              )}
            </div>
          ) : (
            <DashboardGrid
              components={gridComponents}
              editMode={editMode}
              onLayoutChange={handleLayoutChange}
            />
          )}
        </div>
      </div>

      {/* Config Dialog */}
      <ComponentConfigDialog
        open={configOpen}
        onClose={handleCancelConfig}
        onSave={handleSaveConfig}
        title={configTitle}
        onTitleChange={handleTitleChange}
        configSchema={configSchema}
        componentType={selectedComponent?.type || ''}
        previewDataSource={componentConfig.dataSource}
        previewConfig={componentConfig}
        showTitleInDisplay={isTitleInDisplayComponent(selectedComponent?.type)}
        position={selectedComponent?.position}
      />

      {/* Map Editor Dialog */}
      <MapEditorDialog
        open={mapEditorOpen}
        onOpenChange={setMapEditorOpen}
        bindings={mapEditorBindings}
        center={(componentConfig.center as { lat: number; lng: number }) || { lat: 39.9042, lng: 116.4074 }}
        zoom={componentConfig.zoom as number || 10}
        tileLayer={componentConfig.tileLayer as string || 'osm'}
        onSave={handleMapEditorSave}
      />

      {/* Center Picker Dialog */}
      <CenterPickerDialog
        open={centerPickerOpen}
        onOpenChange={setCenterPickerOpen}
        center={(componentConfig.center as { lat: number; lng: number }) || { lat: 39.9042, lng: 116.4074 }}
        zoom={componentConfig.zoom as number || 10}
        tileLayer={componentConfig.tileLayer as string || 'osm'}
        onSave={handleCenterPickerSave}
      />

      {/* Layer Editor Dialog */}
      <LayerEditorDialog
        open={layerEditorOpen}
        onOpenChange={setLayerEditorOpen}
        bindings={layerEditorBindings}
        backgroundType={componentConfig.backgroundType as 'color' | 'image' | 'transparent' | 'grid' || 'grid'}
        backgroundColor={componentConfig.backgroundColor as string}
        backgroundImage={componentConfig.backgroundImage as string}
        onSave={handleLayerEditorSave}
      />

      {/* Mobile Edit Bar */}
      {isMobile && mobileEditBarOpen && mobileSelectedId && (
        <MobileEditBar
          isOpen={mobileEditBarOpen}
          onClose={() => {
            setMobileEditBarOpen(false)
            setMobileSelectedId(null)
          }}
          onSettings={() => {
            const comp = currentDashboard?.components.find(c => c.id === mobileSelectedId)
            if (comp) {
              handleOpenConfig(comp.id)
              setMobileEditBarOpen(false)
            }
          }}
          onCopy={() => {
            if (mobileSelectedId) {
              duplicateComponent(mobileSelectedId)
              setMobileEditBarOpen(false)
              setMobileSelectedId(null)
            }
          }}
          onDelete={() => {
            if (mobileSelectedId) {
              removeComponent(mobileSelectedId)
              setMobileEditBarOpen(false)
              setMobileSelectedId(null)
            }
          }}
          componentName={currentDashboard?.components.find(c => c.id === mobileSelectedId)?.title}
        />
      )}

      {/* Share Management Dialog */}
      <ShareManagerDialog
        open={shareDialogOpen}
        onOpenChange={setShareDialogOpen}
        dashboardId={currentDashboardId}
        dashboardName={currentDashboard?.name}
      />

      {/* Extension Fullscreen Dialog */}
      <FullScreenDialog
        open={!!extFullscreenContent}
        onOpenChange={(open) => { if (!open) closeExtFullscreen() }}
      >
        <FullScreenDialogHeader
          icon={<Monitor className="w-5 h-5" />}
          title="Edit Content"
          onClose={closeExtFullscreen}
        />
        <FullScreenDialogContent>
          {extFullscreenContent}
        </FullScreenDialogContent>
      </FullScreenDialog>
    </div>
  )
})

// Export the memoized component
export { VisualDashboardMemo as VisualDashboard }
