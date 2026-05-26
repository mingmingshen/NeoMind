// Check if a string is a base64-encoded image
export function isBase64Image(value: unknown): boolean {
  if (typeof value !== "string") return false
  const str = value.trim()

  // Check for data URL prefix
  if (str.startsWith("data:image/")) return true

  // Check for base64 pattern that looks like an image
  const imageSignatures = [
    "iVBORw0KGgo",  // PNG
    "/9j/",          // JPEG
    "R0lGODlh",      // GIF
    "UklGR",         // WebP
    "Qk",            // BMP
  ]

  // Base64 string should be reasonably long and have valid padding
  if (str.length < 100) return false
  if (!/^[A-Za-z0-9+/=]+$/.test(str)) return false

  // Check for known image signatures
  return imageSignatures.some(sig => str.startsWith(sig))
}

// Strip whitespace/newlines from base64 data (browsers can't decode base64 with line breaks)
function sanitizeBase64(data: string): string {
  return data.replace(/[\s\r\n]+/g, '')
}

// Validate that a string is valid base64 (quick check via atob)
function isValidBase64(data: string): boolean {
  if (!data || data.length === 0) return false
  try {
    // Only decode a small portion for performance on large images
    atob(data.length > 200 ? data.slice(0, 100) + data.slice(-100) : data)
    return true
  } catch {
    return false
  }
}

// Get the data URL for a base64 image value.
// Handles: raw base64, data:image/ URLs, and malformed data URLs.
export function getImageDataUrl(value: unknown): string | null {
  if (typeof value !== "string") return null
  const str = value.trim()

  // Already a data:image URL
  if (str.startsWith("data:image/")) {
    const commaIdx = str.indexOf(',')
    if (commaIdx === -1) return str
    const prefix = str.slice(0, commaIdx + 1)
    let b64 = sanitizeBase64(str.slice(commaIdx + 1))

    // Detect if the "base64" portion is actually another data URL (double-prefix bug)
    if (b64.startsWith('data:image/') || b64.startsWith('data:')) {
      // Recursively unwrap: the inner data URL is the real one
      return getImageDataUrl(b64)
    }

    if (!isValidBase64(b64)) return null
    return `${prefix}${b64}`
  }

  // Any other data: prefix (malformed)
  if (str.startsWith('data:')) {
    return getImageDataUrl(str.replace(/^data:[^,]*,/, ''))
  }

  // Sanitize base64 data (backend may include newlines)
  const clean = sanitizeBase64(str)
  if (!isValidBase64(clean)) return null

  // Detect image type from signature and add data URL prefix
  if (clean.startsWith("iVBORw0KGgo")) return `data:image/png;base64,${clean}`
  if (clean.startsWith("/9j/")) return `data:image/jpeg;base64,${clean}`
  if (clean.startsWith("R0lGODlh")) return `data:image/gif;base64,${clean}`
  if (clean.startsWith("UklGR")) return `data:image/webp;base64,${clean}`
  if (clean.startsWith("Qk")) return `data:image/bmp;base64,${clean}`

  // Fallback - try as PNG
  return `data:image/png;base64,${clean}`
}

// Format metric value for display
export function formatMetricValue(value: unknown, dataType?: string, t?: (key: string) => string): string {
  if (value === null || value === undefined) return "-"
  if (typeof value === "boolean") return value ? (t?.('devices.yes') || "是") : (t?.('devices.no') || "否")
  if (typeof value === "number") {
    if (dataType === "integer") {
      return value.toLocaleString("zh-CN")
    }
    return parseFloat(value.toFixed(2)).toString()
  }
  // For base64 images, show a placeholder text
  if (isBase64Image(value)) return t?.('devices.image') || "[图片]"
  return String(value)
}

// Check if value is a base64 image for rendering
export function isImageValue(value: unknown): boolean {
  return typeof value === "string" && isBase64Image(value)
}

// Generate device type ID from name
export function generateDeviceTypeId(name: string): string {
  return name.toLowerCase()
    .replace(/\s+/g, "_")
    .replace(/[^a-z0-9_]/g, "")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "")
}
