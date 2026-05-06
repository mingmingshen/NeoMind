import { useState, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { format } from 'date-fns'
import { UnifiedFormDialog } from '@/components/dialog/UnifiedFormDialog'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Calendar } from '@/components/ui/calendar'
import { Download, CalendarIcon, Clock } from 'lucide-react'
import { api } from '@/lib/api'
import { useToast } from '@/hooks/use-toast'
import { textNano } from '@/design-system/tokens/typography'
import { cn } from '@/lib/utils'
import type { UnifiedDataSourceInfo } from '@/types'
import * as XLSX from 'xlsx'
import JSZip from 'jszip'

interface ExportDataDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  source: UnifiedDataSourceInfo | null
}

/** Excel cell max length */
const EXCEL_CELL_MAX = 32000

/** Format Date for display: "yyyy-MM-dd HH:mm:ss" */
function formatDateTimeDisplay(date: Date): string {
  return format(date, 'yyyy-MM-dd HH:mm:ss')
}

/** Format Unix seconds to human-readable "yyyy-MM-dd HH:mm:ss" */
function formatTimestamp(unixSeconds: number): string {
  return formatDateTimeDisplay(new Date(unixSeconds * 1000))
}

/** Sanitize string for use in filenames */
function sanitizeFilename(s: string): string {
  return s.replace(/[:\s]/g, '_').replace(/[^a-zA-Z0-9_\-.]/g, '')
}

/** Get current time as filename-safe string: "YYYYMMDD_HHmmss" */
function fileTimestamp(): string {
  return format(new Date(), 'yyyyMMdd_HHmmss')
}

/** Truncate value string to Excel cell limit */
function truncateForExcel(val: string): string {
  if (val.length <= EXCEL_CELL_MAX) return val
  return val.slice(0, EXCEL_CELL_MAX) + '...[truncated]'
}

function isBinaryDataType(dataType: string): boolean {
  return dataType === 'binary'
}

export function ExportDataDialog({ open, onOpenChange, source }: ExportDataDialogProps) {
  const { t } = useTranslation('data')
  const { toast } = useToast()
  const abortRef = useRef<AbortController | null>(null)

  const now = new Date()
  const defaultStart = new Date(now.getTime() - 24 * 3600 * 1000)
  const [startDate, setStartDate] = useState<Date>(defaultStart)
  const [startTime, setStartTime] = useState(format(defaultStart, 'HH:mm:ss'))
  const [endDate, setEndDate] = useState<Date>(now)
  const [endTime, setEndTime] = useState(format(now, 'HH:mm:ss'))
  const [exporting, setExporting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const isBinary = source ? isBinaryDataType(source.data_type) : false

  // Reset state when source changes
  const handleOpenChange = useCallback((nextOpen: boolean) => {
    if (!nextOpen) {
      abortRef.current?.abort()
      setExporting(false)
      setError(null)
    }
    onOpenChange(nextOpen)
  }, [onOpenChange])

  const handleExport = useCallback(async () => {
    if (!source) return

    abortRef.current?.abort()
    const controller = new AbortController()
    abortRef.current = controller

    setError(null)
    setExporting(true)

    try {
      // Parse DataSourceId: "device:sensor1:temperature" → source="device:sensor1", metric="temperature"
      const parts = source.id.split(':')
      const sourcePart = `${parts[0]}:${parts[1]}`
      const metricPart = parts.slice(2).join(':')

      // Combine date + time into full Date objects, then convert to Unix seconds
      const [startH, startM, startS] = startTime.split(':').map(Number)
      const startDateFull = new Date(startDate)
      startDateFull.setHours(startH, startM, startS || 0, 0)

      const [endH, endM, endS] = endTime.split(':').map(Number)
      const endDateFull = new Date(endDate)
      endDateFull.setHours(endH, endM, endS || 0, 0)

      const startUnix = Math.floor(startDateFull.getTime() / 1000)
      const endUnix = Math.floor(endDateFull.getTime() / 1000)

      const result = await api.queryTelemetry(sourcePart, metricPart, startUnix, endUnix, 1000)
      if (controller.signal.aborted) return

      const data = result?.data || []
      if (data.length === 0) {
        setError(t('export.noData'))
        return
      }

      // Warn about truncation
      if (result.total_count != null && result.total_count > data.length) {
        toast({
          title: t('export.truncated', { count: data.length, total: result.total_count }),
        })
      }

      // Generate file
      const baseName = `${sanitizeFilename(source.source_display_name)}_${sanitizeFilename(source.field_display_name)}_${fileTimestamp()}`

      if (isBinary) {
        await generateZip(baseName, data, source)
      } else {
        await generateExcel(baseName, data)
      }

      // Success toast
      toast({
        title: t('export.success'),
        description: `${baseName}${isBinary ? '.zip' : '.xlsx'}`,
      })

      // Close dialog on success
      handleOpenChange(false)
    } catch (err) {
      if (controller.signal.aborted) return
      console.error('[ExportDataDialog] Export failed:', err)
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      if (!controller.signal.aborted) {
        setExporting(false)
      }
    }
  }, [source, startDate, startTime, endDate, endTime, isBinary, t, toast, handleOpenChange])

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={handleOpenChange}
      title={t('export.title')}
      description={source?.id}
      icon={<Download className="h-5 w-5" />}
      width="sm"
      submitLabel={exporting ? t('export.exporting') : t('export.button')}
      onSubmit={handleExport}
      isSubmitting={exporting}
      submitDisabled={exporting}
      showCancelButton
    >
      {source && (
        <div className="space-y-4">
          {/* Source Info */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <p className="text-xs text-muted-foreground">{t('columns.source')}</p>
              <p className="text-sm font-medium">{source.source_display_name}</p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground">{t('columns.field')}</p>
              <p className="text-sm font-medium">{source.field_display_name}</p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground">{t('columns.dataType')}</p>
              <Badge variant="secondary" className={textNano}>{source.data_type}</Badge>
            </div>
            <div>
              <p className="text-xs text-muted-foreground">{isBinary ? t('export.format.zip') : t('export.format.excel')}</p>
              <p className="text-sm">{isBinary ? '.zip' : '.xlsx'}</p>
            </div>
          </div>

          {/* Start Time Range */}
          <div>
            <label className="text-xs text-muted-foreground mb-1.5 block">{t('export.startTime')}</label>
            <div className="flex items-center gap-2">
              <Popover>
                <PopoverTrigger asChild>
                  <Button
                    variant="outline"
                    className={cn(
                      "h-9 justify-start text-left text-sm font-normal",
                      !startDate && "text-muted-foreground"
                    )}
                  >
                    <CalendarIcon className="mr-1.5 h-3.5 w-3.5" />
                    {startDate ? format(startDate, 'yyyy-MM-dd') : 'Pick date'}
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-auto p-0" align="start">
                  <Calendar
                    mode="single"
                    selected={startDate}
                    onSelect={(d) => d && setStartDate(d)}
                    initialFocus
                  />
                </PopoverContent>
              </Popover>
              <div className="relative">
                <Clock className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
                <Input
                  type="time"
                  step="1"
                  value={startTime}
                  onChange={e => setStartTime(e.target.value)}
                  className="h-9 text-sm pl-7 w-[120px]"
                />
              </div>
            </div>
          </div>

          {/* End Time Range */}
          <div>
            <label className="text-xs text-muted-foreground mb-1.5 block">{t('export.endTime')}</label>
            <div className="flex items-center gap-2">
              <Popover>
                <PopoverTrigger asChild>
                  <Button
                    variant="outline"
                    className={cn(
                      "h-9 justify-start text-left text-sm font-normal",
                      !endDate && "text-muted-foreground"
                    )}
                  >
                    <CalendarIcon className="mr-1.5 h-3.5 w-3.5" />
                    {endDate ? format(endDate, 'yyyy-MM-dd') : 'Pick date'}
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-auto p-0" align="start">
                  <Calendar
                    mode="single"
                    selected={endDate}
                    onSelect={(d) => d && setEndDate(d)}
                    initialFocus
                  />
                </PopoverContent>
              </Popover>
              <div className="relative">
                <Clock className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
                <Input
                  type="time"
                  step="1"
                  value={endTime}
                  onChange={e => setEndTime(e.target.value)}
                  className="h-9 text-sm pl-7 w-[120px]"
                />
              </div>
            </div>
          </div>

          {/* Error */}
          {error && (
            <p className="text-sm text-error bg-error-light rounded px-3 py-2">{error}</p>
          )}
        </div>
      )}
    </UnifiedFormDialog>
  )
}

/** Generate and download an Excel file */
async function generateExcel(
  baseName: string,
  data: Array<{ timestamp: number; value: unknown; quality: number | null }>,
) {
  const rows = data.map(p => {
    const rawValue = typeof p.value === 'object' ? JSON.stringify(p.value) : String(p.value ?? '')
    return {
      Timestamp: formatTimestamp(p.timestamp),
      Value: truncateForExcel(rawValue),
      Quality: p.quality !== null ? `${(p.quality * 100).toFixed(0)}%` : '-',
    }
  })

  const ws = XLSX.utils.json_to_sheet(rows)
  // Auto-width columns
  ws['!cols'] = [
    { wch: 22 }, // Timestamp
    { wch: 30 }, // Value (wider to show more content)
    { wch: 10 }, // Quality
  ]

  const wb = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(wb, ws, 'Data')
  XLSX.writeFile(wb, `${baseName}.xlsx`)
}

/** Generate and download a ZIP file */
async function generateZip(
  baseName: string,
  data: Array<{ timestamp: number; value: unknown; quality: number | null }>,
  source: UnifiedDataSourceInfo,
) {
  const zip = new JSZip()

  // manifest.json with metadata
  zip.file('manifest.json', JSON.stringify({
    source: source.id,
    source_name: source.source_display_name,
    field: source.field_display_name,
    data_type: source.data_type,
    point_count: data.length,
    exported_at: new Date().toISOString(),
    note: 'Binary data export is partially supported. Actual file contents are placeholders.',
  }, null, 2))

  // data.csv for quick reference
  const csvRows = ['Timestamp,Value,Quality']
  for (const p of data) {
    const val = typeof p.value === 'object' ? JSON.stringify(p.value) : String(p.value ?? '')
    const q = p.quality !== null ? `${(p.quality * 100).toFixed(0)}%` : '-'
    csvRows.push(`${formatTimestamp(p.timestamp)},${val},${q}`)
  }
  zip.file('data.csv', csvRows.join('\n'))

  const blob = await zip.generateAsync({ type: 'blob' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${baseName}.zip`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  setTimeout(() => URL.revokeObjectURL(url), 100)
}
