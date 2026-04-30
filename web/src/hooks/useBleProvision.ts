// useBleProvision Hook
//
// Encapsulates Web Bluetooth interactions with ESP-IDF wifi_prov_mgr.
// Protocol: Security0 (no encryption) + protobuf over BLE.
//
// Flow:
//   1. Scan for BLE devices (filter by NE101 name prefix + prov service UUID)
//   2. Connect → discover characteristics → start Security0 session
//   3. Write WiFi credentials (protobuf CmdSetConfig)
//   4. Poll connection status (CmdGetStatus)
//   5. On success: pre-register device via HTTP API → done

import { useState, useRef, useCallback } from 'react'

import {
  BLE_PROV_SERVICE_UUID,
  BLE_CHAR_CONFIG,
  BLE_CHAR_SESSION,
  WifiState,
  encodeSetConfig,
  encodeGetStatus,
  decodeRespGetStatus,
  decodeRespSetConfig,
  viewToBytes,
  MODEL_TO_DEVICE_TYPE,
} from '@/lib/ble-protocol'
import type { BleMqttConfig } from '@/lib/ble-protocol'
import { fetchAPI } from '@/lib/api'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface BleProvisionRequest {
  model: string
  sn: string
  device_type: string
  device_name: string
  broker_id: string
}

interface BleProvisionResponse {
  device_id: string
  mqtt_config: BleMqttConfig
}

export interface UseBleProvisionReturn {
  scanning: boolean
  connecting: boolean
  provisioning: boolean
  disconnected: boolean
  error: string | null
  device: BluetoothDevice | null
  wifiState: WifiState | null

  scan: () => Promise<BluetoothDevice | null>
  connectAndProvision: (device: BluetoothDevice, ssid: string, password: string) => Promise<boolean>
  preRegister: (params: { model: string; sn: string; deviceName: string; brokerId: string }) => Promise<BleProvisionResponse>
  disconnect: () => void
  clearError: () => void
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms)
    promise.then(
      (v) => { clearTimeout(timer); resolve(v) },
      (e) => { clearTimeout(timer); reject(e) },
    )
  })
}

async function writeAndRead(
  char: BluetoothRemoteGATTCharacteristic,
  data: Uint8Array,
  timeoutMs: number,
  label: string,
): Promise<DataView> {
  // Start notifications first
  await char.startNotifications()

  const respPromise = new Promise<DataView>((resolve) => {
    const handler = (e: Event) => {
      const t = e.target as unknown as BluetoothRemoteGATTCharacteristic
      t.removeEventListener('characteristicvaluechanged', handler)
      resolve(t.value!)
    }
    char.addEventListener('characteristicvaluechanged', handler)
  })

  await char.writeValue(data)
  const resp = await withTimeout(respPromise, timeoutMs, label)

  try { await char.stopNotifications() } catch { /* ignore */ }
  return resp
}

function sleep(ms: number) { return new Promise(r => setTimeout(r, ms)) }

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useBleProvision(): UseBleProvisionReturn {
  const [scanning, setScanning] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [provisioning, setProvisioning] = useState(false)
  const [disconnected, setDisconnected] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [device, setDevice] = useState<BluetoothDevice | null>(null)
  const [wifiState, setWifiState] = useState<WifiState | null>(null)

  const gattRef = useRef<BluetoothRemoteGATTServer | null>(null)
  const configCharRef = useRef<BluetoothRemoteGATTCharacteristic | null>(null)
  const disconnectHandlerRef = useRef<(() => void) | null>(null)

  const clearError = useCallback(() => setError(null), [])

  // --- scan ---

  const scan = useCallback(async (): Promise<BluetoothDevice | null> => {
    if (!navigator.bluetooth) {
      setError('Web Bluetooth is not supported')
      return null
    }
    setScanning(true)
    setError(null)
    setDisconnected(false)

    try {
      const selected = await navigator.bluetooth.requestDevice({
        filters: [
          { services: [BLE_PROV_SERVICE_UUID] },
          { namePrefix: 'NE101' },
        ],
        optionalServices: [BLE_PROV_SERVICE_UUID],
      })
      const onDisconnect = () => setDisconnected(true)
      selected.addEventListener('gattserverdisconnected', onDisconnect)
      disconnectHandlerRef.current = () =>
        selected.removeEventListener('gattserverdisconnected', onDisconnect)
      setDevice(selected)
      return selected
    } catch (err: unknown) {
      if (err instanceof DOMException && err.name === 'NotFoundError') return null
      setError(err instanceof Error ? err.message : String(err))
      return null
    } finally {
      setScanning(false)
    }
  }, [])

  // --- connect and provision ---

  const connectAndProvision = useCallback(
    async (bleDevice: BluetoothDevice, ssid: string, password: string): Promise<boolean> => {
      setConnecting(true)
      setError(null)
      setWifiState(null)

      try {
        // 1. Connect GATT
        const gatt = bleDevice.gatt!
        const server = await withTimeout(gatt.connect(), 15_000, 'GATT connect') as BluetoothRemoteGATTServer
        gattRef.current = server

        // 2. Get provisioning service
        const service = await withTimeout(
          server.getPrimaryService(BLE_PROV_SERVICE_UUID),
          10_000, 'Get prov service',
        )

        // 3. Get characteristics
        const sessionChar = await service.getCharacteristic(BLE_CHAR_SESSION)
        const configChar = await service.getCharacteristic(BLE_CHAR_CONFIG)
        configCharRef.current = configChar

        setConnecting(false)
        setProvisioning(true)

        // 4. Security0 session handshake (write empty bytes)
        try {
          await writeAndRead(sessionChar, new Uint8Array(0), 5_000, 'Session init')
        } catch {
          // Some implementations don't require session init
        }

        // 5. Send WiFi credentials
        const setConfigData = encodeSetConfig(ssid, password)
        const setResp = await writeAndRead(configChar, setConfigData, 10_000, 'Set WiFi config')
        const setStatus = decodeRespSetConfig(viewToBytes(setResp))
        if (setStatus !== 0) {
          throw new Error(`SetConfig failed with status ${setStatus}`)
        }

        // 6. Poll connection status (max 30s)
        setWifiState(WifiState.Connecting)
        for (let i = 0; i < 30; i++) {
          await sleep(1000)
          try {
            const statusResp = await writeAndRead(
              configChar, encodeGetStatus(), 5_000, 'Get status',
            )
            const { wifiState: ws } = decodeRespGetStatus(viewToBytes(statusResp))
            setWifiState(ws)

            if (ws === WifiState.Connected) {
              return true
            }
            if (ws === WifiState.ConnectionFailed) {
              throw new Error('WiFi connection failed — check SSID and password')
            }
            // Connecting / Disconnected → keep polling
          } catch (pollErr) {
            // If poll fails due to disconnect, propagate
            if (pollErr instanceof Error && pollErr.message.includes('timed out')) continue
            throw pollErr
          }
        }
        throw new Error('WiFi connection timed out after 30s')
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        setError(msg)
        return false
      } finally {
        setProvisioning(false)
        setConnecting(false)
      }
    }, [],
  )

  // --- preRegister ---

  const preRegister = useCallback(
    async (params: {
      model: string
      sn: string
      deviceName: string
      brokerId: string
    }): Promise<BleProvisionResponse> => {
      const deviceType = MODEL_TO_DEVICE_TYPE[params.model]
      if (!deviceType) throw new Error(`Unknown model: ${params.model}`)

      const body: BleProvisionRequest = {
        model: params.model,
        sn: params.sn,
        device_type: deviceType,
        device_name: params.deviceName,
        broker_id: params.brokerId,
      }
      return fetchAPI<BleProvisionResponse>('/devices/ble-provision', {
        method: 'POST',
        body: JSON.stringify(body),
      })
    }, [],
  )

  // --- disconnect ---

  const disconnect = useCallback(() => {
    disconnectHandlerRef.current?.()
    disconnectHandlerRef.current = null
    try { configCharRef.current?.stopNotifications?.() } catch { /* */ }
    try { gattRef.current?.disconnect() } catch { /* */ }
    gattRef.current = null
    configCharRef.current = null
    setDevice(null)
    setWifiState(null)
    setError(null)
  }, [])

  return {
    scanning, connecting, provisioning, disconnected, error, device, wifiState,
    scan, connectAndProvision, preRegister, disconnect, clearError,
  }
}
