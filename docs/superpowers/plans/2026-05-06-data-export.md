# Data Export Feature - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-row data export to the Data Explorer page with custom time range selection, supporting Excel export for text data and ZIP export for binary data.

**Architecture:** Frontend-only implementation. Uses existing `GET /api/telemetry` API endpoint. File generation done in-browser using `xlsx` and `jszip` libraries. A new `ExportDataDialog` component handles the UI and export logic.

**Tech Stack:** React 18, TypeScript, xlsx, jszip, UnifiedFormDialog, native datetime-local inputs, useToast

**Spec:** `docs/superpowers/specs/2026-05-06-data-export-design.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `web/src/lib/api.ts:1615` | Fix `queryTelemetry` return type (`value: unknown`) |
| Create | `web/src/components/data/ExportDataDialog.tsx` | Export dialog with time range picker and file generation |
| Modify | `web/src/pages/data-explorer.tsx` | Add download buttons + wire up dialog |
| Modify | `web/src/i18n/locales/en/data.json` | English translations for export |
| Modify | `web/src/i18n/locales/zh/data.json` | Chinese translations for export |

---

### Task 1: Install Dependencies

**Files:**
- Modify: `web/package.json`

- [ ] **Step 1: Install xlsx and jszip**

```bash
cd web && npm install xlsx jszip
```

- [ ] **Step 2: Verify installation**

```bash
cd web && cat package.json | grep -E '"xlsx"|"jszip"'
```

Expected: both packages listed in dependencies

- [ ] **Step 3: Commit**

```bash
git add web/package.json web/package-lock.json
git commit -m "chore: add xlsx and jszip dependencies for data export"
```

---

### Task 2: Add i18n Keys

**Files:**
- Modify: `web/src/i18n/locales/en/data.json`
- Modify: `web/src/i18n/locales/zh/data.json`

- [ ] **Step 1: Add English i18n keys to `web/src/i18n/locales/en/data.json`**

Replace the entire file. This adds both the new `export` section AND keys that were previously only used with inline defaults in `data-explorer.tsx` (e.g. `currentValue`, `history`, `range.*`).

```json
{
  "title": "Data Explorer",
  "subtitle": "Browse all data sources across devices, extensions, and transforms",
  "tabs": {
    "all": "All",
    "device": "Devices",
    "extension": "Extensions",
    "transform": "Transforms",
    "ai": "AI Metrics"
  },
  "columns": {
    "type": "Type",
    "source": "Source",
    "field": "Field",
    "dataType": "Data Type",
    "updated": "Updated"
  },
  "noResults": "No data sources match your search",
  "noSources": "No data sources found",
  "search": "Search data sources...",
  "filterSource": "Filter source...",
  "allSources": "All Sources",
  "noSourcesDesc": "Data sources will appear here once devices are connected or extensions are registered",
  "currentValue": "Current Value",
  "unit": "Unit",
  "lastUpdate": "Last Update",
  "description": "Description",
  "history": "History",
  "range.1h": "1 Hour",
  "range.6h": "6 Hours",
  "range.24h": "24 Hours",
  "range.7d": "7 Days",
  "timestamp": "Timestamp",
  "value": "Value",
  "quality": "Quality",
  "noHistory": "No historical data available for this period",
  "export": {
    "title": "Export Data",
    "startTime": "Start Time",
    "endTime": "End Time",
    "button": "Export",
    "exporting": "Exporting...",
    "noData": "No data found for the selected time range",
    "truncated": "Showing {{count}} of {{total}} data points",
    "format.excel": "Excel (.xlsx)",
    "format.zip": "ZIP (.zip)",
    "binaryWarning": "Binary data export is partially supported — actual file contents may not be available."
  }
}
```

- [ ] **Step 2: Add Chinese i18n keys to `web/src/i18n/locales/zh/data.json`**

Replace the entire file. Same as English — adds export keys and fills in previously-hardcoded-only keys.

```json
{
  "title": "数据浏览器",
  "subtitle": "浏览设备、扩展和转换中的所有数据源",
  "tabs": {
    "all": "全部",
    "device": "设备",
    "extension": "扩展",
    "transform": "转换",
    "ai": "AI 指标"
  },
  "columns": {
    "type": "类型",
    "source": "来源",
    "field": "字段",
    "dataType": "数据类型",
    "updated": "更新时间"
  },
  "noResults": "没有匹配的数据源",
  "noSources": "未找到数据源",
  "search": "搜索数据源...",
  "filterSource": "筛选来源...",
  "allSources": "全部来源",
  "noSourcesDesc": "设备连接或扩展注册后，数据源将出现在此处",
  "currentValue": "当前值",
  "unit": "单位",
  "lastUpdate": "最后更新",
  "description": "描述",
  "history": "历史记录",
  "range.1h": "1 小时",
  "range.6h": "6 小时",
  "range.24h": "24 小时",
  "range.7d": "7 天",
  "timestamp": "时间戳",
  "value": "值",
  "quality": "质量",
  "noHistory": "该时间段内没有历史数据",
  "export": {
    "title": "导出数据",
    "startTime": "开始时间",
    "endTime": "结束时间",
    "button": "导出",
    "exporting": "导出中...",
    "noData": "所选时间范围内没有数据",
    "truncated": "显示 {{count}} / {{total}} 条数据",
    "format.excel": "Excel (.xlsx)",
    "format.zip": "ZIP (.zip)",
    "binaryWarning": "二进制数据导出仅部分支持 — 实际文件内容可能不可用。"
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add web/src/i18n/locales/en/data.json web/src/i18n/locales/zh/data.json
git commit -m "feat(i18n): add data export translations for en/zh"
```

---

### Task 3: Fix `queryTelemetry` Return Type

**Files:**
- Modify: `web/src/lib/api.ts:1615`

- [ ] **Step 1: Fix the type definition**

In `web/src/lib/api.ts` at line 1615, change the return type of `queryTelemetry`:

From:
```typescript
return fetchAPI<{ source_id: string; data: Array<{ timestamp: number; value: number; quality: number }>; count: number }>(`/telemetry?${qs}`)
```

To:
```typescript
return fetchAPI<{ source_id: string; data: Array<{ timestamp: number; value: unknown; quality: number | null }>; count: number; total_count?: number }>(`/telemetry?${qs}`)
```

- [ ] **Step 2: Verify no type errors from this change**

```bash
cd web && npx tsc --noEmit 2>&1 | head -30
```

Expected: No new errors. Existing callers of `queryTelemetry` in `data-explorer.tsx` treat `value` as `unknown` already (they pass it to `formatHistoryValue` which handles `unknown`).

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/api.ts
git commit -m "fix: correct queryTelemetry return type to match backend"
```

---

### Task 4: Create ExportDataDialog Component

**Files:**
- Create: `web/src/components/data/ExportDataDialog.tsx`

This is the core component. It contains the dialog UI, data fetching, and file generation logic.

- [ ] **Step 1: Create the directory**

```bash
mkdir -p web/src/components/data
```

- [ ] **Step 2: Create `web/src/components/data/ExportDataDialog.tsx`**

```tsx
import { useState, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { UnifiedFormDialog } from '@/components/dialog/UnifiedFormDialog'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Loader2, Download } from 'lucide-react'
import { api } from '@/lib/api'
import { useToast } from '@/hooks/use-toast'
import { textNano } from '@/design-system/tokens/typography'
import type { UnifiedDataSourceInfo } from '@/types'
import * as XLSX from 'xlsx'
import JSZip from 'jszip'

interface ExportDataDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  source: UnifiedDataSourceInfo | null
}

/** Format Date to datetime-local input value string: "YYYY-MM-DDTHH:mm:ss" */
function toDatetimeLocal(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

/** Format Unix seconds to human-readable "YYYY-MM-DD HH:mm:ss" */
function formatTimestamp(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** Sanitize string for use in filenames */
function sanitizeFilename(s: string): string {
  return s.replace(/[:\s]/g, '_').replace(/[^a-zA-Z0-9_\-.]/g, '')
}

/** Get current time as filename-safe string: "YYYYMMDD_HHmmss" */
function fileTimestamp(): string {
  const now = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}_${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`
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
  const [startTime, setStartTime] = useState(toDatetimeLocal(defaultStart))
  const [endTime, setEndTime] = useState(toDatetimeLocal(now))
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

      const startUnix = Math.floor(new Date(startTime).getTime() / 1000)
      const endUnix = Math.floor(new Date(endTime).getTime() / 1000)

      const result = await api.queryTelemetry(sourcePart, metricPart, startUnix, endUnix, 1000)
      if (controller.signal.aborted) return

      const data = result?.data || []
      if (data.length === 0) {
        setError(t('export.noData'))
        return
      }

      // Warn about truncation (total_count may be absent from older API versions)
      if (result.total_count != null && result.total_count > data.length) {
        toast({
          title: t('export.truncated', { count: data.length, total: result.total_count }),
          variant: 'default',
        })
      }

      // Generate file
      const baseName = `${sanitizeFilename(source.source_display_name)}_${sanitizeFilename(source.field_display_name)}_${fileTimestamp()}`

      if (isBinary) {
        toast({
          title: t('export.binaryWarning'),
          variant: 'default',
        })
        await generateZip(baseName, data, source)
      } else {
        await generateExcel(baseName, data)
      }

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
  }, [source, startTime, endTime, isBinary, t, toast, handleOpenChange])

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

          {/* Time Range */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">{t('export.startTime')}</label>
              <Input
                type="datetime-local"
                step="1"
                value={startTime}
                onChange={e => setStartTime(e.target.value)}
                className="h-9 text-sm"
              />
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">{t('export.endTime')}</label>
              <Input
                type="datetime-local"
                step="1"
                value={endTime}
                onChange={e => setEndTime(e.target.value)}
                className="h-9 text-sm"
              />
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
  const rows = data.map(p => ({
    Timestamp: formatTimestamp(p.timestamp),
    Value: typeof p.value === 'object' ? JSON.stringify(p.value) : String(p.value ?? ''),
    Quality: p.quality !== null ? `${(p.quality * 100).toFixed(0)}%` : '-',
  }))

  const ws = XLSX.utils.json_to_sheet(rows)
  // Auto-width columns
  ws['!cols'] = [
    { wch: 22 }, // Timestamp
    { wch: 20 }, // Value
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
  a.click()
  URL.revokeObjectURL(url)
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd web && npx tsc --noEmit 2>&1 | grep -E "ExportData|error" | head -20
```

Expected: No errors related to ExportDataDialog

- [ ] **Step 4: Commit**

```bash
git add web/src/components/data/ExportDataDialog.tsx
git commit -m "feat: create ExportDataDialog component with Excel/ZIP generation"
```

---

### Task 5: Wire Up Download Buttons in Data Explorer

**Files:**
- Modify: `web/src/pages/data-explorer.tsx`

- [ ] **Step 1: Add import for ExportDataDialog and Download icon**

At the top of `data-explorer.tsx`, add to the lucide-react import (line 20):

Add `Download` to the existing import:
```typescript
import { Search, Database, Cpu, Puzzle, Workflow, Brain, History, Loader2, Eye, Download } from 'lucide-react'
```

Add the ExportDataDialog import after the other imports:
```typescript
import { ExportDataDialog } from '@/components/data/ExportDataDialog'
```

- [ ] **Step 2: Add export source state**

After the `selectedSource` state (around line 95), add:

```typescript
const [exportSource, setExportSource] = useState<UnifiedDataSourceInfo | null>(null)
```

- [ ] **Step 3: Add download button to desktop table actions**

In the `renderCell` function, modify the `actions` case (line 255-265). Replace the existing actions block:

```typescript
case 'actions':
  return (
    <div className="flex items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        className="h-7 px-2"
        onClick={(e) => { e.stopPropagation(); setSelectedSource(source) }}
      >
        <Eye className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="sm"
        className="h-7 px-2"
        onClick={(e) => { e.stopPropagation(); setExportSource(source) }}
      >
        <Download className="h-4 w-4" />
      </Button>
    </div>
  )
```

- [ ] **Step 4: Add download button to mobile card layout**

In the mobile card view (lines 296-303), find the single Eye `<Button>` and wrap it together with a new Download button in a `<div className="flex items-center gap-1">`:

```tsx
<div className="flex items-center gap-1">
  <Button
    variant="ghost"
    size="sm"
    className="h-7 px-2"
    onClick={(e) => { e.stopPropagation(); setSelectedSource(source) }}
  >
    <Eye className="h-4 w-4" />
  </Button>
  <Button
    variant="ghost"
    size="sm"
    className="h-7 px-2"
    onClick={(e) => { e.stopPropagation(); setExportSource(source) }}
  >
    <Download className="h-4 w-4" />
  </Button>
</div>
```

- [ ] **Step 5: Add ExportDataDialog rendering**

Before the closing `</>` of the component's return (before the closing fragment that wraps PageLayout, PageTabsBottomNav, and UnifiedFormDialog), add the ExportDataDialog:

```tsx
<ExportDataDialog
  open={!!exportSource}
  onOpenChange={(open) => !open && setExportSource(null)}
  source={exportSource}
/>
```

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd web && npx tsc --noEmit 2>&1 | grep -E "data-explorer|error" | head -20
```

Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/data-explorer.tsx
git commit -m "feat: add download buttons to data explorer rows"
```

---

### Task 6: Final Verification

- [ ] **Step 1: Full TypeScript check**

```bash
cd web && npx tsc --noEmit
```

Expected: No errors

- [ ] **Step 2: Build check**

```bash
cd web && npm run build 2>&1 | tail -20
```

Expected: Build succeeds

- [ ] **Step 3: Visual verification**

Start the dev server and verify:
1. Data Explorer page loads without errors
2. Download icon appears in each row's action column (desktop and mobile)
3. Clicking download opens the ExportDataDialog
4. Time range inputs show correct defaults (last 24h)
5. Clicking Export triggers a download (requires running backend with data)

```bash
cd web && npm run dev
```
