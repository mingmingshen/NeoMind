import { useEffect, useState, useRef } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { useToast } from "@/hooks/use-toast"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { Upload, Download, Sparkles, Trash2 } from "lucide-react"
import { api } from "@/lib/api"
import type { Device, DiscoveredDevice, DeviceType } from "@/types"
import {
  DeviceList,
  DeviceDetail,
  DiscoveryDialog,
  AddDeviceDialog,
  DeviceTypeList,
  AddDeviceTypeDialog,
  ViewDeviceTypeDialog,
  EditDeviceTypeDialog,
} from "./devices/index"
import { DeviceTypeGeneratorDialog } from "@/components/devices/DeviceTypeGeneratorDialog"
import { DraftDevicesList } from "./devices/DraftDevicesList"

type DeviceTabValue = "devices" | "types" | "drafts"

export function DevicesPage() {
  const { t } = useTranslation(['common', 'devices'])
  const { toast } = useToast()
  const {
    devices,
    devicesLoading,
    fetchDevices,
    fetchDeviceDetails,
    fetchDeviceTypeDetails,
    addDevice,
    deleteDevice,
    deviceTypes,
    deviceTypesLoading,
    fetchDeviceTypes,
    addDeviceType,
    deleteDeviceType,
    validateDeviceType,
    generateMDL,
    addDeviceDialogOpen,
    setAddDeviceDialogOpen,
    sendCommand,
    deviceTypeDetails,
    deviceDetails,
    telemetryData,
    telemetryLoading,
    fetchTelemetryData,
    discoverDevices,
    discovering,
    discoveredDevices,
  } = useStore()

  // Pagination state
  const [devicePage, setDevicePage] = useState(1)
  const devicesPerPage = 10

  // Device type pagination state
  const [deviceTypePage, setDeviceTypePage] = useState(1)
  const deviceTypesPerPage = 10

  // Active tab state
  const [activeTab, setActiveTab] = useState<DeviceTabValue>("devices")

  // Reset device type pagination when data changes
  useEffect(() => {
    setDeviceTypePage(1)
  }, [deviceTypes.length])

  // Paginated device types
  const paginatedDeviceTypes = deviceTypes.slice(
    (deviceTypePage - 1) * deviceTypesPerPage,
    deviceTypePage * deviceTypesPerPage
  )

  // Reset pagination when data changes
  useEffect(() => {
    setDevicePage(1)
  }, [devices.length])

  // Paginated data
  const paginatedDevices = devices.slice(
    (devicePage - 1) * devicesPerPage,
    devicePage * devicesPerPage
  )

  // Dialog states
  const [discoveryOpen, setDiscoveryOpen] = useState(false)

  // Device detail view state
  const [deviceDetailView, setDeviceDetailView] = useState<string | null>(null)
  const [selectedMetric, setSelectedMetric] = useState<string | null>(null)

  // Fetch devices on mount (once)
  const hasFetchedDevices = useRef(false)
  useEffect(() => {
    if (!hasFetchedDevices.current) {
      hasFetchedDevices.current = true
      fetchDevices()
    }
  }, [])

  // Fetch device types on mount (once)
  const hasFetchedDeviceTypes = useRef(false)
  useEffect(() => {
    if (!hasFetchedDeviceTypes.current) {
      hasFetchedDeviceTypes.current = true
      fetchDeviceTypes()
    }
  }, [])

  // Auto-refresh device status every 10 seconds (only when not in detail view)
  useEffect(() => {
    if (deviceDetailView) return

    const interval = setInterval(() => {
      fetchDevices()
    }, 10000)

    return () => clearInterval(interval)
  }, [deviceDetailView])

  // Handlers
  const handleAddDevice = async (request: import('@/types').AddDeviceRequest) => {
    setAddingDevice(true)
    try {
      return await addDevice(request)
    } finally {
      setAddingDevice(false)
    }
  }

  const handleDeleteDevice = async (id: string) => {
    if (confirm(t('devices:deleteConfirm'))) {
      const success = await deleteDevice(id)
      if (success) {
        toast({ title: t('common:success'), description: t('devices:deviceDeleted') })
      } else {
        toast({ title: t('common:failed'), description: t('devices:deleteFailed'), variant: "destructive" })
      }
    }
  }

  const handleOpenDeviceDetails = async (device: Device) => {
    setDeviceDetailView(device.id)
    setSelectedMetric(null)
    await fetchDeviceDetails(device.id)
    await fetchDeviceTypeDetails(device.device_type)
    // Fetch all telemetry data (no specific metric = get all metrics)
    const end = Math.floor(Date.now() / 1000)
    const start = end - 86400 // 24 hours
    await fetchTelemetryData(device.id, undefined, start, end, 100)
  }

  const handleCloseDeviceDetail = () => {
    setDeviceDetailView(null)
    setSelectedMetric(null)
  }

  const handleRefreshDeviceDetail = async () => {
    if (deviceDetailView) {
      await fetchDeviceDetails(deviceDetailView)
      if (selectedMetric) {
        await fetchTelemetryData(deviceDetailView, selectedMetric, undefined, undefined, 1000)
      } else {
        // Fetch all metrics if no specific metric selected
        const end = Math.floor(Date.now() / 1000)
        const start = end - 86400
        await fetchTelemetryData(deviceDetailView, undefined, start, end, 100)
      }
    }
  }

  const handleMetricClick = async (metricName: string) => {
    if (!deviceDetailView) return
    setSelectedMetric(metricName)
    const end = Math.floor(Date.now() / 1000)
    const start = end - 86400 // 24 hours
    await fetchTelemetryData(deviceDetailView, metricName, start, end, 1000)
  }

  const handleSendCommand = async (commandName: string, paramsJson: string) => {
    if (!deviceDetailView) return

    try {
      let params: Record<string, unknown> = {}
      if (paramsJson.trim()) {
        try {
          params = JSON.parse(paramsJson)
        } catch {
          alert(t('devices:paramsError'))
          return
        }
      }
      const success = await sendCommand(deviceDetailView, commandName, params)
      if (success) {
        } else {
        alert(t('devices:sendCommandFailed'))
      }
    } catch {
      alert(t('devices:sendCommandFailed'))
    }
  }

  const handleAddDiscoveredDevice = async (device: DiscoveredDevice) => {
    if (!device.device_type) {
      toast({ title: t('common:failed'), description: t('devices:unknownType'), variant: "destructive" })
      return
    }
    // For discovered devices, use MQTT adapter with default topics
    const success = await addDevice({
      device_id: device.id,
      name: device.id,
      device_type: device.device_type,
      adapter_type: 'mqtt',
      connection_config: {
        telemetry_topic: `device/${device.device_type}/${device.id}/uplink`,
      }
    })
    if (success) {
      setDiscoveryOpen(false)
      toast({ title: t('common:success'), description: t('devices:add.successGeneric') })
    } else {
      toast({ title: t('common:failed'), description: t('devices:addDeviceFailed'), variant: "destructive" })
    }
  }

  const [addingDevice, setAddingDevice] = useState(false)

  // Device Type dialog states
  const [addDeviceTypeOpen, setAddDeviceTypeOpen] = useState(false)
  const [viewDeviceTypeOpen, setViewDeviceTypeOpen] = useState(false)
  const [editDeviceTypeOpen, setEditDeviceTypeOpen] = useState(false)
  const [generatorOpen, setGeneratorOpen] = useState(false)
  const [importingDeviceType, setImportingDeviceType] = useState(false)
  const deviceTypeImportRef = useRef<HTMLInputElement>(null)
  const [selectedDeviceType, setSelectedDeviceType] = useState<DeviceType | null>(null)
  const [editingDeviceType, setEditingDeviceType] = useState<DeviceType | null>(null)
  const [addingType, setAddingType] = useState(false)
  const [validatingType, setValidatingType] = useState(false)
  const [generatingMDL, setGeneratingMDL] = useState(false)

  // Device Type handlers
  const handleRefreshDeviceTypes = () => {
    fetchDeviceTypes()
  }

  const handleViewDeviceType = async (type: DeviceType) => {
    // Fetch full device type details with metrics and commands
    const details = await fetchDeviceTypeDetails(type.device_type)
    if (details) {
      setSelectedDeviceType(details)
      setViewDeviceTypeOpen(true)
    } else {
      toast({
        title: t('devices:loadFailed'),
        description: t('devices:failedToLoadDetails'),
        variant: "destructive",
      })
    }
  }

  const handleEditDeviceType = async (type: DeviceType) => {
    // Fetch full device type details with metrics and commands
    const details = await fetchDeviceTypeDetails(type.device_type)
    if (details) {
      setEditingDeviceType(details)
      setEditDeviceTypeOpen(true)
    } else {
      toast({
        title: t('devices:loadFailed'),
        description: t('devices:failedToLoadDetails'),
        variant: "destructive",
      })
    }
  }

  const handleDeleteDeviceType = async (id: string) => {
    if (confirm(t('devices:deleteTypeConfirm'))) {
      const success = await deleteDeviceType(id)
      if (success) {
        toast({ title: t('common:success'), description: t('devices:deviceTypeDeleted') })
      } else {
        toast({ title: t('common:failed'), description: t('devices:deviceTypeDeleteFailed'), variant: "destructive" })
      }
    }
  }

  const handleAddDeviceType = async (definition: DeviceType) => {
    setAddingType(true)
    try {
      return await addDeviceType(definition)
    } finally {
      setAddingType(false)
    }
  }

  const handleValidateDeviceType = async (definition: DeviceType) => {
    setValidatingType(true)
    try {
      return await validateDeviceType(definition)
    } finally {
      setValidatingType(false)
    }
  }

  const handleGenerateMDL = async (deviceName: string, description: string, metricsExample: string, commandsExample: string) => {
    setGeneratingMDL(true)
    try {
      // Backend API still expects uplink_example/downlink_example for backward compatibility
      const result = await generateMDL({ 
        device_name: deviceName, 
        description, 
        uplink_example: metricsExample, 
        downlink_example: commandsExample 
      })
      // Add metric_count and command_count to the result
      const fullResult = {
        ...result,
        metric_count: result.metrics?.length || 0,
        command_count: result.commands?.length || 0,
      }
      return JSON.stringify(fullResult, null, 2)
    } finally {
      setGeneratingMDL(false)
    }
  }

  const handleEditDeviceTypeSubmit = async (data: DeviceType) => {
    return await handleAddDeviceType(data)
  }

  // Device Type import/export/generator handlers
  const handleDeviceTypeImportClick = () => {
    deviceTypeImportRef.current?.click()
  }

  const handleDeviceTypeImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setImportingDeviceType(true)
    try {
      const text = await file.text()
      const imported = JSON.parse(text)
      const typesToImport = Array.isArray(imported) ? imported : [imported]

      let successCount = 0
      let errorCount = 0

      for (const type of typesToImport) {
        try {
          await addDeviceType(type)
          successCount++
        } catch (err) {
          errorCount++
          console.error(`Failed to import ${type.device_type}:`, err)
        }
      }

      if (successCount > 0) {
        toast({
          title: t('common:success'),
          description: `Imported ${successCount} device type${successCount > 1 ? 's' : ''}${errorCount > 0 ? ` (${errorCount} failed)` : ''}`
        })
        fetchDeviceTypes()
      } else {
        toast({
          title: t('common:failed'),
          description: 'No device types were imported',
          variant: 'destructive'
        })
      }
    } catch (error) {
      toast({
        title: t('common:failed'),
        description: 'Failed to parse JSON file',
        variant: 'destructive'
      })
    } finally {
      setImportingDeviceType(false)
      if (deviceTypeImportRef.current) {
        deviceTypeImportRef.current.value = ''
      }
    }
  }

  const handleDeviceTypeExportAll = async () => {
    try {
      const fullTypes = await Promise.all(
        deviceTypes.map(t => api.getDeviceType(t.device_type))
      )
      const data = JSON.stringify(fullTypes, null, 2)
      const blob = new Blob([data], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `all-device-types.json`
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      URL.revokeObjectURL(url)
      toast({ title: t('common:success'), description: `Exported ${deviceTypes.length} device types` })
    } catch (error) {
      toast({
        title: t('common:failed'),
        description: 'Failed to export device types',
        variant: 'destructive'
      })
    }
  }

  // Draft cleanup handler
  const handleDraftsCleanup = async () => {
    try {
      const result = await api.cleanupDraftDevices()
      toast({
        title: t('common:success'),
        description: t('devices:drafts.cleaned', { count: result.cleaned }),
      })
    } catch (error) {
      toast({
        title: t('common:failed'),
        description: t('devices:drafts.cleanupFailed'),
        variant: 'destructive'
      })
    }
  }

  return (
    <PageLayout
      title={t('devices:title')}
      subtitle={t('devices:subtitle')}
    >
      {deviceDetailView && deviceDetails ? (
        // Device Detail View
        <DeviceDetail
          device={deviceDetails}
          deviceType={deviceTypeDetails}
          telemetryData={telemetryData}
          telemetryLoading={telemetryLoading}
          selectedMetric={selectedMetric}
          onBack={handleCloseDeviceDetail}
          onRefresh={handleRefreshDeviceDetail}
          onMetricClick={handleMetricClick}
          onMetricBack={() => setSelectedMetric(null)}
          onSendCommand={handleSendCommand}
        />
      ) : (
        // Tabbed View
        <PageTabs
          tabs={[
            { value: 'devices', label: t('devices:deviceList') },
            { value: 'types', label: t('devices:deviceTypes') },
            { value: 'drafts', label: t('devices:drafts.tab') },
          ]}
          activeTab={activeTab}
          onTabChange={(v) => setActiveTab(v as DeviceTabValue)}
          actions={
            activeTab === 'devices'
              ? [
                  {
                    label: t('devices:addDevice'),
                    onClick: () => setAddDeviceDialogOpen(true),
                  },
                  {
                    label: t('devices:localNetworkScan'),
                    variant: 'outline',
                    onClick: () => setDiscoveryOpen(true),
                  },
                ]
              : activeTab === 'types'
              ? [
                  {
                    label: t('common:import'),
                    icon: <Upload className="h-4 w-4" />,
                    variant: 'outline',
                    onClick: handleDeviceTypeImportClick,
                    disabled: importingDeviceType,
                  },
                  {
                    label: t('common:export') + ' All',
                    icon: <Download className="h-4 w-4" />,
                    variant: 'outline',
                    onClick: handleDeviceTypeExportAll,
                    disabled: deviceTypes.length === 0,
                  },
                  {
                    label: t('devices:types.generator.button'),
                    icon: <Sparkles className="h-4 w-4" />,
                    variant: 'outline',
                    onClick: () => setGeneratorOpen(true),
                  },
                  {
                    label: t('devices:addDeviceType'),
                    onClick: () => setAddDeviceTypeOpen(true),
                  },
                ]
              : activeTab === 'drafts'
              ? [
                  {
                    label: t('devices:drafts.cleanup'),
                    icon: <Trash2 className="h-4 w-4" />,
                    variant: 'outline',
                    onClick: handleDraftsCleanup,
                    disabled: false,
                  },
                ]
              : []
          }
        >
          {/* Devices Tab */}
          <PageTabsContent value="devices" activeTab={activeTab}>
            <DeviceList
              devices={devices}
              loading={devicesLoading}
              paginatedDevices={paginatedDevices}
              devicePage={devicePage}
              devicesPerPage={devicesPerPage}
              onRefresh={fetchDevices}
              onViewDetails={handleOpenDeviceDetails}
              onDelete={handleDeleteDevice}
              onPageChange={setDevicePage}
              onAddDevice={() => setAddDeviceDialogOpen(true)}
              discoveryDialogOpen={discoveryOpen}
              onDiscoveryOpenChange={setDiscoveryOpen}
              discoveryDialog={
                <DiscoveryDialog
                  open={discoveryOpen}
                  onOpenChange={setDiscoveryOpen}
                  discovering={discovering}
                  discoveredDevices={discoveredDevices}
                  deviceTypes={deviceTypes}
                  onDiscover={discoverDevices}
                  onAddDiscovered={handleAddDiscoveredDevice}
                />
              }
              addDeviceDialog={
                <AddDeviceDialog
                  open={addDeviceDialogOpen}
                  onOpenChange={setAddDeviceDialogOpen}
                  deviceTypes={deviceTypes}
                  onAdd={handleAddDevice}
                  adding={addingDevice}
                />
              }
            />
          </PageTabsContent>

          {/* Device Types Tab */}
          <PageTabsContent value="types" activeTab={activeTab}>
            <DeviceTypeList
              deviceTypes={deviceTypes}
              loading={deviceTypesLoading}
              paginatedDeviceTypes={paginatedDeviceTypes}
              deviceTypePage={deviceTypePage}
              deviceTypesPerPage={deviceTypesPerPage}
              onRefresh={handleRefreshDeviceTypes}
              onViewDetails={handleViewDeviceType}
              onEdit={handleEditDeviceType}
              onDelete={handleDeleteDeviceType}
              onPageChange={setDeviceTypePage}
              addTypeDialog={
                <AddDeviceTypeDialog
                  open={addDeviceTypeOpen}
                  onOpenChange={setAddDeviceTypeOpen}
                  onAdd={handleAddDeviceType}
                  onValidate={handleValidateDeviceType}
                  onGenerateMDL={handleGenerateMDL}
                  adding={addingType}
                  validating={validatingType}
                  generating={generatingMDL}
                />
              }
            />
          </PageTabsContent>

          {/* Draft Devices Tab (Auto-onboarding) */}
          <PageTabsContent value="drafts" activeTab={activeTab}>
            <DraftDevicesList onRefresh={fetchDeviceTypes} />
          </PageTabsContent>
        </PageTabs>
      )}

      {/* Device Type Dialogs */}
      <ViewDeviceTypeDialog
        open={viewDeviceTypeOpen}
        onOpenChange={setViewDeviceTypeOpen}
        deviceType={selectedDeviceType}
      />

      <EditDeviceTypeDialog
        open={editDeviceTypeOpen}
        onOpenChange={setEditDeviceTypeOpen}
        deviceType={editingDeviceType}
        onEdit={handleEditDeviceTypeSubmit}
        editing={addingType}
        onGenerateMDL={handleGenerateMDL}
      />

      {/* Hidden file input for device type import */}
      <input
        ref={deviceTypeImportRef}
        type="file"
        accept=".json"
        className="hidden"
        onChange={handleDeviceTypeImport}
      />

      {/* Device Type Generator Dialog */}
      <DeviceTypeGeneratorDialog
        open={generatorOpen}
        onOpenChange={setGeneratorOpen}
        onDeviceTypeCreated={() => {
          fetchDeviceTypes()
          setGeneratorOpen(false)
        }}
      />
    </PageLayout>
  )
}
