import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import { ResponsiveTable } from "@/components/shared"
import { Eye, Pencil, Trash2, Download, MoreVertical, Cpu, Database, Activity } from "lucide-react"
import type { DeviceType } from "@/types"
import { api } from "@/lib/api"
import { useToast } from "@/hooks/use-toast"
import { TransformsBadge } from "@/components/automation"

interface DeviceTypeListProps {
  deviceTypes: DeviceType[]
  loading: boolean
  paginatedDeviceTypes: DeviceType[]
  deviceTypePage: number
  deviceTypesPerPage: number
  onRefresh: () => void
  onViewDetails: (type: DeviceType) => void
  onEdit: (type: DeviceType) => void
  onDelete: (id: string) => void
  onPageChange: (page: number) => void
  addTypeDialog: React.ReactNode
}

export function DeviceTypeList({
  deviceTypes,
  loading,
  paginatedDeviceTypes,
  deviceTypePage,
  deviceTypesPerPage,
  onRefresh,
  onViewDetails,
  onEdit,
  onDelete,
  onPageChange,
  addTypeDialog,
}: DeviceTypeListProps) {
  const { t } = useTranslation(['common', 'devices'])
  const { toast } = useToast()

  // Export single device type as JSON file
  const handleExport = async (deviceType: DeviceType) => {
    try {
      // Fetch full device type details with metrics and commands
      const fullType = await api.getDeviceType(deviceType.device_type)
      const data = JSON.stringify(fullType, null, 2)
      const blob = new Blob([data], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `device-type-${deviceType.device_type}.json`
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      URL.revokeObjectURL(url)
      toast({ title: t('common:success'), description: `Exported ${deviceType.name}` })
    } catch (error) {
      toast({ title: t('common:failed'), description: 'Failed to export device type', variant: 'destructive' })
    }
  }

  return (
    <>
      {/* Dialogs - addTypeDialog is controlled by parent PageTabs actions */}
      {addTypeDialog}

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
            label: t('devices:types.headers.name'),
          },
          {
            key: 'metrics',
            label: t('devices:types.headers.metrics'),
            align: 'center',
          },
          {
            key: 'commands',
            label: t('devices:types.headers.commands'),
            align: 'center',
          },
          {
            key: 'transforms',
            label: t('automation:transforms', { defaultValue: 'Transforms' }),
            align: 'center',
          },
        ]}
        data={paginatedDeviceTypes as unknown as Record<string, unknown>[]}
        rowKey={(type) => (type as unknown as DeviceType).device_type}
        loading={loading}
        renderCell={(columnKey, rowData) => {
          const type = rowData as unknown as DeviceType
          const index = paginatedDeviceTypes.indexOf(type)

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
                  <div className="w-9 h-9 rounded-lg flex items-center justify-center bg-blue-500/10 text-blue-600">
                    <Database className="h-4 w-4" />
                  </div>
                  <div>
                    <div className="font-medium text-sm">{type.name}</div>
                    <div className="flex items-center gap-2">
                      <code className="text-xs text-muted-foreground font-mono">{type.device_type}</code>
                      {type.description && (
                        <span className="text-xs text-muted-foreground line-clamp-1">
                          {type.description}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              )

            case 'metrics':
              return (
                <Badge variant="outline" className="text-xs bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-950/30 dark:text-blue-400 dark:border-blue-800">
                  {type.metrics?.length ?? type.metric_count ?? 0}
                </Badge>
              )

            case 'commands':
              return (
                <Badge variant="outline" className="text-xs bg-purple-50 text-purple-700 border-purple-200 dark:bg-purple-950/30 dark:text-purple-400 dark:border-purple-800">
                  {type.commands?.length ?? type.command_count ?? 0}
                </Badge>
              )

            case 'transforms':
              return <TransformsBadge deviceTypeId={type.device_type} onRefresh={onRefresh} />

            default:
              return null
          }
        }}
        actions={[
          {
            label: t('devices:types.actions.view'),
            icon: <Eye className="h-4 w-4" />,
            onClick: (rowData) => {
              const type = rowData as unknown as DeviceType
              onViewDetails(type)
            },
          },
          {
            label: t('devices:types.actions.export'),
            icon: <Download className="h-4 w-4" />,
            onClick: (rowData) => {
              const type = rowData as unknown as DeviceType
              handleExport(type)
            },
          },
          {
            label: t('common:edit'),
            icon: <Pencil className="h-4 w-4" />,
            onClick: (rowData) => {
              const type = rowData as unknown as DeviceType
              onEdit(type)
            },
          },
          {
            label: t('common:delete'),
            icon: <Trash2 className="h-4 w-4" />,
            variant: 'destructive',
            onClick: (rowData) => {
              const type = rowData as unknown as DeviceType
              onDelete(type.device_type)
            },
          },
        ]}
      />
    </>
  )
}
