// useBleProvision Hook
//
// Encapsulates Web Bluetooth interactions with ESP-IDF wifi_prov_mgr.
// Protocol: Security0 (no encryption) + protobuf over BLE.
//
// Flow (pure BLE, no HTTP during connection):
//   1. Scan for BLE devices (filter by prov service UUID)
//   2. Parse device name → extract model + SN
//   3. GATT connect → discover characteristics
//   4. Write MQTT config to custom BLE endpoint (JSON)
//   5. Write WiFi credentials (protobuf CmdSetConfig)
//   6. Poll connection status (CmdGetStatus)

import { useState, useRef, useCallback } from 'react'

import {
  BLE_PROV_SERVICE_UUID,
  BLE_CHAR_CONFIG,
  BLE_CHAR_MQTT,
  WifiState,
  encodeSetConfig,
  encodeGetStatus,
  encodeMqttConfig,
  parseBleDeviceName,
  decodeRespGetStatus,
  decodeRespSetConfig,
  viewToBytes,
  writeWithMtu,
} from '@/lib/ble-protocol'
import type { BleMqttConfig } from '@/lib/ble-protocol'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface UseBleProvisionReturn {
  scanning: boolean
  connecting: boolean
  provisioning: boolean
  disconnected: boolean
  error: string | null
  device: BluetoothDevice | null
  deviceModel: string | null
  deviceMac: string | null
  wifiState: WifiState | null
  provisioningStep: string | null

  scan: () => Promise<BluetoothDevice | null>
  /** Pure BLE: connect → write MQTT → write WiFi → poll. No HTTP inside. */
  connectAndProvision: (
    device: BluetoothDevice,
    ssid: string,
    password: string,
    mqttConfig: BleMqttConfig,
  ) => Promise<boolean>
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
  label: string,
): Promise<DataView> {
  // Firmware characteristics only support READ|WRITE (no NOTIFY).
  // Write request, wait briefly for device handler to set response, then read.
  console.log(`[BLE] writeAndRead "${label}": writing ${data.byteLength} bytes`)
  await char.writeValueWithResponse(data)
  console.log(`[BLE] writeAndRead "${label}": write OK, reading...`)

  // Retry read up to 3 times with 300ms delay — the ESP-IDF protocomm handler
  // sets the attribute value asynchronously on write, so we may need a brief wait.
  for (let attempt = 0; attempt < 3; attempt++) {
    await sleep(300)
    const value = await char.readValue()
    if (value.byteLength > 0) {
      console.log(`[BLE] writeAndRead "${label}": read OK, ${value.byteLength} bytes`)
      return value
    }
    console.log(`[BLE] writeAndRead "${label}": read returned empty (attempt ${attempt + 1})`)
  }
  throw new Error(`${label}: read returned empty after 3 attempts`)
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
  const [deviceModel, setDeviceModel] = useState<string | null>(null)
  const [deviceMac, setDeviceMac] = useState<string | null>(null)
  const [wifiState, setWifiState] = useState<WifiState | null>(null)
  const [provisioningStep, setProvisioningStep] = useState<string | null>(null)

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
    setDeviceModel(null)
    setDeviceMac(null)

    try {
      const selected = await navigator.bluetooth.requestDevice({
        filters: [{ services: [BLE_PROV_SERVICE_UUID] }],
        optionalServices: [BLE_PROV_SERVICE_UUID],
      })

      if (selected.name) {
        const parsed = parseBleDeviceName(selected.name)
        if (parsed) {
          setDeviceModel(parsed.model)
          setDeviceMac(parsed.macSuffix)
        }
      }

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

  // --- connect and provision (pure BLE) ---

  const connectAndProvision = useCallback(
    async (
      bleDevice: BluetoothDevice,
      ssid: string,
      password: string,
      mqttConfig: BleMqttConfig,
    ): Promise<boolean> => {
      setConnecting(true)
      setError(null)
      setWifiState(null)

      try {
        // 1. GATT connect
        setProvisioningStep('connecting')
        const gatt = bleDevice.gatt!
        if (gatt.connected) {
          try { gatt.disconnect() } catch { /* ignore */ }
          await sleep(500)
        }
        const server = await withTimeout(gatt.connect(), 30_000, 'GATT connect') as BluetoothRemoteGATTServer
        gattRef.current = server

        // 2. Get provisioning service + characteristics
        const service = await withTimeout(
          server.getPrimaryService(BLE_PROV_SERVICE_UUID),
          10_000, 'Get prov service',
        )
        const configChar = await service.getCharacteristic(BLE_CHAR_CONFIG)
        configCharRef.current = configChar

        setConnecting(false)
        setProvisioning(true)

        // 3. Write MQTT config to custom endpoint (write+read for verification)
        setProvisioningStep('writingMqtt')
        const mqttChar = await service.getCharacteristic(BLE_CHAR_MQTT)
        const mqttData = encodeMqttConfig(mqttConfig)
        console.log(`[BLE] Writing MQTT config: ${mqttData.byteLength} bytes`)
        await writeWithMtu(mqttChar, mqttData)
        // Read back response — firmware returns {"status":0} on success
        await sleep(300)
        const mqttResp = await mqttChar.readValue()
        const mqttRespText = new TextDecoder().decode(mqttResp)
        console.log(`[BLE] MQTT config response: ${mqttRespText}`)
        if (mqttResp.byteLength === 0) {
          throw new Error('MQTT config: device returned empty response')
        }
        if (!mqttRespText.includes('"status":0')) {
          throw new Error(`MQTT config rejected by device: ${mqttRespText}`)
        }

        // 4. Write WiFi credentials
        setProvisioningStep('writingWifi')
        const setConfigData = encodeSetConfig(ssid, password)
        const setResp = await writeAndRead(configChar, setConfigData, 'Set WiFi config')
        const setStatus = decodeRespSetConfig(viewToBytes(setResp))
        if (setStatus !== 0) {
          throw new Error(`SetConfig failed with status ${setStatus}`)
        }

        // 5. Poll connection status (max 30s)
        setProvisioningStep('polling')
        setWifiState(WifiState.Connecting)
        for (let i = 0; i < 30; i++) {
          await sleep(1000)
          try {
            const statusResp = await writeAndRead(
              configChar, encodeGetStatus(), 'Get status',
            )
            const { wifiState: ws } = decodeRespGetStatus(viewToBytes(statusResp))
            setWifiState(ws)

            if (ws === WifiState.Connected) {
              setProvisioningStep('done')
              return true
            }
            if (ws === WifiState.ConnectionFailed) {
              throw new Error('WiFi connection failed — check SSID and password')
            }
          } catch (pollErr) {
            if (pollErr instanceof Error && pollErr.message.includes('timed out')) continue
            // GATT disconnected during polling — device rebooting for WiFi
            if (pollErr instanceof Error && pollErr.message.includes('disconnected')) {
              console.log('[BLE] GATT disconnected during polling (device likely connecting WiFi)')
              setProvisioningStep('done')
              return true
            }
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
        setProvisioningStep(null)
        setConnecting(false)
      }
    },
    [],
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
    setDeviceModel(null)
    setDeviceMac(null)
    setWifiState(null)
    setError(null)
    setProvisioningStep(null)
  }, [])

  return {
    scanning, connecting, provisioning, disconnected, error, device,
    deviceModel, deviceMac, wifiState, provisioningStep,
    scan, connectAndProvision, disconnect, clearError,
  }
}
