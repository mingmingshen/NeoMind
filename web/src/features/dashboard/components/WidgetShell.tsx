/**
 * WidgetShell — wraps every widget with error boundary, loading, and chrome
 */

import React, { Component, Suspense } from 'react'
import { GripVertical, Settings } from 'lucide-react'
import { cn } from '@/lib/utils'
import { WidgetErrorFallback } from './WidgetErrorFallback'
import { WidgetSkeleton } from './WidgetSkeleton'

// ============================================================================
// Error Boundary (class component)
// ============================================================================

interface ErrorBoundaryProps {
  children: React.ReactNode
  fallback: React.ReactNode
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

class WidgetErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, error: null }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error }
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      return (
        <WidgetErrorFallback
          error={this.state.error ?? undefined}
          onRetry={this.handleRetry}
        />
      )
    }
    return this.props.children
  }
}

// ============================================================================
// WidgetShell
// ============================================================================

export interface WidgetShellProps {
  widgetId: string
  title?: string
  isEditing: boolean
  onOpenConfig?: () => void
  children: React.ReactNode
}

export function WidgetShell({
  widgetId,
  title,
  isEditing,
  onOpenConfig,
  children,
}: WidgetShellProps) {
  return (
    <div
      className={cn(
        'relative h-full bg-card rounded-lg border border-border overflow-hidden',
        'flex flex-col',
        isEditing && 'ring-1 ring-primary/20',
      )}
    >
      {/* Header with drag handle and config button */}
      {(title || isEditing) && (
        <div className="flex items-center gap-1 px-2 py-1 min-h-[28px] border-b border-border/50">
          {isEditing && (
            <div className="widget-drag-handle cursor-grab active:cursor-grabbing p-0.5 -ml-1 text-muted-foreground hover:text-foreground">
              <GripVertical className="h-3.5 w-3.5" />
            </div>
          )}
          {title && (
            <span className="text-xs font-medium text-muted-foreground truncate flex-1">
              {title}
            </span>
          )}
          {isEditing && onOpenConfig && (
            <button
              onClick={onOpenConfig}
              className="p-0.5 text-muted-foreground hover:text-foreground rounded-sm hover:bg-accent/50 transition-colors"
              aria-label="Widget settings"
            >
              <Settings className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      )}

      {/* Content area with error boundary + suspense */}
      <div className="flex-1 min-h-0 overflow-hidden">
        <WidgetErrorBoundary fallback={<WidgetErrorFallback />}>
          <Suspense fallback={<WidgetSkeleton />}>
            {children}
          </Suspense>
        </WidgetErrorBoundary>
      </div>
    </div>
  )
}
