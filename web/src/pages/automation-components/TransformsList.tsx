/**
 * Transforms List - Using ResponsiveTable for consistent styling
 */

import { useState } from "react"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu"
import { ResponsiveTable } from "@/components/shared"
import { Edit, Trash2, Code, Database, Globe, Cpu, HardDrive, CheckCircle2, Download, MoreVertical } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import { textMini } from "@/design-system/tokens/typography"
import type { TransformAutomation } from "@/types"
import { useIsMobile } from "@/hooks/useMobile"

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
  global: { label: 'automation:transformBuilder.scopes.global', icon: Globe, color: 'text-accent-purple bg-accent-purple-light border-accent-purple-light' },
  device_type: { label: 'automation:transformBuilder.scopes.deviceType', icon: Cpu, color: 'text-info bg-info-light border-info' },
  device: { label: 'automation:transformBuilder.scopes.device', icon: HardDrive, color: 'text-success bg-success-light border-success-light dark:text-success dark:bg-success-light dark:border-success-light' },
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
  const isMobile = useIsMobile()
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
    isMobile ? (
      <div className="space-y-2">
        {paginatedTransforms.map((transform) => {
          const scopeInfo = getScopeInfo(transform.scope)
          const ScopeIcon = scopeInfo.icon

          return (
            <Card
              key={transform.id}
              className={cn(
                "overflow-hidden border-border shadow-sm cursor-pointer active:scale-[0.99] transition-all",
                !transform.enabled && "opacity-50"
              )}
              onClick={() => onEdit(transform)}
            >
              <div className="px-3 py-2.5">
                {/* Row 1: icon + name + switch + actions */}
                <div className="flex items-center gap-2.5">
                  <div className={cn(
                    "w-8 h-8 rounded-lg flex items-center justify-center shrink-0",
                    transform.enabled ? "bg-accent-cyan-light text-accent-cyan" : "bg-muted text-muted-foreground"
                  )}>
                    <Code className="h-4 w-4" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm truncate">{transform.name}</div>
                  </div>
                  <Switch
                    checked={transform.enabled}
                    onCheckedChange={() => onToggleStatus(transform)}
                    className="scale-75"
                    onClick={(e) => e.stopPropagation()}
                  />
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                      <button className="p-1 rounded-md hover:bg-muted">
                        <MoreVertical className="h-4 w-4 text-muted-foreground" />
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={(e) => { e.stopPropagation(); onEdit(transform) }}>
                        <Edit className="h-4 w-4 mr-2" />
                        {t('common:edit')}
                      </DropdownMenuItem>
                      {onExport && (
                        <DropdownMenuItem onClick={(e) => { e.stopPropagation(); onExport(transform) }}>
                          <Download className="h-4 w-4 mr-2" />
                          {t('common:export')}
                        </DropdownMenuItem>
                      )}
                      <DropdownMenuItem
                        className="text-error"
                        onClick={(e) => { e.stopPropagation(); onDelete(transform) }}
                      >
                        <Trash2 className="h-4 w-4 mr-2" />
                        {t('common:delete')}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                {/* Row 2: scope badge + output prefix */}
                <div className="flex items-center gap-1.5 mt-1.5 ml-[42px]">
                  <Badge variant="outline" className={cn(textMini, "h-5 px-1.5 gap-0.5", scopeInfo.color)}>
                    <ScopeIcon className="h-3 w-3" />
                    {getScopeLabel(transform.scope)}
                  </Badge>
                  <code className={cn(textMini, "text-muted-foreground truncate")}>
                    {(transform.output_prefix || 'transform') + '.'}
                  </code>
                </div>
              </div>
            </Card>
          )
        })}
      </div>
    ) : (
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
                  transform.enabled ? "bg-accent-cyan-light text-accent-cyan" : "bg-muted text-muted-foreground"
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
                <ScopeIcon className="h-4 w-4" />
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
                <Database className="h-4 w-4 text-muted-foreground" />
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
                    ? "bg-success-light text-success border-success"
                    : "text-foreground bg-muted border-border"
                )}>
                  <CheckCircle2 className="h-4 w-4" />
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
  )
}
