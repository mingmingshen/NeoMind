// BLE Provisioning Protocol — ESP-IDF wifi_prov_mgr (Security0)
//
// Uses ESP-IDF standard provisioning over BLE with protobuf encoding.
// Security0 = no encryption, so we can encode protobuf manually without
// pulling in protobufjs.

// ---------------------------------------------------------------------------
// Service & Characteristic UUIDs (ESP-IDF standard)
// ---------------------------------------------------------------------------

/** ESP-IDF BLE provisioning service UUID */
export const BLE_PROV_SERVICE_UUID = '0000ffff-0000-1000-8000-00805f9b34fb'

/** Protocol version */
export const BLE_CHAR_PROTO_VER = '0000ff01-0000-1000-8000-00805f9b34fb'
/** Session endpoint (Security0 handshake) */
export const BLE_CHAR_SESSION = '0000ff02-0000-1000-8000-00805f9b34fb'
/** Config endpoint (WiFi credentials + status) */
export const BLE_CHAR_CONFIG = '0000ff03-0000-1000-8000-00805f9b34fb'

// ---------------------------------------------------------------------------
// Enum values (from ESP-IDF proto definitions)
// ---------------------------------------------------------------------------

export const enum WifiState {
  Connected = 0,
  Connecting = 1,
  Disconnected = 2,
  ConnectionFailed = 3,
}

export const enum ProvStatus {
  Success = 0,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface BleProvisionResult {
  success: boolean
  wifiState: WifiState
  error?: string
}

/** MQTT configuration returned by the pre-register API */
export interface BleMqttConfig {
  host: string
  port: number
  username: string
  password: string
  topic_prefix: string
}

/** Maps BLE device model strings to NeoMind device type identifiers */
export const MODEL_TO_DEVICE_TYPE: Record<string, string> = {
  NE101: 'ne101_camera',
  NE301: 'ne301_camera',
}

// ---------------------------------------------------------------------------
// Minimal protobuf encoder/decoder
// ---------------------------------------------------------------------------

function encodeVarint(value: number): Uint8Array {
  const bytes: number[] = []
  do {
    let b = value & 0x7f
    value >>>= 7
    if (value > 0) b |= 0x80
    bytes.push(b)
  } while (value > 0)
  return new Uint8Array(bytes)
}

function decodeVarint(data: Uint8Array, offset: number): [number, number] {
  let value = 0, shift = 0
  while (offset < data.length) {
    const b = data[offset++]
    value |= (b & 0x7f) << shift
    if (!(b & 0x80)) break
    shift += 7
  }
  return [value, offset]
}

/** Encode a protobuf string field (field_number, UTF-8 value) */
function encodeStringField(field: number, value: string): Uint8Array {
  const payload = new TextEncoder().encode(value)
  const tag = encodeVarint((field << 3) | 2) // wire type 2 = length-delimited
  const len = encodeVarint(payload.length)
  const out = new Uint8Array(tag.length + len.length + payload.length)
  let off = 0
  out.set(tag, off); off += tag.length
  out.set(len, off); off += len.length
  out.set(payload, off)
  return out
}

/**
 * Encode CmdSetConfig { string ssid = 1; string passphrase = 2; }
 */
export function encodeSetConfig(ssid: string, passphrase: string): Uint8Array {
  const a = encodeStringField(1, ssid)
  const b = encodeStringField(2, passphrase)
  const out = new Uint8Array(a.length + b.length)
  out.set(a, 0)
  out.set(b, a.length)
  return out
}

/**
 * Encode CmdGetStatus {} (empty message)
 */
export function encodeGetStatus(): Uint8Array {
  return new Uint8Array(0)
}

/**
 * Decode RespGetStatus { Status status = 1; WifiStationState wifi_state = 10; }
 */
export function decodeRespGetStatus(data: Uint8Array): {
  status: number
  wifiState: number
  failedReason: number
} {
  let status = 0, wifiState = 0, failedReason = 0, offset = 0
  while (offset < data.length) {
    const [tag, off1] = decodeVarint(data, offset)
    offset = off1
    const fieldNum = tag >> 3
    const wireType = tag & 7
    if (wireType === 0) {
      // varint (enum/int)
      const [val, off2] = decodeVarint(data, offset)
      offset = off2
      if (fieldNum === 1) status = val
      else if (fieldNum === 10) wifiState = val
      else if (fieldNum === 11) failedReason = val
    } else if (wireType === 2) {
      // length-delimited: skip
      const [len, off2] = decodeVarint(data, offset)
      offset = off2 + len
    } else {
      break
    }
  }
  return { status, wifiState, failedReason }
}

/**
 * Decode RespSetConfig { Status status = 1; }
 */
export function decodeRespSetConfig(data: Uint8Array): number {
  if (data.length === 0) return 0
  const [tag, off1] = decodeVarint(data, 0)
  if ((tag >> 3) === 1) {
    const [val] = decodeVarint(data, off1)
    return val
  }
  return -1
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const decoder = new TextDecoder()

/** Decode DataView to string */
export function decodeText(value: DataView): string {
  return decoder.decode(value)
}

/** Convert DataView to Uint8Array for protobuf decoding */
export function viewToBytes(value: DataView): Uint8Array {
  return new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
}
