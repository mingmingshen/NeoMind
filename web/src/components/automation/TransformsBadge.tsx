import { useState, useEffect, useRef } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Database } from 'lucide-react'
import { api } from '@/lib/api'
import { fetchCache } from '@/lib/utils/async'
import { useErrorHandler } from '@/hooks/useErrorHandler'
import { DeviceTransformsDialog } from './DeviceTransformsDialog'

interface TransformsBadgeProps {
  deviceId?: string
  deviceTypeId?: string
  onRefresh?: () => void
}

export function TransformsBadge({ deviceId, deviceTypeId, onRefresh }: TransformsBadgeProps) {
  const { handleError } = useErrorHandler()
  const [count, setCount] = useState(0)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [loading, setLoading] = useState(false)
  const mountedRef = useRef(true)

  const fetchTransformCount = async () => {
    // Use shared cache key — all TransformsBadge instances share one endpoint
    const cacheKey = 'transforms-list'
    if (!fetchCache.shouldFetch(cacheKey)) return

    fetchCache.markFetching(cacheKey)
    setLoading(true)
    try {
      const result = await api.listTransforms()
      if (!mountedRef.current) return

      let filtered = result.transforms || []

      if (deviceId) {
        filtered = filtered.filter((tr) =>
          typeof tr.scope === 'object' && 'device' in tr.scope && tr.scope.device === deviceId
        )
      } else if (deviceTypeId) {
        filtered = filtered.filter((tr) =>
          typeof tr.scope === 'object' && 'device_type' in tr.scope && tr.scope.device_type === deviceTypeId
        )
      }

      setCount(filtered.length)
      fetchCache.markFetched(cacheKey)
    } catch (error) {
      fetchCache.invalidate('transforms-list')
      handleError(error, { operation: 'Fetch transform count', showToast: false })
    } finally {
      if (mountedRef.current) setLoading(false)
    }
  }

  useEffect(() => {
    mountedRef.current = true
    fetchTransformCount()
    return () => { mountedRef.current = false }
  }, [deviceId, deviceTypeId])

  const handleRefresh = () => {
    fetchCache.invalidate('transforms-list')
    fetchTransformCount()
    onRefresh?.()
  }

  if (loading) {
    return <Badge variant="outline" className="text-xs">...</Badge>
  }

  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        className="h-7 px-2"
        onClick={() => setDialogOpen(true)}
      >
        <Database className="h-4 w-4 mr-1 text-accent-purple" />
        <Badge variant="outline" className="text-xs">
          {count}
        </Badge>
      </Button>

      {/* Only mount the dialog when open to avoid N×3 API calls on page load */}
      {dialogOpen && (
        <DeviceTransformsDialog
          open={dialogOpen}
          onOpenChange={(open) => {
            setDialogOpen(open)
            if (!open) handleRefresh()
          }}
          deviceId={deviceId}
          deviceTypeId={deviceTypeId}
          onTransformCreated={handleRefresh}
        />
      )}
    </>
  )
}
