/**
 * Dashboard TanStack Query hooks — query key tree + CRUD hooks
 *
 * All dashboard query keys namespaced under ['dashboard', ...] to avoid
 * collision with existing queryKeys in @/lib/react-query-hooks.ts.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import * as dashboardApi from '../api/dashboards'
import type { Dashboard } from '../types'

// ============================================================================
// Query Key Tree
// ============================================================================

export const dashboardKeys = {
  all: ['dashboard'] as const,
  lists: () => [...dashboardKeys.all, 'list'] as const,
  detail: (id: string) => [...dashboardKeys.all, 'detail', id] as const,
  telemetry: (sourceId: string, windowType: string) =>
    [...dashboardKeys.all, 'telemetry', sourceId, windowType] as const,
  deviceMetrics: (deviceId: string) =>
    [...dashboardKeys.all, 'device', deviceId, 'metrics'] as const,
  deviceCurrent: (deviceId: string) =>
    [...dashboardKeys.all, 'device', deviceId, 'current'] as const,
  extensionMetrics: (extId: string) =>
    [...dashboardKeys.all, 'extension', extId, 'metrics'] as const,
  systemMetrics: () =>
    [...dashboardKeys.all, 'system'] as const,
  templates: () =>
    [...dashboardKeys.all, 'templates'] as const,
}

// ============================================================================
// Dashboard list query
// ============================================================================

export function useDashboardList() {
  return useQuery({
    queryKey: dashboardKeys.lists(),
    queryFn: dashboardApi.listDashboards,
    staleTime: 5 * 60 * 1000, // 5 minutes
  })
}

// ============================================================================
// Single dashboard query
// ============================================================================

export function useDashboardDetail(id: string | null) {
  return useQuery({
    queryKey: dashboardKeys.detail(id ?? ''),
    queryFn: () => dashboardApi.getDashboard(id!),
    enabled: !!id,
    staleTime: 2 * 60 * 1000, // 2 minutes
  })
}

// ============================================================================
// Templates query
// ============================================================================

export function useDashboardTemplates() {
  return useQuery({
    queryKey: dashboardKeys.templates(),
    queryFn: dashboardApi.listTemplates,
    staleTime: 10 * 60 * 1000, // 10 minutes
  })
}

// ============================================================================
// Mutation hooks
// ============================================================================

export function useCreateDashboard() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: dashboardApi.createDashboard,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dashboardKeys.lists() })
    },
  })
}

export function useUpdateDashboard() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: Partial<Dashboard> }) =>
      dashboardApi.updateDashboard(id, updates),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: dashboardKeys.detail(id) })
      queryClient.invalidateQueries({ queryKey: dashboardKeys.lists() })
    },
  })
}

export function useDeleteDashboard() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: dashboardApi.deleteDashboard,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dashboardKeys.lists() })
    },
  })
}

export function useShareDashboard() {
  return useMutation({
    mutationFn: dashboardApi.shareDashboard,
  })
}

export function useUnshareDashboard() {
  return useMutation({
    mutationFn: dashboardApi.unshareDashboard,
  })
}
