import { useState, useEffect } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Database } from 'lucide-react'
import { api } from '@/lib/api'
import { DeviceTransformsDialog } from './DeviceTransformsDialog'

interface TransformsBadgeProps {
  deviceId?: string
  deviceTypeId?: string
  onRefresh?: () => void
}

export function TransformsBadge({ deviceId, deviceTypeId, onRefresh }: TransformsBadgeProps) {
  const [count, setCount] = useState(0)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [loading, setLoading] = useState(false)

  const fetchTransformCount = async () => {
    setLoading(true)
    try {
      const result = await api.listTransforms()
      let filtered = result.transforms || []

      // Filter transforms by scope
      if (deviceId) {
        filtered = filtered.filter((tr) =>
          tr.scope.type === 'device' && tr.scope.device_id === deviceId
        )
      } else if (deviceTypeId) {
        filtered = filtered.filter((tr) =>
          tr.scope.type === 'device_type' && tr.scope.device_type === deviceTypeId
        )
      }

      setCount(filtered.length)
    } catch (error) {
      console.error('Failed to fetch transform count:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchTransformCount()
  }, [deviceId, deviceTypeId])

  const handleRefresh = () => {
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
        <Database className="h-3 w-3 mr-1 text-purple-500" />
        <Badge variant="outline" className="text-xs">
          {count}
        </Badge>
      </Button>

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
    </>
  )
}
