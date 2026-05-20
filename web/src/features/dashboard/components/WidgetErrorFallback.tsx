/**
 * WidgetErrorFallback — error display for crashed widgets
 */

import { AlertTriangle, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface WidgetErrorFallbackProps {
  error?: Error
  onRetry?: () => void
}

export function WidgetErrorFallback({ error, onRetry }: WidgetErrorFallbackProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-2 p-4 text-center">
      <AlertTriangle className="h-6 w-6 text-destructive" />
      <p className="text-sm font-medium text-foreground">Widget Error</p>
      {error?.message && (
        <p className="text-xs text-muted-foreground max-w-[200px] truncate">{error.message}</p>
      )}
      {onRetry && (
        <Button variant="ghost" size="sm" onClick={onRetry} className="mt-1">
          <RefreshCw className="h-3 w-3 mr-1" />
          Retry
        </Button>
      )}
    </div>
  )
}
