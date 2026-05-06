import { useState, useEffect, useCallback, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bluetooth, Pencil, Search, RefreshCw, Cpu,
  Wifi, Globe, Radio,
  CheckCircle2, XCircle, Loader2,
  Copy, LucideIcon,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { textMini } from "@/design-system/tokens/typography"
import { useIsMobile } from '@/hooks/useMobile'

import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
  FullScreenDialogMain,
  FullScreenDialogSidebar,
  FullScreenDialogFooter,
} from '@/components/automation/dialog/FullScreenDialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { FormField } from '@/components/ui/field'
import { FormSection, FormSectionGroup } from '@/components/ui/form-section'
import { BleProvisionTab } from './BleProvisionTab'
import { toast } from '@/components/ui/use-toast'
import { api, getServerOrigin } from '@/lib/api'
import { useStore } from '@/store'
import { fetchCache } from '@/lib/utils/async'
import { validateUrl } from '@/lib/form-validation'
import type { DeviceType, AddDeviceRequest, ConnectionConfig, MqttStatus, ExternalBroker } from '@/types'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AddDeviceGlobalDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  deviceTypes: DeviceType[]
  onAdd: (request: AddDeviceRequest) => Promise<boolean>
  adding: boolean
}

type TabValue = 'ble' | 'manual' | 'auto'

interface TabItem {
  value: TabValue
  icon: LucideIcon
  labelKey: string
  descKey: string
}

// ---------------------------------------------------------------------------
// Tab definitions
// ---------------------------------------------------------------------------

const TABS: TabItem[] = [
  { value: 'ble', icon: Bluetooth, labelKey: 'devices:ble.tabTitle', descKey: 'devices:ble.tabDesc' },
  { value: 'manual', icon: Pencil, labelKey: 'devices:add.title', descKey: 'devices:add.tabDesc' },
  { value: 'auto', icon: Search, labelKey: 'devices:auto.title', descKey: 'devices:auto.tabDesc' },
]

function generateRandomId(): string {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  let result = ''
  for (let i = 0; i < 10; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  return result
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export function AddDeviceGlobalDialog({
  open,
  onOpenChange,
  deviceTypes,
  onAdd,
  adding,
}: AddDeviceGlobalDialogProps) {
  const { t } = useTranslation(['common', 'devices'])
  const isMobile = useIsMobile()
  const fetchDevices = useStore((s) => s.fetchDevices)
  const fetchDeviceTypes = useStore((s) => s.fetchDeviceTypes)
  const [activeTab, setActiveTab] = useState<TabValue>('ble')

  // Footer content supplied by the active tab
  const [footer, setFooter] = useState<ReactNode>(null)
  const setTabFooter = useCallback((node: ReactNode) => setFooter(node), [])

  // Ensure device types are loaded when dialog opens
  useEffect(() => {
    if (open) fetchDeviceTypes()
  }, [open, fetchDeviceTypes])

  // Reset footer when switching tabs (old tab unmounts, new tab hasn't set its footer yet)
  useEffect(() => {
    setFooter(null)
  }, [activeTab])

  return (
    <FullScreenDialog open={open} onOpenChange={onOpenChange}>
      <FullScreenDialogHeader
        icon={<Cpu className="w-5 h-5" />}
        iconBg="bg-info-light"
        iconColor="text-info"
        title={t('devices:addDevice')}
        subtitle={t('devices:globalDialog.subtitle')}
        onClose={() => onOpenChange(false)}
      />

      <FullScreenDialogContent>
        {/* Sidebar: tab navigation (hidden on mobile) */}
        <FullScreenDialogSidebar>
          <nav className="flex flex-col py-2 gap-0.5 px-2">
            {TABS.map((tab) => {
              const Icon = tab.icon
              const isActive = activeTab === tab.value
              return (
                <button
                  key={tab.value}
                  onClick={() => setActiveTab(tab.value)}
                  className={cn(
                    'flex items-center gap-2.5 px-3 py-3 rounded-lg text-sm font-medium transition-all text-left',
                    isActive
                      ? 'bg-card shadow-sm border border-primary/20 text-primary'
                      : 'text-muted-foreground hover:text-foreground hover:bg-muted-30 border border-transparent'
                  )}
                >
                  <div className={cn(
                    'w-8 h-8 rounded-lg flex items-center justify-center shrink-0',
                    isActive ? 'bg-primary text-primary-foreground' : 'bg-muted-30'
                  )}>
                    <Icon className="h-4 w-4" />
                  </div>
                  <div className="min-w-0">
                    <span className="truncate block">{t(tab.labelKey)}</span>
                    <span className={cn(
                      textMini, 'leading-tight block mt-0.5 truncate',
                      isActive ? 'text-primary/70' : 'text-muted-foreground'
                    )}>
                      {t(tab.descKey)}
                    </span>
                  </div>
                </button>
              )
            })}
          </nav>
        </FullScreenDialogSidebar>

        {/* Center column: main + footer stacked vertically */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Main scrollable content */}
          <FullScreenDialogMain>
            <div className={cn("max-w-2xl mx-auto py-6", isMobile ? "px-3" : "px-5 md:px-8")}>
              {/* Mobile tab bar - grid layout for equal width */}
              {isMobile && (
                <div className="grid grid-cols-3 gap-2 mb-5">
                  {TABS.map((tab) => {
                    const Icon = tab.icon
                    const isActive = activeTab === tab.value
                    return (
                      <button
                        key={tab.value}
                        onClick={() => setActiveTab(tab.value)}
                        className={cn(
                          'flex flex-col items-center gap-1.5 py-3 rounded-lg text-xs font-medium transition-all',
                          isActive
                            ? 'bg-primary text-primary-foreground'
                            : 'bg-muted-30 text-muted-foreground active:bg-muted'
                        )}
                      >
                        <Icon className="h-5 w-5" />
                        <span className="leading-tight">{t(tab.labelKey)}</span>
                      </button>
                    )
                  })}
                </div>
              )}

              {/* Tab content */}
              {activeTab === 'ble' && open && (
                <BleProvisionTab
                  onComplete={() => {
                    fetchCache.invalidate('devices')
                    fetchDevices()
                    onOpenChange(false)
                  }}
                />
              )}

              {activeTab === 'manual' && open && (
                <ManualAddForm
                  deviceTypes={deviceTypes}
                  onAdd={onAdd}
                  adding={adding}
                  onSuccess={() => onOpenChange(false)}
                  renderFooter={setTabFooter}
                />
              )}

              {activeTab === 'auto' && open && (
                <AutoDiscoveryTab renderFooter={setTabFooter} />
              )}
            </div>
          </FullScreenDialogMain>

          {/* Footer: action buttons from the active tab */}
          {footer && <FullScreenDialogFooter>{footer}</FullScreenDialogFooter>}
        </div>
      </FullScreenDialogContent>
    </FullScreenDialog>
  )
}

// ---------------------------------------------------------------------------
// Auto Discovery Tab — Rich guided experience
// ---------------------------------------------------------------------------

interface StepCardProps {
  step: number
  icon: LucideIcon
  title: string
  description: string
  status?: 'done' | 'active' | 'pending'
}

function StepCard({ step, icon: Icon, title, description, status = 'pending' }: StepCardProps) {
  return (
    <div className={cn(
      'relative flex gap-4 p-4 rounded-xl border transition-colors',
      status === 'done' && 'border-success bg-success-light',
      status === 'active' && 'border-primary bg-card',
      status === 'pending' && 'border-border bg-card',
    )}>
      {/* Step number */}
      <div className={cn(
        'w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold shrink-0',
        status === 'done' && 'bg-success text-primary-foreground',
        status === 'active' && 'bg-primary text-primary-foreground',
        status === 'pending' && 'bg-muted-30 text-muted-foreground',
      )}>
        {status === 'done' ? <CheckCircle2 className="h-4 w-4" /> : step}
      </div>
      <div className="flex-1 min-w-0 space-y-1">
        <div className="flex items-center gap-2">
          <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
          <h4 className="text-sm font-medium">{title}</h4>
        </div>
        <p className="text-xs text-muted-foreground leading-relaxed">{description}</p>
      </div>
    </div>
  )
}

function AutoDiscoveryTab({ renderFooter }: { renderFooter: (node: ReactNode) => void }) {
  const { t } = useTranslation(['devices', 'common'])
  const isMobile = useIsMobile()
  const [mqttStatus, setMqttStatus] = useState<MqttStatus | null>(null)
  const [brokers, setBrokers] = useState<ExternalBroker[]>([])
  const [loading, setLoading] = useState(true)
  const [activeMethod, setActiveMethod] = useState<'mqtt' | 'webhook'>('mqtt')

  useEffect(() => {
    renderFooter(null)
  }, [renderFooter])

  // Fetch MQTT status + full broker details
  useEffect(() => {
    Promise.all([
      api.getMqttStatus().catch(() => null),
      api.getBrokers().catch(() => null),
    ]).then(([statusRes, brokersRes]) => {
      setMqttStatus(statusRes?.status ?? null)
      setBrokers(brokersRes?.brokers ?? [])
    }).finally(() => setLoading(false))
  }, [])

  const brokerRunning = mqttStatus?.connected ?? false
  const serverIp = mqttStatus?.server_ip || getServerOrigin().replace(/^https?:\/\//, '')
  const brokerPort = mqttStatus?.listen_port ?? 1883
  const apiPort = new URL(getServerOrigin()).port || '9375'
  const webhookBaseUrl = `http://${serverIp}:${apiPort}/api/devices/webhook/`
  const enabledBrokers = brokers.filter(b => b.enabled)
  const hasAnyBroker = brokerRunning || enabledBrokers.length > 0

  // Determine step statuses
  const step1Status: StepCardProps['status'] = hasAnyBroker ? 'done' : 'active'
  const step2Status: StepCardProps['status'] = hasAnyBroker ? 'active' : 'pending'
  const step3Status: StepCardProps['status'] = hasAnyBroker ? 'active' : 'pending'

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <FormSectionGroup>
      {/* How it works — Step cards */}
      <FormSection
        title={t('devices:auto.howItWorks')}
        description={t('devices:auto.howItWorksDesc')}
      >
        <div className="space-y-3">
          <StepCard
            step={1}
            icon={Radio}
            title={t('devices:auto.step1_title')}
            description={t('devices:auto.step1_desc')}
            status={step1Status}
          />
          <StepCard
            step={2}
            icon={Wifi}
            title={t('devices:auto.step2_title')}
            description={t('devices:auto.step2_desc')}
            status={step2Status}
          />
          <StepCard
            step={3}
            icon={Search}
            title={t('devices:auto.step3_title')}
            description={t('devices:auto.step3_desc')}
            status={step3Status}
          />
        </div>
      </FormSection>

      {/* Connection methods — MQTT & Webhook */}
      <FormSection
        title={t('devices:auto.brokerInfo')}
        description={t('devices:auto.brokerInfoDesc')}
      >
        {/* Method toggle */}
        <div className="flex gap-2 mb-4">
          {([
            { key: 'mqtt' as const, icon: Radio, color: 'text-accent-orange', bg: 'bg-accent-orange-light' },
            { key: 'webhook' as const, icon: Globe, color: 'text-accent-purple', bg: 'bg-accent-purple-light' },
          ]).map(({ key, icon: MethodIcon, color, bg }) => (
            <button
              key={key}
              type="button"
              onClick={() => setActiveMethod(key)}
              className={cn(
                'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all',
                activeMethod === key
                  ? 'bg-card border border-primary shadow-sm'
                  : 'bg-muted-30 text-muted-foreground hover:bg-card'
              )}
            >
              <div className={cn('w-6 h-6 rounded flex items-center justify-center', bg)}>
                <MethodIcon className={cn('h-3.5 w-3.5', color)} />
              </div>
              {t(`devices:auto.method${key.charAt(0).toUpperCase() + key.slice(1)}`)}
            </button>
          ))}
        </div>

        {/* MQTT panel — built-in + all external brokers */}
        {activeMethod === 'mqtt' && (
          <div className="space-y-3">
            {/* Built-in broker */}
            <div className="rounded-xl border bg-card overflow-hidden">
              <div className={cn(
                'flex items-center gap-2 px-4 py-2.5 text-sm font-medium',
                brokerRunning
                  ? 'bg-success-light text-success'
                  : 'bg-error-light text-error'
              )}>
                {brokerRunning ? (
                  <CheckCircle2 className="h-4 w-4" />
                ) : (
                  <XCircle className="h-4 w-4" />
                )}
                {t('devices:auto.builtInBroker')}
                <span className="ml-auto">{brokerRunning ? t('devices:auto.brokerRunning') : t('devices:auto.brokerStopped')}</span>
              </div>
              <div className="p-4">
                <div className={cn("gap-3", isMobile ? "grid grid-cols-1" : "grid grid-cols-2")}>
                  <div className="space-y-1">
                    <span className="text-xs text-muted-foreground">{t('devices:auto.brokerAddress')}</span>
                    <code className="block text-sm font-mono bg-muted-30 rounded-md px-3 py-1.5">
                      {serverIp}
                    </code>
                  </div>
                  <div className="space-y-1">
                    <span className="text-xs text-muted-foreground">{t('devices:auto.brokerPort')}</span>
                    <code className="block text-sm font-mono bg-muted-30 rounded-md px-3 py-1.5">
                      {brokerPort}
                    </code>
                  </div>
                </div>
              </div>
            </div>

            {/* External brokers — full details from /api/brokers */}
            {enabledBrokers.map((broker) => (
              <div key={broker.id} className="rounded-xl border bg-card overflow-hidden">
                <div className={cn(
                  'flex items-center gap-2 px-4 py-2.5 text-sm font-medium',
                  broker.connected
                    ? 'bg-success-light text-success'
                    : 'bg-error-light text-error'
                )}>
                  {broker.connected ? (
                    <CheckCircle2 className="h-4 w-4" />
                  ) : (
                    <XCircle className="h-4 w-4" />
                  )}
                  <span className="truncate">{broker.name}</span>
                  <span className="ml-auto text-xs">{t('devices:auto.externalBroker')}</span>
                  {broker.tls && (
                    <span className="text-xs bg-muted-30 px-1.5 py-0.5 rounded">{t('devices:auto.tlsEnabled')}</span>
                  )}
                </div>
                <div className="p-4 space-y-3">
                  <div className={cn("gap-3", isMobile ? "grid grid-cols-1" : "grid grid-cols-2")}>
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">{t('devices:auto.brokerAddress')}</span>
                      <code className="block text-sm font-mono bg-muted-30 rounded-md px-3 py-1.5">
                        {broker.broker}
                      </code>
                    </div>
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">{t('devices:auto.brokerPort')}</span>
                      <code className="block text-sm font-mono bg-muted-30 rounded-md px-3 py-1.5">
                        {broker.port}
                      </code>
                    </div>
                  </div>
                  {/* Auth info */}
                  {broker.username && (
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">{t('devices:auto.authInfo')}</span>
                      <div className="flex items-center gap-2">
                        <code className="flex-1 text-sm font-mono bg-muted-30 rounded-md px-3 py-1.5">
                          {broker.username} / ••••••••
                        </code>
                      </div>
                    </div>
                  )}
                  {/* Subscribe topics */}
                  {broker.subscribe_topics && broker.subscribe_topics.length > 0 && (
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">{t('devices:auto.subscribeTopics')}</span>
                      <div className="space-y-1">
                        {broker.subscribe_topics.map((topic, i) => (
                          <div key={i} className="flex items-center gap-2">
                            <code className="flex-1 text-sm font-mono bg-muted-30 rounded-md px-3 py-1.5 break-all">
                              {topic}
                            </code>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="shrink-0 h-7 w-7"
                              onClick={() => {
                                navigator.clipboard.writeText(topic)
                                toast({ title: t('common:copied') })
                              }}
                            >
                              <Copy className="h-3.5 w-3.5" />
                            </Button>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Webhook panel */}
        {activeMethod === 'webhook' && (
          <div className="rounded-xl border bg-card p-4 space-y-3">
            <p className="text-sm text-muted-foreground">
              {t('devices:auto.webhookEndpointDesc')}
            </p>
            <div className="space-y-1">
              <span className="text-xs text-muted-foreground">{t('devices:auto.webhookEndpoint')}</span>
              <code className="block text-sm font-mono bg-muted-30 rounded-md px-3 py-2 break-all">
                {webhookBaseUrl}{'{device_id}'}
              </code>
            </div>
            <p className="text-xs text-muted-foreground">
              POST JSON to <code className="font-mono">{webhookBaseUrl}{'{your_device_id}'}</code>
            </p>
          </div>
        )}
      </FormSection>

      {/* Tips */}
      <FormSection title={t('devices:auto.tips')}>
        <div className="space-y-2">
          {[1, 2, 3].map((n) => (
            <div key={n} className="flex items-start gap-2 text-sm text-muted-foreground">
              <span className="text-xs leading-5 mt-px">&#8226;</span>
              <span>{t(`devices:auto.tip${n}`)}</span>
            </div>
          ))}
        </div>
      </FormSection>
    </FormSectionGroup>
  )
}

// ---------------------------------------------------------------------------
// Manual Add Form — Improved with adapter type cards
// ---------------------------------------------------------------------------

const ADAPTER_TYPES = [
  { value: 'mqtt' as const, icon: Radio, color: 'text-accent-orange', bg: 'bg-accent-orange-light' },
  { value: 'http' as const, icon: Globe, color: 'text-accent-cyan', bg: 'bg-accent-cyan-light' },
  { value: 'webhook' as const, icon: Wifi, color: 'text-accent-purple', bg: 'bg-accent-purple-light' },
]

function ManualAddForm({
  deviceTypes,
  onAdd,
  adding,
  onSuccess,
  renderFooter,
}: {
  deviceTypes: DeviceType[]
  onAdd: (request: AddDeviceRequest) => Promise<boolean>
  adding: boolean
  onSuccess: () => void
  renderFooter: (node: ReactNode) => void
}) {
  const { t } = useTranslation(['common', 'devices'])
  const isMobile = useIsMobile()

  const [selectedDeviceType, setSelectedDeviceType] = useState('')
  const [deviceId, setDeviceId] = useState('')
  const [deviceName, setDeviceName] = useState('')
  const [adapterType, setAdapterType] = useState<'mqtt' | 'http' | 'webhook'>('mqtt')
  const [connectionConfig, setConnectionConfig] = useState<ConnectionConfig>({})
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [serverIp, setServerIp] = useState('')

  // Fetch server IP for webhook URL
  useEffect(() => {
    api.getMqttStatus().then((res) => {
      if (res?.status?.server_ip) {
        setServerIp(res.status.server_ip)
      }
    }).catch(() => {})
  }, [])

  useEffect(() => {
    if (!deviceId) setDeviceId(generateRandomId())
  }, [])

  useEffect(() => {
    if (adapterType === 'mqtt' && selectedDeviceType && deviceId) {
      setConnectionConfig({
        telemetry_topic: `device/${selectedDeviceType}/${deviceId}/uplink`,
        command_topic: `device/${selectedDeviceType}/${deviceId}/downlink`,
      })
    } else if (adapterType === 'http') {
      setConnectionConfig({
        url: 'http://192.168.1.100/api/telemetry',
        method: 'GET',
        poll_interval: 30,
      })
    } else {
      setConnectionConfig({})
    }
  }, [adapterType, selectedDeviceType, deviceId])

  const handleSubmit = useCallback(async () => {
    const newErrors: Record<string, string> = {}
    if (!selectedDeviceType) {
      newErrors.deviceType = t('devices:deviceType') + ' is required'
    }
    if (adapterType === 'http' && connectionConfig.url) {
      const urlError = validateUrl(connectionConfig.url, 'URL')
      if (urlError) newErrors.httpUrl = urlError
    }
    setErrors(newErrors)
    if (Object.keys(newErrors).length > 0) return

    const request: AddDeviceRequest = {
      device_id: deviceId || undefined,
      name: deviceName || deviceId || selectedDeviceType,
      device_type: selectedDeviceType,
      adapter_type: adapterType,
      connection_config: connectionConfig,
    }

    const success = await onAdd(request)
    if (success) {
      toast({
        title: t('devices:add.success'),
        description: deviceId ? t('devices:add.successWithId', { deviceId }) : t('devices:add.successGeneric'),
      })
      onSuccess()
    } else {
      toast({
        title: t('devices:add.error'),
        description: t('devices:add.retryMessage'),
        variant: 'destructive',
      })
    }
  }, [selectedDeviceType, adapterType, connectionConfig, deviceId, deviceName, onAdd, onSuccess, t])

  // Push footer button to parent
  useEffect(() => {
    renderFooter(
      <Button onClick={handleSubmit} disabled={!selectedDeviceType || adding}>
        {adding ? t('common:loading') : t('common:add')}
      </Button>
    )
  }, [selectedDeviceType, adding, handleSubmit, renderFooter, t])

  const selectedTemplate = deviceTypes.find(dt => dt.device_type === selectedDeviceType)
  const hasCommands = (selectedTemplate?.commands?.length || 0) > 0

  return (
    <FormSectionGroup>
      {/* Basic Info */}
      <FormSection>
        {/* Device Type */}
        <FormField label={t('devices:deviceType')} required error={errors.deviceType}>
          <Select value={selectedDeviceType} onValueChange={(v) => { setSelectedDeviceType(v); setErrors(prev => { const next = { ...prev }; delete next.deviceType; return next }) }}>
            <SelectTrigger>
              <SelectValue placeholder={t('devices:add.typePlaceholder')} />
            </SelectTrigger>
            <SelectContent>
              {deviceTypes.map((type) => (
                <SelectItem key={type.device_type} value={type.device_type}>
                  {type.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>

        {/* Device ID & Name */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <FormField label={t('devices:deviceId')}>
            <div className="flex gap-2">
              <Input
                value={deviceId}
                onChange={(e) => setDeviceId(e.target.value)}
                placeholder={t('devices:id.autoGenerate')}
                className="font-mono"
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={() => setDeviceId(generateRandomId())}
                title={t('devices:id.regenerate')}
              >
                <RefreshCw className="h-4 w-4" />
              </Button>
            </div>
          </FormField>
          <FormField label={t('devices:deviceName')}>
            <Input
              value={deviceName}
              onChange={(e) => setDeviceName(e.target.value)}
              placeholder={t('common:optional')}
            />
          </FormField>
        </div>
      </FormSection>

      {/* Connection Settings Section */}
      <FormSection
        title={t('devices:add.connectionSettings')}
        description={t('devices:add.connectionSettingsDesc')}
      >
        {/* Adapter type — visual cards */}
        <div className={cn("gap-2", isMobile ? "grid grid-cols-1" : "grid grid-cols-3")}>
          {ADAPTER_TYPES.map(({ value, icon: AdapterIcon, color, bg }) => {
            const isActive = adapterType === value
            return (
              <button
                key={value}
                type="button"
                onClick={() => setAdapterType(value)}
                className={cn(
                  'flex flex-col items-center gap-2 p-3 rounded-xl border-2 transition-all text-center',
                  isActive
                    ? 'border-primary bg-card'
                    : 'border-transparent bg-muted-30 hover:bg-card hover:border-border'
                )}
              >
                <div className={cn(
                  'w-9 h-9 rounded-lg flex items-center justify-center',
                  bg
                )}>
                  <AdapterIcon className={cn('h-4.5 w-4.5', color)} />
                </div>
                <div className="min-w-0">
                  <span className={cn(
                    'text-sm font-medium block',
                    isActive ? 'text-foreground' : 'text-muted-foreground'
                  )}>
                    {t(`devices:add.adapterLabel_${value}`)}
                  </span>
                  <span className={cn(textMini, "text-muted-foreground leading-tight block mt-0.5 truncate max-w-[140px]")}>
                    {t(`devices:add.adapterDesc_${value}`)}
                  </span>
                </div>
              </button>
            )
          })}
        </div>

        {/* MQTT Config */}
        {adapterType === 'mqtt' && (
          <div className="space-y-3 pt-1">
            <FormField label={t('devices:add.telemetryTopic')}>
              <Input
                value={connectionConfig.telemetry_topic || ''}
                onChange={(e) => setConnectionConfig({ ...connectionConfig, telemetry_topic: e.target.value })}
                placeholder="device/{type}/{id}/uplink"
                className="font-mono text-sm"
              />
            </FormField>
            {hasCommands && (
              <FormField label={t('devices:add.commandTopic')}>
                <Input
                  value={connectionConfig.command_topic || ''}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, command_topic: e.target.value })}
                  placeholder="device/{type}/{id}/downlink"
                  className="font-mono text-sm"
                />
              </FormField>
            )}
          </div>
        )}

        {/* HTTP Config */}
        {adapterType === 'http' && (
          <div className="space-y-3 pt-1">
            <FormField label={t('devices:add.httpUrl')} error={errors.httpUrl}>
              <Input
                value={connectionConfig.url || ''}
                onChange={(e) => { setConnectionConfig({ ...connectionConfig, url: e.target.value }); setErrors(prev => { const next = { ...prev }; delete next.httpUrl; return next }) }}
                placeholder="http://192.168.1.100/api/telemetry"
                className="font-mono text-sm"
              />
            </FormField>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <FormField label={t('devices:add.requestMethod')}>
                <Select
                  value={connectionConfig.method || 'GET'}
                  onValueChange={(v) => setConnectionConfig({ ...connectionConfig, method: v })}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="GET">GET</SelectItem>
                    <SelectItem value="POST">POST</SelectItem>
                  </SelectContent>
                </Select>
              </FormField>
              <FormField label={t('devices:add.pollInterval')}>
                <Input
                  type="number"
                  min="1"
                  value={connectionConfig.poll_interval || 30}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, poll_interval: parseInt(e.target.value) || 30 })}
                />
              </FormField>
            </div>
          </div>
        )}

        {/* Webhook Config */}
        {adapterType === 'webhook' && (
          <div className="pt-1">
            <div className="rounded-lg border bg-muted-30 p-4">
              <p className="text-sm text-muted-foreground mb-2">
                {t('devices:add.webhookUrlDescription')}
              </p>
              <code className="text-xs break-all block font-mono">
                {serverIp
                  ? `http://${serverIp}:${new URL(getServerOrigin()).port || '9375'}/api/devices/webhook/${deviceId}`
                  : `${getServerOrigin()}/api/devices/webhook/${deviceId}`}
              </code>
            </div>
          </div>
        )}
      </FormSection>
    </FormSectionGroup>
  )
}
