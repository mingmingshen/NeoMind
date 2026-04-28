// ConnectionStatus component - displays WebSocket connection state
import { cn } from '@/lib/utils'
import type { ConnectionState } from '@/lib/websocket'
import { useTranslation } from 'react-i18next'
import { RotateCcw } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface ConnectionStatusProps {
  state: ConnectionState
  className?: string
  onManualReconnect?: () => void
}

export function ConnectionStatus({ state, className, onManualReconnect }: ConnectionStatusProps) {
  const { t } = useTranslation('chat')

  const getStatusInfo = (connectionState: ConnectionState) => {
    switch (connectionState.status) {
      case 'connected':
        return {
          icon: (
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          ),
          text: t('connection.connected'),
          bgClass: 'bg-success-light text-success border-success-light dark:bg-success-light dark:text-success dark:border-success-light'
        }
      case 'reconnecting':
        return {
          icon: (
            <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          ),
          text: t('connection.reconnecting'),
          bgClass: 'bg-warning-light text-warning border-warning'
        }
      case 'error':
        return {
          icon: (
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          ),
          text: connectionState.errorMessage || t('connection.error'),
          bgClass: 'bg-error-light text-error border-error'
        }
      case 'disconnected':
        return {
          icon: (
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
            </svg>
          ),
          text: t('connection.disconnected'),
          bgClass: 'bg-muted text-foreground border-border'
        }
    }
  }

  const info = getStatusInfo(state)

  return (
    <div className={cn(
      "connection-status flex items-center gap-2 px-3 py-2 rounded-lg text-sm border transition-colors",
      info.bgClass,
      className
    )}>
      {info.icon}
      <span>{info.text}</span>

      {/* Show retry count and countdown when reconnecting */}
      {state.status === 'reconnecting' && state.retryCount && state.retryCount > 0 && (
        <span className="text-xs opacity-75">
          (尝试 {state.retryCount}/10{state.nextRetryIn !== undefined && ` · ${state.nextRetryIn}s`})
        </span>
      )}

      {/* Show manual reconnect button when max retries reached */}
      {state.status === 'error' && onManualReconnect && (
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-xs gap-1 hover:bg-inherit"
          onClick={onManualReconnect}
        >
          <RotateCcw className="w-4 h-4" />
          重新连接
        </Button>
      )}
    </div>
  )
}
