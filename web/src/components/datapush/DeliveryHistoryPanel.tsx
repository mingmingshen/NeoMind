/**
 * DeliveryHistoryPanel - shows delivery logs for a push target.
 */

import { useEffect } from 'react'
import { useStore } from '@/store'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { DeliveryStatus } from '@/types'

interface DeliveryHistoryPanelProps {
  targetId: string
  onClose: () => void
}

const statusColors: Record<DeliveryStatus, string> = {
  success: 'text-success',
  failed: 'text-error',
  pending: 'text-warning',
  retrying: 'text-warning',
}

export function DeliveryHistoryPanel({ targetId, onClose }: DeliveryHistoryPanelProps) {
  const { t } = useTranslation()
  const { deliveryLogs, deliveryLogsLoading, fetchDeliveryLogs } = useStore()

  useEffect(() => {
    fetchDeliveryLogs(targetId, 50)
  }, [targetId, fetchDeliveryLogs])

  return (
    <div className="border border-border rounded-lg bg-card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h4 className="text-sm font-medium">
          {t('common:dataPush.deliveryHistory', 'Delivery History')}
        </h4>
        <button onClick={onClose} className="p-1 rounded-md hover:bg-accent text-muted-foreground">
          <X className="h-4 w-4" />
        </button>
      </div>

      <div className="max-h-80 overflow-y-auto">
        {deliveryLogsLoading && (
          <div className="flex items-center justify-center py-8 text-muted-foreground text-sm">
            <div className="animate-spin mr-2 h-4 w-4 border-2 border-current border-t-transparent rounded-full" />
            {t('common:loading', 'Loading...')}
          </div>
        )}

        {!deliveryLogsLoading && deliveryLogs.length === 0 && (
          <div className="text-center py-8 text-muted-foreground text-sm">
            {t('common:dataPush.noLogs', 'No delivery logs')}
          </div>
        )}

        {deliveryLogs.map((log) => (
          <div
            key={log.id}
            className="flex items-center justify-between px-4 py-2 border-b border-border last:border-b-0 text-xs"
          >
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className={`font-medium ${statusColors[log.status]}`}>
                  {log.status.toUpperCase()}
                </span>
                <span className="text-muted-foreground truncate">{log.data_source_id}</span>
              </div>
              {log.error && (
                <div className="text-error truncate mt-0.5">{log.error}</div>
              )}
            </div>
            <div className="text-muted-foreground flex-shrink-0 ml-4">
              <span>{t('common:dataPush.attempts', { defaultValue: '{{count}} attempts', count: log.attempts })}</span>
              <span className="ml-2">
                {new Date(log.created_at * 1000).toLocaleString()}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
