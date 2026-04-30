import { useState, useCallback, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bluetooth, Wifi, Loader2, CheckCircle2, XCircle,
  Server, ChevronRight,
} from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useBleProvision } from '@/hooks/useBleProvision'
import { WifiState } from '@/lib/ble-protocol'
import { api, isTauriEnv } from '@/lib/api'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface BrokerOption {
  id: string
  label: string
}

interface BleProvisionTabProps {
  onComplete?: (deviceId: string) => void
}

type Phase = 'scan' | 'connecting' | 'connected' | 'provisioning' | 'done' | 'failed'

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function BleProvisionTab({ onComplete }: BleProvisionTabProps) {
  const { t } = useTranslation(['common', 'devices'])
  const ble = useBleProvision()

  const [phase, setPhase] = useState<Phase>('scan')
  const [wifiSsid, setWifiSsid] = useState('')
  const [wifiPassword, setWifiPassword] = useState('')
  const [deviceName, setDeviceName] = useState('')
  const [brokerId, setBrokerId] = useState('embedded')
  const [brokers, setBrokers] = useState<BrokerOption[]>([])
  const [registeredDeviceId, setRegisteredDeviceId] = useState<string | null>(null)
  const [serverSsid, setServerSsid] = useState<string | null>(null)

  useEffect(() => {
    api.get<{ brokers: Array<{ id: string; name: string; broker: string }> }>('/brokers')
      .then((data) => {
        const list: BrokerOption[] = [{ id: 'embedded', label: t('devices:ble.embeddedBroker') }]
        for (const b of data.brokers) {
          list.push({ id: b.id, label: b.name || b.broker })
        }
        setBrokers(list)
      })
      .catch(() => {})

    if (isTauriEnv()) {
      api.get<{ ssid: string; ip: string }>('/system/network-info')
        .then((d) => d.ssid && setServerSsid(d.ssid))
        .catch(() => {})
    }
  }, [t])

  // --- Step 1: Scan ---
  const handleScan = useCallback(async () => {
    const dev = await ble.scan()
    if (dev) setPhase('connecting')
  }, [ble])

  // --- Step 2: Connect + Provision ---
  const handleConnect = useCallback(async () => {
    if (!ble.device) return
    setPhase('provisioning')
    const ok = await ble.connectAndProvision(ble.device, wifiSsid, wifiPassword)
    if (ok) {
      setPhase('done')
    } else {
      setPhase('failed')
    }
  }, [ble, wifiSsid, wifiPassword])

  // --- Step 3: Pre-register ---
  const handleRegister = useCallback(async () => {
    if (!deviceName) return
    try {
      // Use NE101 as default model/SN since wifi_prov_mgr doesn't send device info
      const resp = await ble.preRegister({
        model: 'NE101',
        sn: ble.device?.id || 'unknown',
        deviceName,
        brokerId,
      })
      setRegisteredDeviceId(resp.device_id)
      onComplete?.(resp.device_id)
    } catch (err) {
      console.error('Pre-register failed:', err)
    }
  }, [ble, deviceName, brokerId, onComplete])

  const reset = useCallback(() => {
    ble.disconnect()
    setPhase('scan')
    setRegisteredDeviceId(null)
  }, [ble])

  // ---------------------------------------------------------------------------
  // Render helpers
  // ---------------------------------------------------------------------------

  const renderScan = () => (
    <div className="min-h-[480px] flex flex-col items-center justify-center gap-6 px-4">
      <div className="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center">
        <Bluetooth className="w-8 h-8 text-primary" />
      </div>
      <div className="text-center">
        <h3 className="text-lg font-medium">{t('devices:ble.scanTitle')}</h3>
        <p className="text-sm text-muted-foreground mt-1">{t('devices:ble.scanDesc')}</p>
      </div>
      <Button onClick={handleScan} disabled={ble.scanning}>
        {ble.scanning ? <Loader2 className="w-4 h-4 mr-2 animate-spin" /> : <Bluetooth className="w-4 h-4 mr-2" />}
        {t('devices:ble.scanBtn')}
      </Button>
      {serverSsid && (
        <p className="text-xs text-muted-foreground">
          {t('devices:ble.serverSsid', { ssid: serverSsid })}
        </p>
      )}
    </div>
  )

  const renderConfig = () => (
    <div className="max-w-md mx-auto py-4 space-y-6">
      <div className="text-center">
        <h3 className="text-lg font-medium">{t('devices:ble.configTitle')}</h3>
        <p className="text-sm text-muted-foreground">{ble.device?.name}</p>
      </div>

      <div className="rounded-xl border bg-card p-4 space-y-4">
        <div className="flex items-center gap-2 mb-2">
          <Wifi className="w-4 h-4 text-primary" />
          <span className="text-sm font-medium">{t('devices:ble.wifiConfig')}</span>
        </div>
        <div className="space-y-3">
          <div>
            <Label>{t('devices:ble.ssid')}</Label>
            <Input value={wifiSsid} onChange={(e) => setWifiSsid(e.target.value)} placeholder={serverSsid || 'SSID'} />
          </div>
          <div>
            <Label>{t('devices:ble.password')}</Label>
            <Input type="password" value={wifiPassword} onChange={(e) => setWifiPassword(e.target.value)} placeholder="••••••••" />
          </div>
        </div>
      </div>

      <div className="rounded-xl border bg-card p-4 space-y-3">
        <div className="flex items-center gap-2 mb-2">
          <Server className="w-4 h-4 text-primary" />
          <span className="text-sm font-medium">{t('devices:ble.mqttConfig')}</span>
        </div>
        <div>
          <Label>{t('devices:ble.deviceName')}</Label>
          <Input value={deviceName} onChange={(e) => setDeviceName(e.target.value)} placeholder="NE101 Camera" />
        </div>
        <div>
          <Label>{t('devices:ble.broker')}</Label>
          <Select value={brokerId} onValueChange={setBrokerId}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {brokers.map((b) => (
                <SelectItem key={b.id} value={b.id}>{b.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <Button
        className="w-full"
        onClick={handleConnect}
        disabled={!wifiSsid || !wifiPassword || ble.provisioning}
      >
        {ble.provisioning ? (
          <>
            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
            {t('devices:ble.provisioning')}
          </>
        ) : (
          <>
            <ChevronRight className="w-4 h-4 mr-2" />
            {t('devices:ble.apply')}
          </>
        )}
      </Button>

      {ble.wifiState !== null && (
        <div className="text-center text-sm text-muted-foreground">
          {ble.wifiState === WifiState.Connecting && t('devices:ble.status.net_connecting')}
          {ble.wifiState === WifiState.Connected && t('devices:ble.status.net_connected')}
          {ble.wifiState === WifiState.ConnectionFailed && t('devices:ble.status.failed')}
        </div>
      )}
    </div>
  )

  const renderDone = () => (
    <div className="min-h-[480px] flex flex-col items-center justify-center gap-6 px-4">
      <div className="w-16 h-16 rounded-2xl bg-success-light/50 flex items-center justify-center">
        <CheckCircle2 className="w-8 h-8 text-success" />
      </div>
      <div className="text-center">
        <h3 className="text-lg font-medium">{t('devices:ble.successTitle')}</h3>
        <p className="text-sm text-muted-foreground mt-1">{t('devices:ble.successDesc')}</p>
      </div>
      {!registeredDeviceId && (
        <Button onClick={handleRegister} disabled={!deviceName}>
          {t('devices:ble.registerDevice')}
        </Button>
      )}
      {registeredDeviceId && (
        <p className="text-sm text-success">{t('devices:ble.registered', { id: registeredDeviceId })}</p>
      )}
      <Button variant="outline" onClick={reset}>{t('common:close')}</Button>
    </div>
  )

  const renderFailed = () => (
    <div className="min-h-[480px] flex flex-col items-center justify-center gap-6 px-4">
      <div className="w-16 h-16 rounded-2xl bg-error-light/50 flex items-center justify-center">
        <XCircle className="w-8 h-8 text-error" />
      </div>
      <div className="text-center">
        <h3 className="text-lg font-medium">{t('devices:ble.failedTitle')}</h3>
        <p className="text-sm text-muted-foreground mt-1">{ble.error}</p>
      </div>
      <Button onClick={reset}>{t('common:retry')}</Button>
    </div>
  )

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (ble.error && phase === 'failed') return renderFailed()
  if (phase === 'done') return renderDone()
  if (phase === 'scan') return renderScan()
  return renderConfig() // connecting or provisioning — show config form
}
