import { useState, useCallback, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bluetooth, Wifi, Loader2, CheckCircle2, XCircle,
  Server, ChevronRight, ExternalLink,
} from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useBleProvision } from '@/hooks/useBleProvision'
import { WifiState, parseBleDeviceName, modelToDeviceType } from '@/lib/ble-protocol'
import type { BleMqttConfig } from '@/lib/ble-protocol'
import { api, fetchAPI, isTauriEnv } from '@/lib/api'

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

type Phase = 'scan' | 'config' | 'provisioning' | 'done' | 'failed'

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
  const [deviceId, setDeviceId] = useState<string | null>(null)
  const [registerError, setRegisterError] = useState<string | null>(null)

  useEffect(() => {
    api.get<{ brokers: Array<{ id: string; name: string; broker: string }> }>('/brokers')
      .then((data) => {
        const list: BrokerOption[] = [{ id: 'embedded', label: t('devices:ble.embeddedBroker') }]
        for (const b of data.brokers) {
          list.push({ id: b.id, label: b.name || b.broker })
        }
        setBrokers(list)
      })
      .catch((err) => {
        console.warn('[BLE] Failed to load broker list:', err)
      })
  }, [t])

  // --- Scan ---
  const handleScan = useCallback(async () => {
    const dev = await ble.scan()
    if (dev) setPhase('config')
  }, [ble])

  // --- Apply: preRegister (HTTP) → then pure BLE ---
  const handleConnect = useCallback(async () => {
    if (!ble.device) return

    const modelName = ble.deviceModel || parseBleDeviceName(ble.device.name || '')?.model
    const macSuffix = ble.deviceMac || parseBleDeviceName(ble.device.name || '')?.macSuffix
    if (!modelName) {
      ble.disconnect()
      setPhase('failed')
      return
    }

    const sn = macSuffix ? `${modelName}_${macSuffix}` : (ble.device.name || modelName)
    const deviceType = modelToDeviceType(modelName)
    const name = deviceName || ble.device.name || modelName

    setPhase('provisioning')
    setRegisterError(null)

    try {
      // Step A: HTTP pre-register (get MQTT config) — BEFORE BLE connect
      const resp = await fetchAPI<{ device_id: string; mqtt_config: BleMqttConfig }>(
        '/devices/ble-provision', {
          method: 'POST',
          body: JSON.stringify({
            model: modelName,
            sn,
            device_type: deviceType,
            device_name: name,
            broker_id: brokerId,
          }),
        },
      )
      setDeviceId(resp.device_id)

      // Step B: Pure BLE — connect → write MQTT → write WiFi → poll
      const ok = await ble.connectAndProvision(
        ble.device,
        wifiSsid,
        wifiPassword,
        resp.mqtt_config,
      )

      if (ok) {
        setPhase('done')
        onComplete?.(resp.device_id)
      } else {
        setPhase('failed')
      }
    } catch (err) {
      setRegisterError(err instanceof Error ? err.message : String(err))
      setPhase('failed')
    }
  }, [ble, wifiSsid, wifiPassword, deviceName, brokerId, onComplete])

  const reset = useCallback(() => {
    ble.disconnect()
    setPhase('scan')
    setDeviceId(null)
    setRegisterError(null)
  }, [ble])

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  const handleOpenInBrowser = useCallback(() => {
    const url = `${window.location.origin}${window.location.pathname}${window.location.search}`
    import('@tauri-apps/plugin-shell').then(({ open }) => open(url))
  }, [])

  const bleSupported = typeof navigator !== 'undefined' && !!navigator.bluetooth

  const modelName = ble.deviceModel || parseBleDeviceName(ble.device?.name || '')?.model
  const macSuffix = ble.deviceMac || parseBleDeviceName(ble.device?.name || '')?.macSuffix

  if (phase === 'scan' && !ble.device) {
    return (
      <div className="min-h-[480px] flex flex-col items-center justify-center gap-6 px-4">
        <div className="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center">
          <Bluetooth className="w-8 h-8 text-primary" />
        </div>
        <div className="text-center">
          <h3 className="text-lg font-medium">{t('devices:ble.scanTitle')}</h3>
          <p className="text-sm text-muted-foreground mt-1">{t('devices:ble.scanDesc')}</p>
        </div>
        {isTauriEnv() ? (
          <Button onClick={handleOpenInBrowser}>
            <ExternalLink className="w-4 h-4 mr-2" />
            {t('devices:ble.openInBrowser')}
          </Button>
        ) : !bleSupported ? (
          <div className="text-center space-y-3 max-w-sm">
            <p className="text-sm text-warning">{t('devices:ble.notSupported')}</p>
            <p className="text-xs text-muted-foreground">{t('devices:ble.notSupportedHint')}</p>
          </div>
        ) : (
          <Button onClick={handleScan} disabled={ble.scanning}>
            {ble.scanning ? <Loader2 className="w-4 h-4 mr-2 animate-spin" /> : <Bluetooth className="w-4 h-4 mr-2" />}
            {t('devices:ble.scanBtn')}
          </Button>
        )}
        {ble.error && <p className="text-sm text-error text-center max-w-sm">{ble.error}</p>}
      </div>
    )
  }

  if (phase === 'done') {
    return (
      <div className="min-h-[480px] flex flex-col items-center justify-center gap-4 px-4">
        <div className="w-16 h-16 rounded-2xl bg-success-light/50 flex items-center justify-center">
          <CheckCircle2 className="w-8 h-8 text-success" />
        </div>
        <div className="text-center">
          <h3 className="text-lg font-medium">{t('devices:ble.successTitle')}</h3>
          <p className="text-sm text-muted-foreground mt-1">{t('devices:ble.successDesc')}</p>
        </div>
        {deviceId && <p className="text-sm text-success">{t('devices:ble.registered', { id: deviceId })}</p>}
        <Button variant="outline" onClick={reset}>{t('common:close')}</Button>
      </div>
    )
  }

  if (phase === 'failed') {
    return (
      <div className="min-h-[480px] flex flex-col items-center justify-center gap-6 px-4">
        <div className="w-16 h-16 rounded-2xl bg-error-light/50 flex items-center justify-center">
          <XCircle className="w-8 h-8 text-error" />
        </div>
        <div className="text-center">
          <h3 className="text-lg font-medium">{t('devices:ble.failedTitle')}</h3>
          <p className="text-sm text-muted-foreground mt-1">{ble.error || registerError}</p>
        </div>
        <Button onClick={reset}>{t('common:retry')}</Button>
      </div>
    )
  }

  // Config or provisioning — show form + status
  return (
    <div className="max-w-md mx-auto py-4 space-y-6">
      <div className="text-center">
        <h3 className="text-lg font-medium">{t('devices:ble.configTitle')}</h3>
        <p className="text-sm text-muted-foreground">{ble.device?.name}</p>
        {modelName && (
          <p className="text-xs text-muted-foreground mt-1">
            {t('devices:ble.detectedModel', { model: modelName })}
            {macSuffix && <span className="ml-3">{t('devices:ble.detectedSn', { sn: macSuffix })}</span>}
          </p>
        )}
      </div>

      <div className="rounded-xl border bg-card p-4 space-y-4">
        <div className="flex items-center gap-2 mb-2">
          <Wifi className="w-4 h-4 text-primary" />
          <span className="text-sm font-medium">{t('devices:ble.wifiConfig')}</span>
        </div>
        <div className="space-y-3">
          <div>
            <Label>{t('devices:ble.ssid')}</Label>
            <Input value={wifiSsid} onChange={(e) => setWifiSsid(e.target.value)} placeholder="SSID" />
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
          <Input value={deviceName} onChange={(e) => setDeviceName(e.target.value)} placeholder={modelName || 'Camera'} />
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
        disabled={!wifiSsid || ble.provisioning || ble.connecting}
      >
        {ble.connecting || ble.provisioning ? (
          <>
            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
            {ble.provisioningStep === 'writingMqtt' && t('devices:ble.status.writingMqtt')}
            {ble.provisioningStep === 'writingWifi' && t('devices:ble.status.writingWifi')}
            {ble.provisioningStep === 'polling' && t('devices:ble.provisioning')}
            {ble.provisioningStep === 'connecting' && t('devices:ble.connectingTitle')}
            {!ble.provisioningStep && t('devices:ble.provisioning')}
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
}
