import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { SubPageHeader } from "@/components/layout"
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ChevronLeft, RefreshCw, Activity, Send, Clock, Server } from "lucide-react"
import { toast } from "@/components/ui/use-toast"
import { formatTimestamp } from "@/lib/utils/format"
import type { Device, DeviceType, CommandDefinition, TelemetryDataResponse, CommandHistoryResponse } from "@/types"
import { isBase64Image } from "./utils"
import { EmptyStateInline } from "@/components/shared"

interface DeviceDetailProps {
  device: Device | null
  deviceType: DeviceType | null
  telemetryData: TelemetryDataResponse | null
  commandHistory: CommandHistoryResponse | null
  telemetryLoading: boolean
  selectedMetric: string | null
  onBack: () => void
  onRefresh: () => void
  onMetricClick: (metricName: string) => void
  onMetricBack: () => void
  onSendCommand: (commandName: string, params: string) => void
}

// Expanded metric with nested path support
interface ExpandedMetric {
  path: string[]
  displayName: string
  fullName: string
  value: unknown
  isNested: boolean
  depth: number
}

/**
 * Expand nested objects in current_values to show all levels
 * For example: { "environment": { "temperature": 25, "humidity": 60 } }
 * becomes: [
 *   { path: ["environment", "temperature"], displayName: "environment.temperature", value: 25 },
 *   { path: ["environment", "humidity"], displayName: "environment.humidity", value: 60 }
 * ]
 */
function expandNestedValues(
  values: Record<string, unknown>,
  parentPath: string[] = [],
  parentDisplayName: string = "",
  depth: number = 0
): ExpandedMetric[] {
  const result: ExpandedMetric[] = []

  for (const [key, value] of Object.entries(values)) {
    const currentPath = [...parentPath, key]
    const displayName = parentDisplayName ? `${parentDisplayName}.${key}` : key
    const fullName = currentPath.join('.')

    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      // Nested object - expand it
      const nested = expandNestedValues(value as Record<string, unknown>, currentPath, displayName, depth + 1)
      result.push(...nested)
    } else {
      // Leaf value - add to results
      result.push({
        path: currentPath,
        displayName,
        fullName,
        value,
        isNested: parentPath.length > 0,
        depth,
      })
    }
  }

  return result
}

// Render metric value - returns either a text string or an img element (truncated for table)
function renderMetricValue(
  value: unknown,
  dataType: string | undefined,
  _onImageClick: (dataUrl: string) => void,
  t: (key: string) => string
): React.ReactNode {
  if (value === null || value === undefined) return <span className="text-muted-foreground">-</span>
  if (typeof value === "boolean") return value ? t('command.dialog.yes') : t('command.dialog.no')
  if (typeof value === "number") {
    if (dataType === "integer") {
      return value.toLocaleString("zh-CN")
    }
    return parseFloat(value.toFixed(2)).toString()
  }
  if (typeof value === "string" && isBase64Image(value)) {
    return <span className="text-blue-500">📷 Image</span>
  }
  // Handle objects and arrays - show brief preview
  if (typeof value === "object" && value !== null) {
    if (Array.isArray(value)) {
      return <span className="text-muted-foreground">[{value.length} items]</span>
    }
    const keys = Object.keys(value)
    if (keys.length > 5) {
      return <span className="text-muted-foreground">{`{${keys[0]}, ${keys[1]}, ...} (${keys.length})`}</span>
    }
    // For objects with image field, show preview
    const objValue = value as Record<string, unknown>
    if (typeof objValue.image === 'string' && objValue.image.startsWith('data:image')) {
      return <span className="text-blue-500">📷 {keys.length} fields</span>
    }
    const compact = JSON.stringify(value)
    if (compact.length > 50) {
      return <span className="text-muted-foreground">{compact.substring(0, 47)}...</span>
    }
    return <span className="text-xs font-mono">{compact}</span>
  }
  return String(value)
}

export function DeviceDetail({
  device,
  deviceType,
  telemetryData,
  commandHistory,
  telemetryLoading,
  selectedMetric,
  onBack,
  onRefresh,
  onMetricClick,
  onMetricBack,
  onSendCommand,
}: DeviceDetailProps) {
  const { t } = useTranslation(['common', 'devices'])
  const [previewImageDataUrl, setPreviewImageDataUrl] = useState<string | null>(null)
  const [imagePreviewOpen, setImagePreviewOpen] = useState(false)
  const [commandDialogOpen, setCommandDialogOpen] = useState(false)
  const [selectedCommandDef, setSelectedCommandDef] = useState<CommandDefinition | null>(null)
  const [dialogParams, setDialogParams] = useState<Record<string, unknown>>({})

  // Get commands from template (simplified format) or legacy format
  const commands = deviceType?.commands || []

  const handleCommandClick = (cmd: CommandDefinition) => {
    setSelectedCommandDef(cmd)
    // Initialize params with default values
    const defaultParams: Record<string, unknown> = {}
    if (cmd.parameters) {
      cmd.parameters.forEach(param => {
        if (param.default_value !== undefined) {
          defaultParams[param.name] = param.default_value
        } else {
          // Set reasonable defaults based on data type
          switch (param.data_type) {
            case 'integer':
            case 'float':
              defaultParams[param.name] = 0
              break
            case 'boolean':
              defaultParams[param.name] = false
              break
            case 'string':
              defaultParams[param.name] = ''
              break
            default:
              defaultParams[param.name] = null
          }
        }
      })
    }
    setDialogParams(defaultParams)
    setCommandDialogOpen(true)
  }

  const handleSendCommand = () => {
    if (!selectedCommandDef) return

    // Convert params to JSON string for the API
    const paramsJson = JSON.stringify(dialogParams)
    onSendCommand(selectedCommandDef.name, paramsJson)
    setCommandDialogOpen(false)
    toast({
      title: t('devices:commandSent'),
      description: t('devices:command.dialog.sendCommand') + `: ${selectedCommandDef.display_name || selectedCommandDef.name}`,
    })
  }

  const updateParam = (name: string, value: unknown) => {
    setDialogParams(prev => ({ ...prev, [name]: value }))
  }

  const getMetricDisplayName = (metricName: string): string => {
    // First try exact match
    let metricDef = deviceType?.metrics?.find(m => m.name === metricName)
    if (metricDef) {
      return metricDef.display_name || metricName
    }

    // For nested paths like "environment.temperature", try matching parts
    if (metricName.includes('.')) {
      const parts = metricName.split('.')
      // Try matching the full path
      metricDef = deviceType?.metrics?.find(m => m.name === metricName)
      if (metricDef) {
        return metricDef.display_name || metricName
      }
      // Try matching the last part with its parent prefix
      const lastPart = parts[parts.length - 1]!
      const parentPrefix = parts.slice(0, -1).join('.')
      metricDef = deviceType?.metrics?.find(m => m.name === lastPart && m.display_name?.includes(parentPrefix))
      if (metricDef) {
        return metricDef.display_name || metricName
      }
      // Build display name from parts
      return metricName.split('.').map(part => {
        const def = deviceType?.metrics?.find(m => m.name === part)
        return def?.display_name || part
      }).join(' → ')
    }

    return metricName
  }

  const handleImageClick = (dataUrl: string) => {
    setPreviewImageDataUrl(dataUrl)
    setImagePreviewOpen(true)
  }

  if (!device) return null

  return (
    <>
      <div className="flex-1 flex flex-col overflow-hidden -mt-6">
        {/* Header */}
        <SubPageHeader
          title={device.name || device.id}
          description={t('devices:detail.deviceInfo', { deviceId: device.device_id || device.id, deviceType: device.device_type })}
          onBack={onBack}
          backLabel={t('common:back')}
          actions={
            <>
              <Badge variant={device.status === "online" ? "default" : "secondary"}>
                {device.status === "online" ? t('devices:status.online') : t('devices:status.offline')}
              </Badge>
              <Button variant="outline" size="sm" onClick={onRefresh} disabled={telemetryLoading}>
                <RefreshCw className={`h-4 w-4 ${telemetryLoading ? "animate-spin" : ""}`} />
              </Button>
            </>
          }
          className="border-b px-6 py-2 -mx-6 mb-2"
        />

        {/* Content with Tabs */}
        <div className="flex-1 overflow-hidden">
          <Tabs defaultValue="connection" className="h-full flex flex-col">
            <div className="px-6 pb-2">
              <TabsList>
                <TabsTrigger value="connection">
                  <Server className="mr-2 h-4 w-4" />
                  {t('devices:detail.tabs.connection')}
                </TabsTrigger>
                <TabsTrigger value="metrics">
                  <Activity className="mr-2 h-4 w-4" />
                  {t('devices:detail.tabs.metrics')}
                </TabsTrigger>
                <TabsTrigger value="commands" className={commands.length === 0 ? "hidden" : ""}>
                  <Send className="mr-2 h-4 w-4" />
                  {t('devices:detail.tabs.commands')}
                </TabsTrigger>
                <TabsTrigger value="history">
                  <Clock className="mr-2 h-4 w-4" />
                  {t('devices:detail.tabs.history')}
                </TabsTrigger>
              </TabsList>
            </div>

            {/* Connection Tab */}
            <TabsContent value="connection" className="flex-1 overflow-auto px-6 pb-6 m-0">
              <div className="space-y-4">
                <Card>
                  <CardContent className="pt-6">
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">{t('devices:detail.fields.id')}</p>
                        <p className="text-sm font-mono font-medium">{device.device_id || device.id || '-'}</p>
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">{t('devices:detail.fields.name')}</p>
                        <p className="text-sm font-medium">{device.name || '-'}</p>
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">{t('devices:detail.fields.type')}</p>
                        <p className="text-sm font-medium">{device.device_type || '-'}</p>
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">{t('devices:detail.fields.status')}</p>
                        <div className="flex items-center gap-2">
                          <span className={`h-2 w-2 rounded-full ${
                            device.status === 'online' ? 'bg-green-500' :
                            device.status === 'offline' ? 'bg-gray-400' :
                            'bg-red-500'
                          }`} />
                          <p className="text-sm font-medium capitalize">{device.status || t('devices:status.unknown')}</p>
                        </div>
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">{t('devices:detail.fields.lastOnline')}</p>
                        <p className="text-sm font-medium">
                          {device.last_seen ? formatTimestamp(new Date(device.last_seen).getTime() / 1000) : '-'}
                        </p>
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">{t('devices:detail.fields.adapter')}</p>
                        <div className="flex items-center gap-1">
                          <Badge variant="outline" className="text-xs">
                            {device.plugin_name || t('devices:builtinMqtt')}
                          </Badge>
                          {device.plugin_id && device.plugin_id !== 'internal-mqtt' && (
                            <span className="text-xs text-muted-foreground">({device.plugin_id})</span>
                          )}
                        </div>
                      </div>
                      {/* Connection Config - MQTT Topics */}
                      {device.connection_config?.telemetry_topic && (
                        <div className="space-y-1 md:col-span-2 lg:col-span-3">
                          <p className="text-xs text-muted-foreground flex items-center gap-1">
                            <Activity className="h-3 w-3" />
                            {t('devices:detail.fields.telemetryTopic') || 'Telemetry Topic'}
                          </p>
                          <p className="text-sm font-mono text-xs bg-muted px-2 py-1 rounded">
                            {device.connection_config.telemetry_topic}
                          </p>
                        </div>
                      )}
                      {device.connection_config?.command_topic && (
                        <div className="space-y-1 md:col-span-2 lg:col-span-3">
                          <p className="text-xs text-muted-foreground flex items-center gap-1">
                            <Send className="h-3 w-3" />
                            {t('devices:detail.fields.commandTopic') || 'Command Topic'}
                          </p>
                          <p className="text-sm font-mono text-xs bg-muted px-2 py-1 rounded">
                            {device.connection_config.command_topic}
                          </p>
                        </div>
                      )}
                      {/* Fallback to default topic format if connection_config not available */}
                      {!device.connection_config?.telemetry_topic && !device.connection_config?.command_topic && (
                        <>
                          <div className="space-y-1 md:col-span-2 lg:col-span-3">
                            <p className="text-xs text-muted-foreground flex items-center gap-1">
                              <Activity className="h-3 w-3" />
                              {t('devices:detail.fields.telemetryTopic') || 'Telemetry Topic'}
                            </p>
                            <p className="text-sm font-mono text-xs bg-muted px-2 py-1 rounded">
                              device/{device.device_type}/{device.device_id || device.id}/telemetry
                            </p>
                          </div>
                          <div className="space-y-1 md:col-span-2 lg:col-span-3">
                            <p className="text-xs text-muted-foreground flex items-center gap-1">
                              <Send className="h-3 w-3" />
                              {t('devices:detail.fields.commandTopic') || 'Command Topic'}
                            </p>
                            <p className="text-sm font-mono text-xs bg-muted px-2 py-1 rounded">
                              device/{device.device_type}/{device.device_id || device.id}/commands
                            </p>
                          </div>
                        </>
                      )}
                    </div>
                  </CardContent>
                </Card>
              </div>
            </TabsContent>

            {/* Metrics Tab */}
            <TabsContent value="metrics" className="flex-1 overflow-auto px-6 pb-6 m-0">

              {selectedMetric ? (
                // Metric History Detail View
                <div className="space-y-4">
                  <div className="flex items-center gap-2">
                    <Button variant="ghost" size="sm" onClick={onMetricBack}>
                      <ChevronLeft className="mr-1 h-4 w-4" />
                      {t('common:back')}
                    </Button>
                    <span className="text-sm text-muted-foreground">{getMetricDisplayName(selectedMetric)}</span>
                  </div>

                  {telemetryData && selectedMetric in telemetryData.data ? (
                    <Card>
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>{t('devices:detail.metricHistory.time')}</TableHead>
                            <TableHead>{t('devices:detail.metricHistory.value')}</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {telemetryData.data[selectedMetric].length > 0 ? (
                            telemetryData.data[selectedMetric]
                              .slice()
                              .reverse()
                              .slice(0, 100)
                              .map((point, index) => (
                                <TableRow key={index}>
                                  <TableCell className="text-sm text-muted-foreground">
                                    {formatTimestamp(point.timestamp)}
                                  </TableCell>
                                  <TableCell>{renderMetricValue(point.value, undefined, handleImageClick, t)}</TableCell>
                                </TableRow>
                              ))
                          ) : (
                            <EmptyStateInline title={t('devices:detail.metricHistory.noData')} colSpan={2} />
                          )}
                        </TableBody>
                      </Table>
                    </Card>
                  ) : (
                    <Card>
                      <CardContent className="flex items-center justify-center py-12">
                        <p className="text-muted-foreground">{t('devices:detail.noHistory')}</p>
                      </CardContent>
                    </Card>
                  )}
                </div>
              ) : deviceType?.metrics && deviceType.metrics.length > 0 ? (
                // Defined metrics - show card grid with expanded nested values
                device.current_values && Object.keys(device.current_values).length > 0 ? (
                  (() => {
                    // Expand nested values to show all levels
                    const expandedMetrics = expandNestedValues(device.current_values)

                    if (expandedMetrics.length === 0) {
                      return (
                        <Card>
                          <CardContent className="flex items-center justify-center py-12">
                            <p className="text-muted-foreground">{t('devices:detail.noMetrics')}</p>
                          </CardContent>
                        </Card>
                      )
                    }

                    return (
                      <div className="space-y-4">
                        {/* Group by top-level metric for better organization */}
                        {(() => {
                          const grouped = new Map<string, ExpandedMetric[]>()
                          expandedMetrics.forEach(m => {
                            const topLevel = m.path[0]!
                            if (!grouped.has(topLevel)) {
                              grouped.set(topLevel, [])
                            }
                            grouped.get(topLevel)!.push(m)
                          })

                          return Array.from(grouped.entries()).map(([topLevel, subMetrics]) => {
                            const topLevelValue = device.current_values?.[topLevel]
                            const isNestedGroup = subMetrics.some(m => m.isNested)
                            const metricDef = deviceType?.metrics?.find(m => m.name === topLevel)
                            const groupDisplayName = metricDef?.display_name || topLevel

                            return (
                              <div key={topLevel}>
                                {isNestedGroup ? (
                                  // Show nested metrics as a group
                                  <Card>
                                    <CardHeader className="pb-3">
                                      <CardTitle className="text-base flex items-center gap-2">
                                        <span>{groupDisplayName}</span>
                                        <Badge variant="outline" className="text-xs">
                                          {subMetrics.length} {t('devices:detail.fields') || 'fields'}
                                        </Badge>
                                      </CardTitle>
                                    </CardHeader>
                                    <CardContent>
                                      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                                        {subMetrics.map((metric) => {
                                          const subMetricDef = deviceType?.metrics?.find(m => m.name === metric.fullName)
                                          const displayName = subMetricDef?.display_name || metric.displayName
                                          const unit = subMetricDef?.unit || ""
                                          const dataType = subMetricDef?.data_type || ""

                                          return (
                                            <Card
                                              key={metric.fullName}
                                              className={`cursor-pointer transition-all hover:border-primary/50 ${
                                                metric.isNested ? "border-l-2 border-l-primary/30" : ""
                                              }`}
                                              onClick={() => onMetricClick(metric.fullName)}
                                            >
                                              <CardContent className="p-3">
                                                <div className="space-y-1">
                                                  <p className="text-xs text-muted-foreground truncate" title={displayName}>
                                                    {displayName}
                                                  </p>
                                                  <p className="text-lg font-semibold truncate">
                                                    {renderMetricValue(metric.value, dataType, handleImageClick, t)}
                                                    {unit && <span className="text-sm font-normal text-muted-foreground ml-1">{unit}</span>}
                                                  </p>
                                                  <p className="text-xs text-muted-foreground font-mono truncate" title={metric.fullName}>
                                                    {metric.fullName}
                                                  </p>
                                                </div>
                                              </CardContent>
                                            </Card>
                                          )
                                        })}
                                      </div>
                                    </CardContent>
                                  </Card>
                                ) : (
                                  // Show single metric card
                                  <Card
                                    key={topLevel}
                                    className="cursor-pointer transition-all hover:border-primary/50"
                                    onClick={() => onMetricClick(topLevel)}
                                  >
                                    <CardHeader className="pb-3">
                                      <CardDescription className="text-xs truncate" title={groupDisplayName}>
                                        {groupDisplayName}
                                      </CardDescription>
                                    </CardHeader>
                                    <CardContent>
                                      <div className="text-2xl font-bold truncate">
                                        {renderMetricValue(topLevelValue, metricDef?.data_type, handleImageClick, t)}
                                        {metricDef?.unit && <span className="text-base font-normal text-muted-foreground ml-1">{metricDef.unit}</span>}
                                      </div>
                                      <div className="text-xs text-muted-foreground mt-1 truncate" title={topLevel}>
                                        {topLevel}
                                      </div>
                                    </CardContent>
                                  </Card>
                                )}
                              </div>
                            )
                          })
                        })()}
                      </div>
                    )
                  })()
                ) : (
                  <Card>
                    <CardContent className="flex items-center justify-center py-12">
                      <p className="text-muted-foreground">{t('devices:detail.noMetrics')}</p>
                    </CardContent>
                  </Card>
                )
              ) : (
                // No metrics defined - show raw data table
                <div className="space-y-4">
                  {telemetryData && Object.keys(telemetryData.data).length > 0 ? (
                    Object.entries(telemetryData.data).map(([metricName, points]) => (
                      <Card key={metricName}>
                        <CardHeader className="pb-3">
                          <CardTitle className="text-base">{metricName}</CardTitle>
                          <CardDescription className="text-xs">
                            {points.length} {t('devices:detail.rawData.records')}
                          </CardDescription>
                        </CardHeader>
                        <CardContent>
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead className="w-40">{t('devices:detail.metricHistory.time')}</TableHead>
                                <TableHead>{t('devices:detail.metricHistory.value')}</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {points.length > 0 ? (
                                points
                                  .slice()
                                  .reverse()
                                  .slice(0, 50)
                                  .map((point, index) => {
                                    const fullValue = typeof point.value === 'object' ? JSON.stringify(point.value, null, 2) : String(point.value)
                                    return (
                                      <TableRow key={index}>
                                        <TableCell className="text-sm text-muted-foreground whitespace-nowrap">
                                          {formatTimestamp(point.timestamp)}
                                        </TableCell>
                                        <TableCell className="font-mono text-xs min-w-0 max-w-md">
                                          <div className="truncate" title={fullValue}>
                                            {renderMetricValue(point.value, undefined, handleImageClick, t)}
                                          </div>
                                        </TableCell>
                                      </TableRow>
                                    )
                                  })
                              ) : (
                                <EmptyStateInline title={t('devices:detail.metricHistory.noData')} colSpan={2} />
                              )}
                            </TableBody>
                          </Table>
                        </CardContent>
                      </Card>
                    ))
                  ) : (
                    <Card>
                      <CardContent className="flex items-center justify-center py-12">
                        <p className="text-muted-foreground">{t('devices:detail.rawData.noData')}</p>
                      </CardContent>
                    </Card>
                  )}
                </div>
              )}
            </TabsContent>

            {/* Commands Tab */}
            <TabsContent value="commands" className="flex-1 overflow-auto px-6 pb-6 m-0">
              <div className="space-y-4">
                {commands.length === 0 ? (
                  <Card>
                    <CardContent className="flex items-center justify-center py-12">
                      <p className="text-muted-foreground">{t('devices:noCommands')}</p>
                    </CardContent>
                  </Card>
                ) : (
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {commands.map((cmd: CommandDefinition) => (
                      <Card
                        key={cmd.name}
                        className="cursor-pointer transition-all hover:border-primary/50 hover:shadow-md"
                        onClick={() => handleCommandClick(cmd)}
                      >
                        <CardHeader>
                          <div className="flex items-start justify-between">
                            <div className="flex-1">
                              <CardTitle className="text-base">{cmd.display_name || cmd.name}</CardTitle>
                              <CardDescription className="text-xs font-mono mt-1">
                                {cmd.name}
                              </CardDescription>
                            </div>
                            <Send className="h-4 w-4 text-muted-foreground" />
                          </div>
                        </CardHeader>
                        <CardContent>
                          {cmd.parameters && cmd.parameters.length > 0 ? (
                            <div className="space-y-1">
                              <div className="text-xs text-muted-foreground">{t('devices:command.parameters')}</div>
                              <div className="flex flex-wrap gap-1">
                                {cmd.parameters.map((param) => (
                                  <Badge key={param.name} variant="secondary" className="text-xs">
                                    {param.display_name || param.name}
                                    <span className="text-muted-foreground ml-1">({param.data_type})</span>
                                  </Badge>
                                ))}
                              </div>
                            </div>
                          ) : (
                            <p className="text-xs text-muted-foreground">{t('devices:command.noParameters')}</p>
                          )}
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                )}
              </div>
            </TabsContent>

            {/* Command History Tab */}
            <TabsContent value="history" className="flex-1 overflow-auto px-6 pb-6 m-0">
              <Card>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t('devices:history.time')}</TableHead>
                      <TableHead>{t('devices:history.command')}</TableHead>
                      <TableHead>{t('devices:history.parameters')}</TableHead>
                      <TableHead align="center">{t('devices:history.status')}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {commandHistory && commandHistory.commands.length > 0 ? (
                      commandHistory.commands.map((entry, index) => (
                        <TableRow key={index}>
                          <TableCell className="text-sm">
                            {entry.timestamp ? formatTimestamp(entry.timestamp) : "-"}
                          </TableCell>
                          <TableCell className="font-mono text-xs">{entry.command}</TableCell>
                          <TableCell className="text-sm text-muted-foreground max-w-[200px] truncate">
                            {JSON.stringify(entry.parameters)}
                          </TableCell>
                          <TableCell align="center">
                            {entry.success ? (
                              <Badge className="badge-success">{t('devices:history.success')}</Badge>
                            ) : (
                              <Badge variant="destructive">{t('devices:history.failed')}</Badge>
                            )}
                          </TableCell>
                        </TableRow>
                      ))
                    ) : (
                      <EmptyStateInline title={commandHistory?.note || t('devices:history.noRecords')} colSpan={4} />
                    )}
                  </TableBody>
                </Table>
              </Card>
            </TabsContent>
          </Tabs>
        </div>
      </div>

      {/* Image Preview Dialog */}
      {imagePreviewOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80" onClick={() => setImagePreviewOpen(false)}>
          <div className="max-w-4xl max-h-[90vh] p-4" onClick={(e) => e.stopPropagation()}>
            {previewImageDataUrl && (
              <img
                src={previewImageDataUrl}
                alt="Preview"
                className="max-w-full max-h-[85vh] object-contain rounded"
              />
            )}
            <div className="flex justify-center mt-4">
              <Button onClick={() => setImagePreviewOpen(false)}>{t('common:close')}</Button>
            </div>
          </div>
        </div>
      )}

      {/* Command Send Dialog */}
      {selectedCommandDef && (
        <Dialog open={commandDialogOpen} onOpenChange={setCommandDialogOpen}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Send className="h-5 w-5" />
                {selectedCommandDef.display_name || selectedCommandDef.name}
              </DialogTitle>
              <DialogDescription className="font-mono text-xs">
                {selectedCommandDef.name}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-4">
              {selectedCommandDef.parameters && selectedCommandDef.parameters.length > 0 ? (
                <div className="space-y-4">
                  {selectedCommandDef.parameters.map((param) => {
                    const value = dialogParams[param.name as keyof typeof dialogParams]

                    return (
                      <div key={param.name} className="space-y-2">
                        <Label htmlFor={`param-${param.name}`} className="flex items-center gap-2">
                          <span>{param.display_name || param.name}</span>
                          <Badge variant="outline" className="text-xs py-0 px-1">
                            {param.data_type}
                          </Badge>
                        </Label>

                        {param.data_type === 'boolean' ? (
                          <div className="flex items-center gap-2">
                            <button
                              type="button"
                              onClick={() => updateParam(param.name, true)}
                              className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                                value === true
                                  ? 'bg-primary text-primary-foreground'
                                  : 'bg-muted text-muted-foreground hover:bg-muted/80'
                              }`}
                            >
                              {t('command.dialog.yes')}
                            </button>
                            <button
                              type="button"
                              onClick={() => updateParam(param.name, false)}
                              className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                                value === false
                                  ? 'bg-primary text-primary-foreground'
                                  : 'bg-muted text-muted-foreground hover:bg-muted/80'
                              }`}
                            >
                              {t('command.dialog.no')}
                            </button>
                          </div>
                        ) : param.allowed_values && param.allowed_values.length > 0 ? (
                          <div className="flex flex-wrap gap-2">
                            {param.allowed_values.map((allowedValue) => (
                              <button
                                key={JSON.stringify(allowedValue)}
                                type="button"
                                onClick={() => updateParam(param.name, allowedValue)}
                                className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                                  value === allowedValue
                                    ? 'bg-primary text-primary-foreground'
                                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                                }`}
                              >
                                {String(allowedValue)}
                              </button>
                            ))}
                          </div>
                        ) : (
                          <Input
                            id={`param-${param.name}`}
                            type={param.data_type === 'integer' || param.data_type === 'float' ? 'number' : 'text'}
                            value={value === undefined || value === null ? '' : String(value)}
                            onChange={(e) => {
                              const inputValue = e.target.value
                              let newValue: unknown = inputValue

                              if (param.data_type === 'integer') {
                                newValue = inputValue === '' ? 0 : parseInt(inputValue, 10)
                              } else if (param.data_type === 'float') {
                                newValue = inputValue === '' ? 0 : parseFloat(inputValue)
                              }

                              updateParam(param.name, newValue)
                            }}
                            placeholder={param.default_value !== undefined ? t('command.dialog.defaultParam', { value: JSON.stringify(param.default_value) }) : t('command.dialog.pleaseEnter')}
                          />
                        )}

                        {param.min !== undefined || param.max !== undefined ? (
                          <p className="text-xs text-muted-foreground">
                            {t('command.dialog.range', { min: param.min ?? '-∞', max: param.max ?? '+∞' })}
                          </p>
                        ) : null}
                      </div>
                    )
                  })}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground text-center py-4">
                  {t('devices:command.dialog.noParamsNeeded')}
                </p>
              )}

              {/* JSON Preview */}
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">{t('devices:command.dialog.jsonPreview')}</Label>
                <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                  {JSON.stringify(dialogParams, null, 2)}
                </pre>
              </div>
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={() => setCommandDialogOpen(false)}>
                {t('common:cancel')}
              </Button>
              <Button onClick={handleSendCommand}>
                <Send className="h-4 w-4 mr-2" />
                {t('devices:command.dialog.sendCommand')}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </>
  )
}
