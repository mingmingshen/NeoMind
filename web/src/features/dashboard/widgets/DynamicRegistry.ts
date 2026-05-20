/**
 * DynamicRegistry — Thin facade over the existing DynamicComponentRegistry.
 *
 * Re-exports the singleton from the old registry module so the new dashboard feature
 * can reference it through a stable import path. When the old code is removed during
 * Phase 5 cutover, the implementation can be inlined here.
 */

export {
  dynamicRegistry,
  DynamicComponentRegistry,
  dtoToComponentMeta,
} from '@/components/dashboard/registry/DynamicRegistry'
