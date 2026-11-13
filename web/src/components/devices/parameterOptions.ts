/**
 * Helpers for command-parameter option lists.
 *
 * Backend MetricDataType::Enum serializes as `{ enum: { options: string[] } }`.
 * The UI historically expected `{ enum: string[] }` (and plain `allowed_values`
 * string arrays). Treating the object form as an array caused
 * `o.map is not a function` when opening a command dialog.
 */

import type { ParameterDataType, ParameterDefinition } from '@/types/device'

/** Unwrap tagged MetricValue objects like `{ String: "white" }` / `{ string: "white" }`. */
export function unwrapMetricValue(v: unknown): unknown {
  if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
    const obj = v as Record<string, unknown>
    const keys = Object.keys(obj)
    if (keys.length === 1) {
      const k = keys[0]
      if (
        /^(String|string|Integer|integer|Float|float|Boolean|boolean|Null|null)$/.test(
          k,
        )
      ) {
        return obj[k]
      }
    }
  }
  return v
}

/**
 * Resolve enum option strings from either frontend shape `{ enum: string[] }`
 * or backend shape `{ enum: { options: string[] } }`.
 */
export function getEnumOptions(
  dataType: ParameterDataType | undefined | null,
): string[] | null {
  if (!dataType || typeof dataType !== 'object' || !('enum' in dataType)) {
    return null
  }
  const e = (dataType as { enum: unknown }).enum
  if (Array.isArray(e) && e.every((x) => typeof x === 'string')) {
    return e as string[]
  }
  if (
    e !== null &&
    typeof e === 'object' &&
    Array.isArray((e as { options?: unknown }).options)
  ) {
    const opts = (e as { options: unknown[] }).options
    if (opts.every((x) => typeof x === 'string')) {
      return opts as string[]
    }
  }
  return null
}

/** Flatten allowed_values to plain JS values for Select / defaults. */
export function getAllowedValues(
  param: Pick<ParameterDefinition, 'allowed_values'>,
): unknown[] | null {
  if (!param.allowed_values || param.allowed_values.length === 0) {
    return null
  }
  return param.allowed_values.map(unwrapMetricValue)
}

/**
 * True for plain `"array"` or backend object form
 * `{ array: { element_type?: ... } }` / `{ Array: ... }`.
 */
export function isArrayDataType(
  dataType: ParameterDataType | undefined | null,
): boolean {
  if (dataType === 'array') return true
  if (!dataType || typeof dataType !== 'object') return false
  const keys = Object.keys(dataType)
  return keys.length === 1 && keys[0].toLowerCase() === 'array'
}
