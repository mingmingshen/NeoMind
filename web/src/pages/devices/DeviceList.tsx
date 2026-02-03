import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import { ResponsiveTable, StatusBadge } from "@/components/shared"
import { Eye, MoreVertical, Trash2, Cpu, Database, Waves } from "lucide-react"
import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils/format"
import type { Device } from "@/types"
import { TransformsBadge } from "@/components/automation"
import { useDeviceEvents } from "@/hooks/useEvents"
import { useStore } from "@/store"

interface DeviceListProps {
  devices: Device[]
  loading: boolean
  paginatedDevices: Device[]
  devicePage: number
  devicesPerPage: number
  onRefresh: () => void
  onViewDetails: (device: Device) => void
  onDelete: (id: string) => void
  onPageChange: (page: number) => void
  onAddDevice: () => void
  discoveryDialogOpen: boolean
  onDiscoveryOpenChange: (open: boolean) => void
  discoveryDialog: React.ReactNode
  addDeviceDialog: React.ReactNode
}

export function DeviceList({
  devices,
  loading,
  paginatedDevices,
  devicePage,
  devicesPerPage,
  onRefresh,
  onViewDetails,
  onDelete,
  onPageChange,
  onAddDevice: _onAddDevice,
  discoveryDialogOpen: _discoveryDialogOpen,
  onDiscoveryOpenChange: _onDiscoveryOpenChange,
  discoveryDialog,
  addDeviceDialog,
}: DeviceListProps) {
  const { t } = useTranslation(['common', 'devices'])
  const updateDeviceStatus = useStore((state) => state.updateDeviceStatus)

  // Listen to device status change events
  useDeviceEvents({
    enabled: true,
    eventTypes: ['DeviceOnline', 'DeviceOffline'],
    onEvent: (event) => {
      if (event.type === 'DeviceOnline' || event.type === 'DeviceOffline') {
        const data = event.data as { device_id: string }
        if (data.device_id) {
          updateDeviceStatus(data.device_id, event.type === 'DeviceOnline' ? 'online' : 'offline')
        }
      }
    },
  })

  // Get adapter icon
  const getAdapterIcon = (adapter: string) => {
    const lower = adapter?.toLowerCase() || ''
    if (lower.includes('mqtt') || lower === 'mqtt') return Database
    if (lower.includes('modbus')) return Cpu
    return Waves
  }

  return (
    <>
      {/* Dialogs (由上层 TAB 操作按钮控制 open 状态) */}
      {addDeviceDialog}
      {discoveryDialog}

      <ResponsiveTable
        columns={[
          {
            key: 'index',
            label: '#',
            width: 'w-10',
            align: 'center',
          },
          {
            key: 'name',
            label: t('devices:headers.name'),
          },
          {
            key: 'type',
            label: t('devices:headers.type'),
          },
          {
            key: 'adapter',
            label: t('devices:headers.adapter'),
          },
          {
            key: 'transforms',
            label: t('automation:transforms', { defaultValue: 'Transforms' }),
            align: 'center',
          },
          {
            key: 'status',
            label: t('devices:headers.status'),
            align: 'center',
          },
          {
            key: 'lastOnline',
            label: t('devices:headers.lastOnline'),
            align: 'center',
          },
        ]}
        data={paginatedDevices as unknown as Record<string, unknown>[]}
        rowKey={(device) => (device as unknown as Device).id}
        loading={loading}
        renderCell={(columnKey, rowData) => {
          const device = rowData as unknown as Device
          const index = paginatedDevices.indexOf(device)
          const AdapterIcon = getAdapterIcon(device.adapter_type)

          switch (columnKey) {
            case 'index':
              return (
                <span className="text-xs text-muted-foreground font-medium">
                  {index + 1}
                </span>
              )

            case 'name':
              return (
                <div className="flex items-center gap-3">
                  <div className={cn(
                    "w-9 h-9 rounded-lg flex items-center justify-center transition-colors",
                    device.status === 'online'
                      ? "bg-green-500/10 text-green-600"
                      : "bg-muted text-muted-foreground"
                  )}>
                    <Cpu className="h-4 w-4" />
                  </div>
                  <div>
                    <div className="font-medium text-sm">{device.name || "-"}</div>
                    <code className="text-xs text-muted-foreground font-mono">{device.id}</code>
                  </div>
                </div>
              )

            case 'type':
              return (
                <Badge variant="outline" className="text-xs">
                  {device.device_type}
                </Badge>
              )

            case 'adapter':
              return (
                <div className="flex items-center gap-2">
                  <AdapterIcon className="h-3.5 w-3.5 text-muted-foreground" />
                  <Badge variant="outline" className="text-xs">
                    {device.adapter_type || 'mqtt'}
                  </Badge>
                </div>
              )

            case 'transforms':
              return <TransformsBadge deviceId={device.id} onRefresh={onRefresh} />

            case 'status':
              return <StatusBadge status={device.status} />

            case 'lastOnline':
              return (
                <span className="text-xs text-muted-foreground">
                  {formatTimestamp(device.last_seen, false)}
                </span>
              )

            default:
              return null
          }
        }}
        actions={[
          {
            label: t('devices:actions.viewDetails'),
            icon: <Eye className="h-4 w-4" />,
            onClick: (rowData) => {
              const device = rowData as unknown as Device
              onViewDetails(device)
            },
          },
          {
            label: t('common:delete'),
            icon: <Trash2 className="h-4 w-4" />,
            variant: 'destructive',
            onClick: (rowData) => {
              const device = rowData as unknown as Device
              onDelete(device.id)
            },
          },
        ]}
      />
    </>
  )
}
