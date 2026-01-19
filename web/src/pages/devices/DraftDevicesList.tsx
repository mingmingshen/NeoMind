import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { LoadingState, EmptyStateInline, Pagination } from "@/components/shared"
import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Check, X, Eye, RefreshCw, Zap } from "lucide-react"
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
import { cn } from "@/lib/utils"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"

// Types for draft devices
interface DraftDevice {
  id: string
  device_id: string
  source: string
  status: string
  sample_count: number
  max_samples: number
  generated_type?: GeneratedDeviceType
  discovered_at: number
  updated_at: number
  error_message?: string
  user_name?: string
}

// Type signature for existing types
interface TypeSignatures {
  signatures: Record<string, string>  // signature_hash -> device_type
  count: string
}

interface GeneratedDeviceType {
  device_type: string
  name: string
  description: string
  category: string
  metrics: MetricSummary[]
  confidence: number
  summary: ProcessingSummary
}

interface MetricSummary {
  name: string
  path: string
  semantic_type: string
  display_name: string
  confidence: number
}

interface ProcessingSummary {
  samples_analyzed: number
  fields_discovered: number
  metrics_generated: number
  inferred_category: string
  insights: string[]
  warnings: string[]
  recommendations: string[]
}

interface DraftDevicesListProps {
  onRefresh?: () => void
}

export function DraftDevicesList({ onRefresh }: DraftDevicesListProps) {
  const { t } = useTranslation(['common', 'devices'])
  const { toast } = useToast()

  const [drafts, setDrafts] = useState<DraftDevice[]>([])
  const [loading, setLoading] = useState(true)

  // Pagination state
  const [page, setPage] = useState(1)
  const itemsPerPage = 10

  // Reset pagination when data changes
  useEffect(() => {
    setPage(1)
  }, [drafts.length])

  // Paginated data
  const paginatedDrafts = drafts.slice(
    (page - 1) * itemsPerPage,
    page * itemsPerPage
  )
  const [selectedDraft, setSelectedDraft] = useState<DraftDevice | null>(null)
  const [processing, setProcessing] = useState<string | null>(null)
  const [typeSignatures, setTypeSignatures] = useState<TypeSignatures | null>(null)
  const [showApproveDialog, setShowApproveDialog] = useState(false)
  const [selectedDraftForApproval, setSelectedDraftForApproval] = useState<DraftDevice | null>(null)
  const [selectedExistingType, setSelectedExistingType] = useState<string | undefined>(undefined)

  // Fetch drafts
  const fetchDrafts = async () => {
    setLoading(true)
    try {
      const response = await api.getDraftDevices()
      setDrafts(response.items || [])
    } catch (error) {
      console.error('Failed to fetch draft devices:', error)
      // Don't show error toast - endpoint might not be implemented yet
      setDrafts([])
    } finally {
      setLoading(false)
    }
  }

  // Fetch type signatures for type reuse
  const fetchTypeSignatures = async () => {
    try {
      const response = await api.getTypeSignatures()
      setTypeSignatures(response)
    } catch (error) {
      console.error('Failed to fetch type signatures:', error)
      // Not critical, continue without type signatures
      setTypeSignatures(null)
    }
  }

  useEffect(() => {
    fetchDrafts()
    fetchTypeSignatures()
    // Poll every 10 seconds for updates
    const interval = setInterval(() => {
      fetchDrafts()
      fetchTypeSignatures()
    }, 10000)
    return () => clearInterval(interval)
  }, [])

  // Approve draft device
  const handleApproveClick = (draft: DraftDevice) => {
    setSelectedDraftForApproval(draft)
    setSelectedExistingType(undefined)
    setShowApproveDialog(true)
  }

  const handleApprove = async () => {
    if (!selectedDraftForApproval) return

    setProcessing(selectedDraftForApproval.id)
    try {
      await api.approveDraftDeviceWithType(selectedDraftForApproval.device_id, selectedExistingType)
      toast({
        title: t('common:success'),
        description: selectedExistingType
          ? t('devices:drafts.approvedWithExisting', { deviceId: selectedDraftForApproval.device_id, type: selectedExistingType })
          : t('devices:drafts.approved', { deviceId: selectedDraftForApproval.device_id }),
      })
      setShowApproveDialog(false)
      await fetchDrafts()
      onRefresh?.()
    } catch (error) {
      toast({
        title: t('common:failed'),
        description: t('devices:drafts.approveFailed'),
        variant: "destructive"
      })
    } finally {
      setProcessing(null)
      setSelectedDraftForApproval(null)
      setSelectedExistingType(undefined)
    }
  }

  // Reject draft device
  const handleReject = async (draft: DraftDevice) => {
    const reason = prompt(t('devices:drafts.rejectReason'))
    if (!reason) return

    setProcessing(draft.id)
    try {
      await api.rejectDraftDevice(draft.device_id, { reason })
      toast({
        title: t('common:success'),
        description: t('devices:drafts.rejected', { deviceId: draft.device_id }),
      })
      await fetchDrafts()
    } catch (error) {
      toast({
        title: t('common:failed'),
        description: t('devices:drafts.rejectFailed'),
        variant: "destructive"
      })
    } finally {
      setProcessing(null)
    }
  }

  // Trigger manual analysis
  const handleAnalyze = async (draft: DraftDevice) => {
    setProcessing(draft.id)
    try {
      await api.triggerDraftAnalysis(draft.device_id)
      toast({
        title: t('common:success'),
        description: t('devices:drafts.analysisTriggered'),
      })
      await fetchDrafts()
    } catch (error) {
      toast({
        title: t('common:failed'),
        description: t('devices:drafts.analysisFailed'),
        variant: "destructive"
      })
    } finally {
      setProcessing(null)
    }
  }

  // Get status badge
  const getStatusBadge = (status: string) => {
    const statusMap: Record<string, { color: string; label: string }> = {
      collecting: { color: "bg-blue-500/20 text-blue-700", label: t('devices:drafts.status.collecting') },
      analyzing: { color: "bg-purple-500/20 text-purple-700", label: t('devices:drafts.status.analyzing') },
      pending_review: { color: "bg-yellow-500/20 text-yellow-700", label: t('devices:drafts.status.pendingReview') },
      registered: { color: "bg-green-500/20 text-green-700", label: t('devices:drafts.status.registered') },
      rejected: { color: "bg-red-500/20 text-red-700", label: t('devices:drafts.status.rejected') },
      failed: { color: "bg-red-500/20 text-red-700", label: t('devices:drafts.status.failed') },
    }

    const key = status.toLowerCase().replace(/[^a-z]/g, '_')
    const info = statusMap[key] || { color: "bg-gray-500/20 text-gray-700", label: status }

    return (
      <Badge className={cn("font-normal", info.color)}>
        {info.label}
      </Badge>
    )
  }

  // Get confidence badge
  const getConfidenceBadge = (confidence?: number) => {
    if (!confidence) return null

    if (confidence >= 0.85) {
      return <Badge variant="default" className="bg-green-500/20 text-green-700">
        {t('devices:drafts.confidence.high')} ({Math.round(confidence * 100)}%)
      </Badge>
    } else if (confidence >= 0.6) {
      return <Badge variant="default" className="bg-yellow-500/20 text-yellow-700">
        {t('devices:drafts.confidence.medium')} ({Math.round(confidence * 100)}%)
      </Badge>
    } else {
      return <Badge variant="default" className="bg-red-500/20 text-red-700">
        {t('devices:drafts.confidence.low')} ({Math.round(confidence * 100)}%)
      </Badge>
    }
  }

  return (
    <>
      {loading ? (
        <LoadingState text={t('devices:drafts.loading')} />
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t('devices:drafts.headers.deviceId')}</TableHead>
                <TableHead>{t('devices:drafts.headers.source')}</TableHead>
                <TableHead>{t('devices:drafts.headers.status')}</TableHead>
                <TableHead>{t('devices:drafts.headers.samples')}</TableHead>
                <TableHead>{t('devices:drafts.headers.confidence')}</TableHead>
                <TableHead>{t('devices:drafts.headers.category')}</TableHead>
                <TableHead className="text-right">{t('devices:drafts.headers.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {drafts.length === 0 ? (
                <EmptyStateInline
                  title={t('devices:drafts.noDrafts')}
                  colSpan={7}
                />
              ) : (
                paginatedDrafts.map((draft) => (
                  <TableRow key={draft.id}>
                    <TableCell className="font-mono text-xs">
                      {draft.device_id}
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="text-xs">
                        {draft.source}
                      </Badge>
                    </TableCell>
                    <TableCell>{getStatusBadge(draft.status)}</TableCell>
                    <TableCell>
                      {draft.sample_count} / {draft.max_samples}
                    </TableCell>
                    <TableCell>
                      {getConfidenceBadge(draft.generated_type?.confidence)}
                    </TableCell>
                    <TableCell>
                      {draft.generated_type?.category || "-"}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        {/* View details */}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => setSelectedDraft(draft)}
                          title={t('devices:drafts.viewDetails')}
                        >
                          <Eye className="h-4 w-4" />
                        </Button>

                        {/* Trigger analysis - only for collecting status */}
                        {draft.status === "collecting" && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => handleAnalyze(draft)}
                            disabled={processing === draft.id}
                            title={t('devices:drafts.triggerAnalysis')}
                          >
                            <Zap className="h-4 w-4" />
                          </Button>
                        )}

                        {/* Refresh - for any status */}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => fetchDrafts()}
                          title={t('common:refresh')}
                        >
                          <RefreshCw className="h-4 w-4" />
                        </Button>

                        {/* Approve - only for pending_review */}
                        {draft.status === "pending_review" && draft.generated_type && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-green-600"
                            onClick={() => handleApproveClick(draft)}
                            disabled={processing === draft.id}
                            title={t('devices:drafts.approve')}
                          >
                            <Check className="h-4 w-4" />
                          </Button>
                        )}

                        {/* Reject - for pending_review or collecting */}
                        {(draft.status === "pending_review" || draft.status === "collecting") && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-red-600"
                            onClick={() => handleReject(draft)}
                            disabled={processing === draft.id}
                            title={t('devices:drafts.reject')}
                          >
                            <X className="h-4 w-4" />
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </Card>
      )}

      {drafts.length > itemsPerPage && (
        <div className="pt-4">
          <Pagination
            total={drafts.length}
            pageSize={itemsPerPage}
            currentPage={page}
            onPageChange={setPage}
          />
        </div>
      )}

      {/* Approve Draft Dialog */}
      {showApproveDialog && selectedDraftForApproval && (
        <Dialog open={showApproveDialog} onOpenChange={setShowApproveDialog}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>{t('devices:drafts.approveTitle')}</DialogTitle>
              <DialogDescription>
                {t('devices:drafts.approveDesc')}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-4">
              {/* Device info */}
              <div className="text-sm">
                <span className="text-muted-foreground">{t('devices:drafts.headers.deviceId')}:</span>
                <span className="ml-2 font-mono">{selectedDraftForApproval.device_id}</span>
              </div>

              {/* Generated type info */}
              {selectedDraftForApproval.generated_type && (
                <div className="text-sm">
                  <span className="text-muted-foreground">{t('devices:drafts.generatedType')}:</span>
                  <span className="ml-2">{selectedDraftForApproval.generated_type.name}</span>
                </div>
              )}

              {/* Existing type selector */}
              {typeSignatures && Object.keys(typeSignatures.signatures).length > 0 && (
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('devices:drafts.selectExistingType')}</label>
                  <Select value={selectedExistingType} onValueChange={setSelectedExistingType}>
                    <SelectTrigger>
                      <SelectValue placeholder={t('devices:drafts.selectExistingTypePlaceholder')} />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">
                        {t('devices:drafts.createNewType')}
                      </SelectItem>
                      {Array.from(new Set(Object.values(typeSignatures.signatures))).map((typeId) => (
                        <SelectItem key={typeId} value={typeId}>
                          {typeId}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    {t('devices:drafts.selectExistingTypeHint')}
                  </p>
                </div>
              )}
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={() => setShowApproveDialog(false)}>
                {t('common:cancel')}
              </Button>
              <Button onClick={handleApprove} disabled={processing === selectedDraftForApproval.id}>
                {processing === selectedDraftForApproval.id ? t('common:processing') : t('devices:drafts.approve')}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}

      {/* Draft Details Dialog */}
      {selectedDraft && (
        <DraftDetailDialog
          draft={selectedDraft}
          open={!!selectedDraft}
          onOpenChange={(open) => !open && setSelectedDraft(null)}
        />
      )}
    </>
  )
}

// Draft detail dialog component
interface DraftDetailDialogProps {
  draft: DraftDevice
  open: boolean
  onOpenChange: (open: boolean) => void
}

function DraftDetailDialog({ draft, open, onOpenChange }: DraftDetailDialogProps) {
  const { t } = useTranslation(['common', 'devices'])

  const genType = draft.generated_type

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <span className="font-mono text-sm">{draft.device_id}</span>
            <Badge variant="outline" className="text-xs">
              {draft.source}
            </Badge>
          </DialogTitle>
          <DialogDescription>
            {t('devices:drafts.detailDesc')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Status info */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-sm font-medium text-muted-foreground">{t('devices:drafts.headers.status')}</label>
              <div className="mt-1">
                {/* Status will be shown here - simplified version */}
                <span className="capitalize">{draft.status.replace(/_/g, ' ')}</span>
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-muted-foreground">{t('devices:drafts.headers.samples')}</label>
              <div className="mt-1">
                {draft.sample_count} / {draft.max_samples}
              </div>
            </div>
          </div>

          {/* Generated type info */}
          {genType ? (
            <>
              <div className="border-t pt-4">
                <h3 className="font-semibold mb-3">{t('devices:drafts.generatedType')}</h3>
                <div className="space-y-3">
                  <div>
                    <label className="text-sm font-medium text-muted-foreground">{t('devices:types.headers.name')}</label>
                    <p className="mt-1">{genType.name}</p>
                  </div>
                  <div>
                    <label className="text-sm font-medium text-muted-foreground">{t('devices:types.headers.description')}</label>
                    <p className="mt-1 text-sm text-muted-foreground">{genType.description}</p>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="text-sm font-medium text-muted-foreground">{t('devices:drafts.headers.category')}</label>
                      <p className="mt-1 capitalize">{genType.category.toLowerCase()}</p>
                    </div>
                    <div>
                      <label className="text-sm font-medium text-muted-foreground">{t('devices:drafts.headers.confidence')}</label>
                      <p className="mt-1">{Math.round(genType.confidence * 100)}%</p>
                    </div>
                  </div>
                </div>
              </div>

              {/* Metrics */}
              {genType.metrics.length > 0 && (
                <div className="border-t pt-4">
                  <h4 className="font-semibold mb-3">{t('devices:drafts.metrics')} ({genType.metrics.length})</h4>
                  <div className="max-h-60 overflow-y-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>{t('devices:drafts.metricName')}</TableHead>
                          <TableHead>{t('devices:drafts.path')}</TableHead>
                          <TableHead>{t('devices:drafts.semanticType')}</TableHead>
                          <TableHead>{t('devices:drafts.confidence')}</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {genType.metrics.slice(0, 10).map((metric, i) => (
                          <TableRow key={i}>
                            <TableCell className="font-mono text-xs">{metric.name}</TableCell>
                            <TableCell className="font-mono text-xs text-muted-foreground">{metric.path}</TableCell>
                            <TableCell className="capitalize text-sm">{metric.semantic_type.toLowerCase()}</TableCell>
                            <TableCell>
                              <Badge
                                variant={metric.confidence >= 0.7 ? "default" : "secondary"}
                                className={metric.confidence >= 0.7 ? "bg-green-500/20 text-green-700" : "bg-yellow-500/20 text-yellow-700"}
                              >
                                {Math.round(metric.confidence * 100)}%
                              </Badge>
                            </TableCell>
                          </TableRow>
                        ))}
                        {genType.metrics.length > 10 && (
                          <TableRow>
                            <TableCell colSpan={4} className="text-center text-sm text-muted-foreground">
                                + {genType.metrics.length - 10} {t('devices:drafts.moreMetrics')}
                              </TableCell>
                          </TableRow>
                        )}
                      </TableBody>
                    </Table>
                  </div>
                </div>
              )}

              {/* Summary */}
              {genType.summary && (
                <div className="border-t pt-4">
                  <h4 className="font-semibold mb-3">{t('devices:drafts.analysisSummary')}</h4>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-muted-foreground">{t('devices:drafts.samplesAnalyzed')}:</span>
                      <span>{genType.summary.samples_analyzed}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-muted-foreground">{t('devices:drafts.fieldsDiscovered')}:</span>
                      <span>{genType.summary.fields_discovered}</span>
                    </div>
                    {genType.summary.warnings.length > 0 && (
                      <div className="rounded-md bg-yellow-500/10 p-3">
                        <p className="font-medium text-yellow-700 mb-1">{t('devices:drafts.warnings')}:</p>
                        <ul className="list-disc list-inside text-sm text-yellow-600 space-y-1">
                          {genType.summary.warnings.map((warning, i) => (
                            <li key={i}>{warning}</li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              <p>{t('devices:drafts.noGeneratedType')}</p>
              <p className="text-sm">{t('devices:drafts.waitingForSamples')}</p>
            </div>
          )}

          {/* Error message */}
          {draft.error_message && (
            <div className="rounded-md bg-red-500/10 p-3">
              <p className="font-medium text-red-700">{t('devices:drafts.error')}:</p>
              <p className="text-sm text-red-600">{draft.error_message}</p>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common:close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
