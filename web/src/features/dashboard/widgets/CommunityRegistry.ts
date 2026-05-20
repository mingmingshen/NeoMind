/**
 * CommunityRegistry — Thin facade over the existing CommunityComponentRegistry.
 *
 * Re-exports the singleton from the old registry module so the new dashboard feature
 * can reference it through a stable import path. When the old code is removed during
 * Phase 5 cutover, the implementation can be inlined here.
 */

export {
  communityRegistry,
  CommunityComponentRegistry,
} from '@/components/dashboard/registry/CommunityRegistry'
