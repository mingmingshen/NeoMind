/**
 * Transforms List - Using ResponsiveTable for consistent styling
 */

import { useState } from "react"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import { ResponsiveTable } from "@/components/shared"
import { Edit, Trash2, Code, Database, Globe, Cpu, HardDrive, CheckCircle2, Download } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import type { TransformAutomation } from "@/types"

interface TransformsListProps {
  transforms: TransformAutomation[]
  loading: boolean
  paginatedTransforms?: TransformAutomation[]
  page?: number
  onPageChange?: (page: number) => void
  onEdit: (transform: TransformAutomation) => void
  onDelete: (transform: TransformAutomation) => void
  onToggleStatus: (transform: TransformAutomation) => void
  onExport?: (transform: TransformAutomation) => void
}

// Scope configuration
const SCOPE_CONFIG: Record<string, { label: string; icon: typeof Globe; color: string }> = {
  global: { label: 'automation:transformBuilder.scopes.global', icon: Globe, color: 'text-purple-700 bg-purple-50 border-purple-200 dark:text-purple-400 dark:bg-purple-950/30 dark:border-purple-800' },
  device_type: { label: 'automation:transformBuilder.scopes.deviceType', icon: Cpu, color: 'text-blue-700 bg-blue-50 border-blue-200 dark:text-blue-400 dark:bg-blue-950/30 dark:border-blue-800' },
  device: { label: 'automation:transformBuilder.scopes.device', icon: HardDrive, color: 'text-green-700 bg-green-50 border-green-200 dark:text-green-400 dark:bg-green-950/30 dark:border-green-800' },
}

export const ITEMS_PER_PAGE = 10

// Get code summary for display
function getCodeSummary(jsCode: string): string {
  if (!jsCode) return '-'

  // Try to extract return statement
  const returnMatch = jsCode.match(/return\s+({[^}]*}|\[[^\]]*\]|[^;{};]+)/s)
  if (returnMatch) {
    const ret = returnMatch[1].trim()
    if (ret.length > 45) {
      return ret.substring(0, 42) + '...'
    }
    return ret
  }

  // Show first non-comment line
  const lines = jsCode.split('\n').filter(l => l.trim() && !l.trim().startsWith('//'))
  if (lines.length > 0) {
    const firstLine = lines[0].trim()
    if (firstLine.length > 45) {
      return firstLine.substring(0, 42) + '...'
    }
    return firstLine
  }

  return jsCode.substring(0, 45) + (jsCode.length > 45 ? '...' : '')
}

export function TransformsList({
  transforms,
  loading,
  paginatedTransforms: propsPaginatedTransforms,
  page: propsPage,
  onPageChange,
  onEdit,
  onDelete,
  onToggleStatus,
  onExport,
}: TransformsListProps) {
  const { t } = useTranslation(['common', 'automation'])
  const [internalPage, setInternalPage] = useState(1)

  // Use props if provided, otherwise use internal state (backward compatibility)
  const page = propsPage ?? internalPage
  const setPage = onPageChange ?? setInternalPage

  const totalPages = Math.ceil(transforms.length / ITEMS_PER_PAGE) || 1
  const startIndex = (page - 1) * ITEMS_PER_PAGE
  const endIndex = startIndex + ITEMS_PER_PAGE
  const paginatedTransforms = propsPaginatedTransforms ?? transforms.slice(startIndex, endIndex)

  function getScopeInfo(scope: any): { label: string; icon: typeof Globe; color: string } {
    if (!scope || scope === 'global' || (typeof scope === 'string' && scope === 'global')) {
      return SCOPE_CONFIG.global
    }
    if (scope.device_type || (typeof scope === 'object' && scope.device_type)) {
      return SCOPE_CONFIG.device_type
    }
    if (scope.device || (typeof scope === 'object' && scope.device)) {
      return SCOPE_CONFIG.device
    }
    return SCOPE_CONFIG.global
  }

  function getScopeLabel(scope: any): string {
    if (!scope || scope === 'global' || (typeof scope === 'string' && scope === 'global')) {
      return t('automation:transformBuilder.scopes.global')
    }
    if (scope.device_type || (typeof scope === 'object' && scope.device_type)) {
      return `${t('automation:transformBuilder.scopes.deviceType')}: ${scope.device_type || scope.type}`
    }
    if (scope.device || (typeof scope === 'object' && scope.device)) {
      return `${t('automation:transformBuilder.scopes.device')}: ${scope.device}`
    }
    return t('automation:transformBuilder.scopes.global')
  }

  return (
    <ResponsiveTable
      columns={[
        {
          key: 'index',
          label: '#',
          width: 'w-10',
          align: 'center',
        },
        {
          key: 'name',
          label: t('automation:name'),
        },
        {
          key: 'scope',
          label: t('automation:scope'),
        },
        {
          key: 'transformCode',
          label: t('automation:transformBuilder.transformCode'),
        },
        {
          key: 'outputPrefix',
          label: t('automation:outputPrefix'),
        },
        {
          key: 'status',
          label: t('automation:status'),
          align: 'center',
        },
      ]}
      data={paginatedTransforms as unknown as Record<string, unknown>[]}
      rowKey={(transform) => (transform as unknown as TransformAutomation).id}
      loading={loading}
      getRowClassName={(rowData) => {
        const transform = rowData as unknown as TransformAutomation
        return cn(!transform.enabled && "opacity-50")
      }}
      renderCell={(columnKey, rowData) => {
        const transform = rowData as unknown as TransformAutomation
        const index = paginatedTransforms.indexOf(transform)
        const scopeInfo = getScopeInfo(transform.scope)
        const ScopeIcon = scopeInfo.icon

        switch (columnKey) {
          case 'index':
            return (
              <span className="text-xs text-muted-foreground font-medium">
                {startIndex + index + 1}
              </span>
            )

          case 'name':
            return (
              <div className="flex items-center gap-3">
                <div className={cn(
                  "w-9 h-9 rounded-lg flex items-center justify-center transition-colors",
                  transform.enabled ? "bg-cyan-500/10 text-cyan-600" : "bg-muted text-muted-foreground"
                )}>
                  <Code className="h-4 w-4" />
                </div>
                <div>
                  <div className="font-medium text-sm">{transform.name}</div>
                  <div className="text-xs text-muted-foreground line-clamp-1">
                    {transform.description || '-'}
                  </div>
                </div>
              </div>
            )

          case 'scope':
            return (
              <Badge variant="outline" className={cn("text-xs gap-1.5", scopeInfo.color)}>
                <ScopeIcon className="h-3 w-3" />
                {getScopeLabel(transform.scope)}
              </Badge>
            )

          case 'transformCode':
            return (
              <code className="text-xs bg-muted px-2 py-1 rounded-md font-mono truncate block max-w-[200px]">
                {getCodeSummary(transform.js_code || '')}
              </code>
            )

          case 'outputPrefix':
            return (
              <div className="flex items-center gap-1.5">
                <Database className="h-3 w-3 text-muted-foreground" />
                <code className="text-xs bg-muted px-2 py-0.5 rounded">
                  {(transform.output_prefix || 'transform') + '.'}
                </code>
              </div>
            )

          case 'status':
            return (
              <div className="flex items-center justify-start gap-2">
                <Switch
                  checked={transform.enabled}
                  onCheckedChange={() => onToggleStatus(transform)}
                  className="scale-90"
                />
                <Badge variant="outline" className={cn(
                  "text-xs gap-1 hidden sm:flex",
                  transform.enabled
                    ? "text-green-700 bg-green-50 border-green-200 dark:text-green-400 dark:bg-green-950/30 dark:border-green-800"
                    : "text-gray-700 bg-gray-50 border-gray-200 dark:text-gray-400 dark:bg-gray-800 dark:border-gray-700"
                )}>
                  <CheckCircle2 className="h-3 w-3" />
                  {transform.enabled ? t('automation:statusEnabled') : t('automation:statusDisabled')}
                </Badge>
              </div>
            )

          default:
            return null
        }
      }}
      actions={[
        {
          label: t('common:edit'),
          icon: <Edit className="h-4 w-4" />,
          onClick: (rowData) => {
            const transform = rowData as unknown as TransformAutomation
            onEdit(transform)
          },
        },
        ...(onExport ? [{
          label: t('common:export'),
          icon: <Download className="h-4 w-4" />,
          onClick: (rowData: unknown) => {
            const transform = rowData as unknown as TransformAutomation
            onExport(transform)
          },
        }] : []),
        {
          label: t('common:delete'),
          icon: <Trash2 className="h-4 w-4" />,
          variant: 'destructive',
          onClick: (rowData) => {
            const transform = rowData as unknown as TransformAutomation
            onDelete(transform)
          },
        },
      ]}
    />
  )
}
