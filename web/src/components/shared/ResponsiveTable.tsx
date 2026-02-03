/**
 * ResponsiveTable Component
 *
 * Desktop: Shows a standard table layout
 * Mobile: Shows cards with better readability
 */

import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { MoreVertical } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface TableColumn {
  key: string
  label: string | ReactNode
  width?: string
  align?: 'left' | 'center' | 'right'
  className?: string
}

export interface TableRowAction {
  label: string
  icon?: ReactNode
  onClick: (rowData?: Record<string, unknown>) => void
  variant?: 'default' | 'destructive'
  disabled?: boolean
  show?: (rowData: Record<string, unknown>) => boolean
}

export interface ResponsiveTableProps {
  columns: TableColumn[]
  data: Record<string, unknown>[]
  renderCell: (columnKey: string, rowData: Record<string, unknown>) => ReactNode
  rowKey: (rowData: Record<string, unknown>) => string
  actions?: TableRowAction[]
  onRowClick?: (rowData: Record<string, unknown>) => void
  className?: string
  loading?: boolean
  emptyState?: ReactNode
  getRowClassName?: (rowData: Record<string, unknown>) => string
}

/**
 * Render column label as ReactNode
 */
function renderColumnLabel(label: string | ReactNode): ReactNode {
  return label
}

/**
 * Responsive table component
 * Desktop: Standard table
 * Mobile: Card layout
 */
export function ResponsiveTable({
  columns,
  data,
  renderCell,
  rowKey,
  actions,
  onRowClick,
  className,
  loading,
  emptyState,
  getRowClassName,
}: ResponsiveTableProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-center">
          <div className="inline-block h-8 w-8 animate-spin rounded-full border-4 border-solid border-primary border-r-transparent mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">Loading...</p>
        </div>
      </div>
    )
  }

  if (data.length === 0) {
    return <>{emptyState}</>
  }

  return (
    <>
      {/* Desktop Table */}
      <div className="hidden md:block">
        <table className={cn("w-full caption-bottom text-sm", className)}>
          <thead className="[&_tr]:border-b">
            <tr className="border-b bg-muted/30 hover:bg-transparent">
              {columns.map((column) => (
                <th
                  key={column.key}
                  className={cn(
                    "h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground",
                    column.align === 'center' && 'text-center',
                    column.align === 'right' && 'text-right',
                    !column.align && 'text-left',
                    column.width
                  )}
                >
                  {renderColumnLabel(column.label)}
                </th>
              ))}
              {actions && actions.length > 0 && (
                <th className="w-12"></th>
              )}
            </tr>
          </thead>
          <tbody className="[&_tr:last-child]:border-0">
            {data.map((rowData) => {
              const rowClass = getRowClassName?.(rowData)
              const visibleActions = actions?.filter(a => a.show?.(rowData) !== false)

              return (
                <tr
                  key={rowKey(rowData)}
                  className={cn(
                    "border-b transition-colors hover:bg-muted/50 group",
                    onRowClick && 'cursor-pointer',
                    rowClass
                  )}
                  onClick={() => onRowClick?.(rowData)}
                >
                  {columns.map((column) => (
                    <td
                      key={column.key}
                      className={cn(
                        "p-4 align-middle",
                        column.align === 'center' && 'text-center',
                        column.align === 'right' && 'text-right',
                        !column.align && 'text-left',
                        column.className
                      )}
                    >
                      {renderCell(column.key, rowData)}
                    </td>
                  ))}
                  {visibleActions && visibleActions.length > 0 && (
                    <td className="p-4 align-middle">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                          <Button variant="ghost" size="icon" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          {visibleActions.map((action, idx) => (
                            <DropdownMenuItem
                              key={idx}
                              onClick={(e) => {
                                e.stopPropagation()
                                action.onClick(rowData)
                              }}
                              className={action.variant === 'destructive' ? 'text-destructive' : ''}
                              disabled={action.disabled}
                            >
                              {action.icon && <span className="mr-2 h-4 w-4">{action.icon}</span>}
                              {action.label}
                            </DropdownMenuItem>
                          ))}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  )}
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>

      {/* Mobile Card List */}
      <div className="md:hidden space-y-3">
        {data.map((rowData) => {
          const rowClass = getRowClassName?.(rowData)
          const visibleActions = actions?.filter(a => a.show?.(rowData) !== false)

          return (
            <Card
              key={rowKey(rowData)}
              className={cn(
                'overflow-hidden',
                onRowClick && 'cursor-pointer hover:bg-accent/50 transition-colors',
                rowClass
              )}
            >
              {/* Card Header - First column as title */}
              <div className="bg-muted/30 px-4 py-3 border-b">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    {renderCell(columns[0].key, rowData)}
                  </div>
                  {visibleActions && visibleActions.length > 0 && (
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                        <Button variant="ghost" size="icon" className="h-8 w-8 shrink-0">
                          <MoreVertical className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        {visibleActions.map((action, idx) => (
                          <DropdownMenuItem
                            key={idx}
                            onClick={(e) => {
                              e.stopPropagation()
                              action.onClick(rowData)
                            }}
                            className={action.variant === 'destructive' ? 'text-destructive' : ''}
                            disabled={action.disabled}
                          >
                            {action.icon && <span className="mr-2 h-4 w-4">{action.icon}</span>}
                            {action.label}
                          </DropdownMenuItem>
                        ))}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  )}
                </div>
              </div>

              {/* Card Body - Other columns as key-value pairs */}
              <div className="p-4 space-y-3">
                {columns.slice(1).map((column) => {
                  const cellContent = renderCell(column.key, rowData)
                  // Skip if content is empty
                  if (!cellContent || (typeof cellContent === 'object' && 'props' in cellContent && (cellContent as any).props.children === '')) {
                    return null
                  }

                  return (
                    <div key={column.key} className="flex items-start gap-3">
                      <span className="text-xs text-muted-foreground shrink-0 w-24 pt-0.5">
                        {renderColumnLabel(column.label)}
                      </span>
                      <div className="text-sm flex-1 text-left">
                        {cellContent}
                      </div>
                    </div>
                  )
                })}
              </div>
            </Card>
          )
        })}
      </div>
    </>
  )
}

/**
 * Convenience component for simple card layout on mobile
 */
export interface MobileCardProps {
  title: ReactNode
  subtitle?: ReactNode
  icon?: ReactNode
  actions?: ReactNode
  children: ReactNode
  className?: string
  onClick?: () => void
}

export function MobileCard({ title, subtitle, icon, actions, children, className, onClick }: MobileCardProps) {
  return (
    <Card className={cn('overflow-hidden', onClick && 'cursor-pointer hover:bg-accent/50 transition-colors', className)}>
      {/* Card Header */}
      <div className="bg-muted/30 px-4 py-3 border-b">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0 flex-1">
            {icon && <div className="shrink-0">{icon}</div>}
            <div className="min-w-0 flex-1">
              <div className="font-medium text-sm truncate">{title}</div>
              {subtitle && <div className="text-xs text-muted-foreground truncate">{subtitle}</div>}
            </div>
          </div>
          {actions && <div className="shrink-0">{actions}</div>}
        </div>
      </div>

      {/* Card Body */}
      <div className="p-4">
        {children}
      </div>
    </Card>
  )
}
