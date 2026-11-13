import { describe, expect, it } from 'vitest'
import type { ParameterDefinition } from '@/types/device'
import {
  prepareCommandParams,
  validateCommandParams,
} from '../commandParams'
import { seedCommandDefaults } from '../seedCommandDefaults'
import {
  getEnumOptions,
  isArrayDataType,
  mapJsonSchemaType,
  normalizeDataType,
  unwrapMetricValue,
} from '../parameterOptions'

describe('parameterOptions', () => {
  it('unwraps tagged MetricValue objects', () => {
    expect(unwrapMetricValue({ String: 'white' })).toBe('white')
    expect(unwrapMetricValue({ string: 'white' })).toBe('white')
    expect(unwrapMetricValue(42)).toBe(42)
  })

  it('reads both enum wire shapes', () => {
    expect(getEnumOptions({ enum: ['a', 'b'] })).toEqual(['a', 'b'])
    expect(getEnumOptions({ enum: { options: ['a', 'b'] } })).toEqual(['a', 'b'])
  })

  it('detects array object form', () => {
    expect(isArrayDataType('array')).toBe(true)
    expect(isArrayDataType({ array: { element_type: 'string' } } as never)).toBe(
      true,
    )
    expect(isArrayDataType('string')).toBe(false)
  })

  it('normalizes TitleCase and JSON Schema number', () => {
    expect(normalizeDataType('Integer')).toBe('integer')
    expect(normalizeDataType('Float')).toBe('float')
    expect(mapJsonSchemaType('number')).toBe('float')
    expect(normalizeDataType({ Enum: { options: ['a', 'b'] } })).toEqual({
      enum: { options: ['a', 'b'] },
    })
    expect(normalizeDataType({ Array: { element_type: 'String' } })).toEqual({
      array: { element_type: 'string' },
    })
  })
})

describe('seedCommandDefaults', () => {
  it('seeds numeric min instead of 0', () => {
    const params: ParameterDefinition[] = [
      {
        name: 'duration_sec',
        data_type: 'integer',
        min: 1,
        max: 3600,
        required: true,
      },
    ]
    expect(seedCommandDefaults(params)).toEqual({ duration_sec: 1 })
  })

  it('unwraps tagged defaults and leaves unset numerics without min', () => {
    const params: ParameterDefinition[] = [
      {
        name: 'color',
        data_type: { enum: { options: ['white', 'black'] } },
        default_value: { String: 'white' } as never,
      },
      { name: 'count', data_type: 'integer', required: true },
    ]
    expect(seedCommandDefaults(params)).toEqual({ color: 'white' })
  })
})

describe('validateCommandParams / prepareCommandParams', () => {
  const sleepParams: ParameterDefinition[] = [
    {
      name: 'duration_sec',
      display_name: 'Sleep Duration',
      data_type: 'integer',
      min: 1,
      max: 3600,
      required: true,
    },
  ]

  it('rejects missing required', () => {
    const r = validateCommandParams(sleepParams, {})
    expect(r.ok).toBe(false)
  })

  it('rejects below min', () => {
    const r = validateCommandParams(sleepParams, { duration_sec: 0 })
    expect(r.ok).toBe(false)
  })

  it('accepts valid number', () => {
    expect(validateCommandParams(sleepParams, { duration_sec: 60 }).ok).toBe(
      true,
    )
  })

  it('omits empty optional strings on prepare', () => {
    const params: ParameterDefinition[] = [
      { name: 'note', data_type: 'string', required: false },
      { name: 'mode', data_type: 'string', required: true },
    ]
    expect(
      prepareCommandParams(params, { note: '', mode: 'always', extra: null }),
    ).toEqual({ mode: 'always' })
  })

  it('rejects half-parsed array strings', () => {
    const params: ParameterDefinition[] = [
      { name: 'items', data_type: 'array', required: true },
    ]
    const r = validateCommandParams(params, { items: '[1, 2' })
    expect(r.ok).toBe(false)
  })
})
