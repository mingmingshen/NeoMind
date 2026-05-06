# Data Export Feature - Design Spec

## Summary

Add per-row data export to the Data Explorer page. Users click a download button on any data source row, pick a custom time range, and download the historical telemetry data as Excel (.xlsx) for text data or ZIP (.zip) for binary/image data.

## Scope

- Frontend-only: no backend changes required
- Single data source export (one row at a time)
- Reuses existing `GET /api/telemetry` endpoint (max 1000 points per query)

## User Flow

1. User clicks the download icon (`Download`) in a data source row's action column (desktop table and mobile card)
2. `ExportDataDialog` opens showing:
   - Data source info (source name, field name, data type, format indicator)
   - Start datetime input (default: 24h ago)
   - End datetime input (default: now)
   - Export button (disabled while loading)
3. On export click:
   - Call `GET /api/telemetry?source=...&metric=...&start=...&end=...&limit=1000`
   - All timestamps in the API are **Unix seconds** (not milliseconds)
   - Check `total_count` from response; if it exceeds returned data count, show warning toast
   - Generate file in-browser based on data type
   - Trigger browser download
4. Dialog closes after download starts

## Export Format Mapping

| `data_type` | Export Format | File Content |
|-------------|--------------|-------------|
| `float`, `integer`, `boolean`, `string`, `array`, `unknown`, `null` | `.xlsx` | Columns: `Timestamp | Value | Quality` |
| `binary` | `.zip` | Individual files + `manifest.json` (see Binary Limitations below) |

**Binary limitation**: The current telemetry API returns `<binary>` as a string placeholder for binary values (see `data.rs` line 212: `MetricValue::Binary(_) => serde_json::json!("<binary>")`). The ZIP export will include a `manifest.json` with metadata and timestamps, but actual binary file contents will note this limitation. If the user attempts to export binary data, show a toast: "Binary data export is partially supported — actual file contents may not be available." This can be enhanced later with a dedicated binary download API.

**File naming**: `{source_name}_{field_name}_{YYYYMMDD_HHmmss}.xlsx` or `.zip`
- Sanitize names: replace spaces and colons with underscores, remove other special characters

## Frontend Changes

### New Dependencies

- `xlsx` - Excel file generation (lightweight, no server needed)
- `jszip` - ZIP file generation for binary data

No date picker library needed — use native `<input type="datetime-local">` styled consistently. No `date-fns` needed — use native `Date` APIs matching existing patterns (`formatTime`, `formatDateTime` in data-explorer.tsx).

### New Files

1. **`web/src/components/data/ExportDataDialog.tsx`** - Dialog component
   - Props: `open: boolean`, `onOpenChange: (open: boolean) => void`, `source: UnifiedDataSourceInfo`
   - Uses `UnifiedFormDialog` with:
     - `submitLabel={t('data:export.button')}`
     - `onSubmit={handleExport}` — async, calls API and generates file
     - `showCancelButton={true}`
   - Internal state: `startTime: Date`, `endTime: Date`, `exporting: boolean`
   - AbortController for cancelling in-flight requests on unmount or dialog close (follows existing pattern in data-explorer.tsx)

### Modified Files

1. **`web/src/pages/data-explorer.tsx`**
   - Add `Download` icon import from lucide-react
   - Add download button to desktop table action column (alongside existing Eye button)
   - Add download button to mobile card layout (alongside existing Eye button)
   - Add `exportSource` state + `ExportDataDialog` rendering
   - Parse `source.id` into `source`/`metric` parts for telemetry API call

2. **`web/src/lib/api.ts`**
   - Fix `queryTelemetry` return type: change `value: number` to `value: unknown` (backend returns any JSON value, not just numbers)

3. **`web/src/i18n/locales/en/data.json`** - New i18n keys
4. **`web/src/i18n/locales/zh/data.json`** - Chinese translations

### i18n Keys

```json
{
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

## Implementation Details

### Data Fetching

```typescript
// Parse DataSourceId: "device:sensor1:temperature" → source="device:sensor1", metric="temperature"
const parts = source.id.split(':')
const sourcePart = `${parts[0]}:${parts[1]}`
const metricPart = parts.slice(2).join(':')

// Convert datetime-local values to Unix seconds
const startUnix = Math.floor(startTime.getTime() / 1000)
const endUnix = Math.floor(endTime.getTime() / 1000)

const result = await api.queryTelemetry(sourcePart, metricPart, startUnix, endUnix, 1000)

// Check for truncation
if (result.total_count > result.data.length) {
  toast.info(t('data:export.truncated', { count: result.data.length, total: result.total_count }))
}
```

### Excel Generation (text data)

Using `xlsx` library:
- Sheet name: field display name (sanitized)
- Header row: Timestamp, Value, Quality
- Timestamps: convert Unix seconds to `YYYY-MM-DD HH:mm:ss` format using native `Date`
- Data rows from telemetry response
- Auto-column-width

### ZIP Generation (binary data)

Using `jszip` library:
- `manifest.json`: source info, time range, data point count, note about binary limitation
- Data points listed with timestamps (no actual binary file contents due to API limitation)

### Date Time Input

Native `<input type="datetime-local">` with default values:
- End: current time
- Start: 24 hours before end
- Use `step="1"` for second-level precision

## Error Handling

- No data in range: show inline message in dialog (no toast)
- API error: show error message inline in dialog
- Data truncation (total_count > returned count): show warning toast with counts
- Binary data: show informational toast about partial support
- Dialog close during export: abort via AbortController
