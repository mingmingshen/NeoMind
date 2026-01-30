// ConnectionStatus component - displays WebSocket connection state
import { cn } from '@/lib/utils'
import type { ConnectionState } from '@/lib/websocket'
import { useTranslation } from 'react-i18next'

interface ConnectionStatusProps {
  state: ConnectionState
  className?: string
}

export function ConnectionStatus({ state, className }: ConnectionStatusProps) {
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
          bgClass: 'bg-green-50 text-green-700 border-green-200'
        }
      case 'reconnecting':
        return {
          icon: (
            <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          ),
          text: t('connection.reconnecting'),
          bgClass: 'bg-yellow-50 text-yellow-700 border-yellow-200'
        }
      case 'error':
        return {
          icon: (
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          ),
          text: connectionState.errorMessage || t('connection.error'),
          bgClass: 'bg-red-50 text-red-700 border-red-200'
        }
      case 'disconnected':
        return {
          icon: (
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
            </svg>
          ),
          text: t('connection.disconnected'),
          bgClass: 'bg-gray-50 text-gray-700 border-gray-200'
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
      {state.status === 'reconnecting' && state.retryCount && state.retryCount > 1 && (
        <span className="text-xs opacity-75">
          ({t('connection.retryAttempt', { count: state.retryCount })})
        </span>
      )}
    </div>
  )
}
