import { useEffect, useState, useMemo, useRef } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, Check, Trash2, Eye, Bell } from "lucide-react"
import { EmptyStateInline, Pagination, AlertBadge, BulkActionBar, ActionBar } from "@/components/shared"
import { formatTimestamp } from "@/lib/utils/format"
import { api } from "@/lib/api"
import { useToast } from "@/hooks/use-toast"
import type { Alert } from "@/types"

type AlertFilter = 'all' | 'unacknowledged' | 'info' | 'warning' | 'critical'

export function AlertsTab() {
  const { t } = useTranslation(['common', 'alerts'])
  const { alerts, alertsLoading, fetchAlerts, acknowledgeAlert, createAlert } = useStore()
  const { toast } = useToast()

  // Filter state
  const [filter, setFilter] = useState<AlertFilter>('all')

  // Pagination state
  const [page, setPage] = useState(1)
  const alertsPerPage = 20

  // Bulk selection state
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [bulkProcessing, setBulkProcessing] = useState(false)

  // Create alert dialog state
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [newAlertTitle, setNewAlertTitle] = useState("")
  const [newAlertMessage, setNewAlertMessage] = useState("")
  const [newAlertSeverity, setNewAlertSeverity] = useState<"info" | "warning" | "critical">("info")
  const [creating, setCreating] = useState(false)
  const [acknowledgingId, setAcknowledgingId] = useState<string | null>(null)

  // Detail dialog state
  const [selectedAlert, setSelectedAlert] = useState<Alert | null>(null)

  // Fetch alerts on mount (once)
  const hasFetchedAlerts = useRef(false)
  useEffect(() => {
    if (!hasFetchedAlerts.current) {
      hasFetchedAlerts.current = true
      fetchAlerts()
    }
  }, [])

  // Reset pagination when alerts change
  useEffect(() => {
    setPage(1)
  }, [alerts.length])

  // Reset selection when filter changes
  useEffect(() => {
    setSelectedIds(new Set())
  }, [filter])

  // Filter alerts
  const filteredAlerts = useMemo(() => {
    if (filter === 'all') return alerts
    if (filter === 'unacknowledged') return alerts.filter(a => !a.acknowledged)
    return alerts.filter(a => a.severity === filter)
  }, [alerts, filter])

  // Paginated alerts
  const paginatedAlerts = filteredAlerts.slice(
    (page - 1) * alertsPerPage,
    page * alertsPerPage
  )

  const handleAcknowledge = async (id: string) => {
    setAcknowledgingId(id)
    try {
      await acknowledgeAlert(id)
      toast({ title: t('common:success'), description: t('alerts:acknowledged') })
    } finally {
      setAcknowledgingId(null)
    }
  }

  const handleCreateAlert = async () => {
    if (!newAlertTitle.trim() || !newAlertMessage.trim()) return

    setCreating(true)
    try {
      await createAlert({
        title: newAlertTitle,
        message: newAlertMessage,
        severity: newAlertSeverity,
        source: "manual",
      })
      toast({ title: t('common:success'), description: t('alerts:alertCreated') })
      setCreateDialogOpen(false)
      setNewAlertTitle("")
      setNewAlertMessage("")
      setNewAlertSeverity("info")
    } finally {
      setCreating(false)
    }
  }

  // Toggle selection
  const toggleSelection = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  // Toggle all on current page
  const toggleAll = () => {
    const pageIds = new Set(paginatedAlerts.map((a) => a.id))
    if (paginatedAlerts.every((a) => selectedIds.has(a.id))) {
      setSelectedIds((prev) => {
        const next = new Set(prev)
        pageIds.forEach((id) => next.delete(id))
        return next
      })
    } else {
      setSelectedIds((prev) => new Set([...prev, ...pageIds]))
    }
  }

  // Bulk actions
  const handleBulkAcknowledge = async () => {
    if (selectedIds.size === 0) return
    setBulkProcessing(true)
    try {
      const response = await api.bulkAcknowledgeAlerts(Array.from(selectedIds))
      if (response.acknowledged) {
        toast({ title: t('common:success'), description: t('alerts:acknowledgedCount', { count: response.acknowledged }) })
        setSelectedIds(new Set())
        await fetchAlerts()
      }
    } catch (error) {
      toast({ title: t('common:failed'), description: t('alerts:bulkAcknowledgeFailed'), variant: "destructive" })
    } finally {
      setBulkProcessing(false)
    }
  }

  const handleBulkDelete = async () => {
    if (selectedIds.size === 0) return
    if (!confirm(t('alerts:deleteConfirm', { count: selectedIds.size }))) return

    setBulkProcessing(true)
    try {
      const response = await api.bulkDeleteAlerts(Array.from(selectedIds))
      if (response.deleted) {
        toast({ title: t('common:success'), description: t('alerts:deletedCount', { count: response.deleted }) })
        setSelectedIds(new Set())
        await fetchAlerts()
      }
    } catch (error) {
      toast({ title: t('common:failed'), description: t('alerts:bulkDeleteFailed'), variant: "destructive" })
    } finally {
      setBulkProcessing(false)
    }
  }

  const allOnPageSelected = paginatedAlerts.length > 0 && paginatedAlerts.every((a) => selectedIds.has(a.id))

  const filters = [
    { value: 'all' as AlertFilter, label: t('alerts:all') },
    { value: 'unacknowledged' as AlertFilter, label: t('alerts:unacknowledged') },
    { value: 'info' as AlertFilter, label: t('alerts:info') },
    { value: 'warning' as AlertFilter, label: t('alerts:warning') },
    { value: 'critical' as AlertFilter, label: t('alerts:critical') },
  ]

  return (
    <div className="space-y-4">
      {/* Header */}
      <ActionBar
        title={t('automation:alerts')}
        titleIcon={<Bell className="h-5 w-5" />}
        description={t('automation:alertsDesc')}
        actions={[
          {
            label: t('alerts:createAlert'),
            icon: <Plus className="h-4 w-4" />,
            onClick: () => setCreateDialogOpen(true),
          },
        ]}
        onRefresh={fetchAlerts}
      />

      {/* Filter tabs */}
      <div className="flex gap-2">
        {filters.map((f) => (
          <Button
            key={f.value}
            variant={filter === f.value ? "default" : "outline"}
            size="sm"
            onClick={() => setFilter(f.value)}
          >
            {f.label}
          </Button>
        ))}
      </div>

      {/* Bulk Actions Bar */}
      <BulkActionBar
        selectedCount={selectedIds.size}
        actions={[
          { label: t('alerts:acknowledgeSelected'), icon: <Check className="h-4 w-4" />, onClick: handleBulkAcknowledge, disabled: bulkProcessing },
          { label: t('alerts:deleteSelected'), icon: <Trash2 className="h-4 w-4" />, onClick: handleBulkDelete, disabled: bulkProcessing, variant: 'outline' },
        ]}
        onCancel={() => setSelectedIds(new Set())}
      />

      {/* Table */}
      <Card>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[40px]">
                <Checkbox
                  checked={allOnPageSelected}
                  onCheckedChange={toggleAll}
                />
              </TableHead>
              <TableHead>{t('alerts:alertTitle')}</TableHead>
              <TableHead>{t('alerts:severity')}</TableHead>
              <TableHead>{t('common:status')}</TableHead>
              <TableHead>{t('alerts:source')}</TableHead>
              <TableHead>{t('alerts:createdAt')}</TableHead>
              <TableHead align="right">{t('automation:actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {alertsLoading ? (
              <EmptyStateInline title={t('common:loading')} colSpan={7} />
            ) : filteredAlerts.length === 0 ? (
              <EmptyStateInline title={`${t('alerts:noAlerts')} - ${t('alerts:noAlertsDesc')}`} colSpan={7} />
            ) : paginatedAlerts.length === 0 ? (
              <EmptyStateInline title={t('alerts:noAlertsOnPage')} colSpan={7} />
            ) : (
              paginatedAlerts.map((alert) => (
                <TableRow
                  key={alert.id}
                  className={!alert.acknowledged ? (
                    alert.severity === 'critical' ? 'bg-red-500/5' :
                    alert.severity === 'warning' ? 'bg-yellow-500/5' :
                    'bg-blue-500/5'
                  ) : ''}
                >
                  <TableCell>
                    <Checkbox
                      checked={selectedIds.has(alert.id)}
                      onCheckedChange={() => toggleSelection(alert.id)}
                    />
                  </TableCell>
                  <TableCell>
                    <div className="max-w-md">
                      <div className="font-medium truncate">{alert.title}</div>
                      <div className="text-xs text-muted-foreground truncate">{alert.message}</div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <AlertBadge level={alert.severity as "critical" | "warning" | "info" | "emergency"} />
                  </TableCell>
                  <TableCell>
                    {alert.acknowledged ? (
                      <Badge variant="outline">{t('alerts:acknowledged')}</Badge>
                    ) : (
                      <Badge variant="default">{t('alerts:unacknowledged')}</Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    {alert.source && (
                      <Badge variant="outline" className="text-xs">
                        {alert.source}
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {formatTimestamp(alert.created_at)}
                  </TableCell>
                  <TableCell align="right">
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => setSelectedAlert(alert)}
                      >
                        <Eye className="h-4 w-4" />
                      </Button>
                      {!alert.acknowledged && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8"
                          onClick={() => handleAcknowledge(alert.id)}
                          disabled={acknowledgingId === alert.id}
                        >
                          <Check className="mr-1 h-3 w-3" />
                          {acknowledgingId === alert.id ? t('alerts:acknowledging') : t('alerts:acknowledge')}
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>

        {/* Pagination */}
        {filteredAlerts.length > alertsPerPage && (
          <div className="px-4 pt-4 border-t">
            <Pagination
              total={filteredAlerts.length}
              pageSize={alertsPerPage}
              currentPage={page}
              onPageChange={setPage}
            />
          </div>
        )}
      </Card>

      {/* Create Alert Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('alerts:createAlert')}</DialogTitle>
            <DialogDescription>
              {t('alerts:manualCreateDesc')}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label htmlFor="alert-title">{t('alerts:alertTitle')}</Label>
              <Input
                id="alert-title"
                value={newAlertTitle}
                onChange={(e) => setNewAlertTitle(e.target.value)}
                placeholder={t('alerts:titlePlaceholder')}
              />
            </div>
            <div>
              <Label htmlFor="alert-severity">{t('alerts:severity')}</Label>
              <Select value={newAlertSeverity} onValueChange={(v: any) => setNewAlertSeverity(v)}>
                <SelectTrigger id="alert-severity">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="info">{t('alerts:info')}</SelectItem>
                  <SelectItem value="warning">{t('alerts:warning')}</SelectItem>
                  <SelectItem value="critical">{t('alerts:critical')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label htmlFor="alert-message">{t('alerts:detailDescription')}</Label>
              <Textarea
                id="alert-message"
                value={newAlertMessage}
                onChange={(e) => setNewAlertMessage(e.target.value)}
                placeholder={t('alerts:descriptionPlaceholder')}
                className="min-h-[80px]"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
              {t('common:cancel')}
            </Button>
            <Button
              onClick={handleCreateAlert}
              disabled={!newAlertTitle.trim() || !newAlertMessage.trim() || creating}
            >
              {creating ? t('common:creating') : t('common:add')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Alert Detail Dialog */}
      <Dialog open={!!selectedAlert} onOpenChange={() => setSelectedAlert(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Bell className="h-5 w-5" />
              {selectedAlert?.title}
            </DialogTitle>
            <DialogDescription>
              {selectedAlert && <AlertBadge level={selectedAlert.severity as "critical" | "warning" | "info" | "emergency"} />}
            </DialogDescription>
          </DialogHeader>
          {selectedAlert && (
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">{t('common:status')}:</span>{' '}
                  {selectedAlert.acknowledged ? (
                    <Badge variant="outline">{t('alerts:acknowledged')}</Badge>
                  ) : (
                    <Badge variant="default">{t('alerts:unacknowledged')}</Badge>
                  )}
                </div>
                <div>
                  <span className="text-muted-foreground">{t('alerts:source')}:</span>{' '}
                  <span className="font-medium">{selectedAlert.source || 'N/A'}</span>
                </div>
                <div className="col-span-2">
                  <span className="text-muted-foreground">{t('alerts:createdAt')}:</span>{' '}
                  <span className="font-medium">{formatTimestamp(selectedAlert.created_at)}</span>
                </div>
              </div>

              <div>
                <h4 className="font-medium mb-2">{t('alerts:detailDescription')}</h4>
                <p className="text-sm text-muted-foreground whitespace-pre-wrap">{selectedAlert.message}</p>
              </div>
            </div>
          )}
          <DialogFooter>
            {!selectedAlert?.acknowledged && (
              <Button
                onClick={() => {
                  if (selectedAlert) {
                    handleAcknowledge(selectedAlert.id)
                    setSelectedAlert(null)
                  }
                }}
                disabled={acknowledgingId === selectedAlert?.id}
              >
                {t('alerts:confirmAlert')}
              </Button>
            )}
            <Button variant="outline" onClick={() => setSelectedAlert(null)}>
              {t('common:close')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
