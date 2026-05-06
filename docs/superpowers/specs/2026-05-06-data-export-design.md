# Data Export Feature - Design Spec

## Summary

Add per-row data export to the Data Explorer page. Users click a download button on any data source row, pick a custom time range, and download the historical telemetry data as Excel (.xlsx) for text data or ZIP (.zip) for binary/image data.

## Scope

- Frontend-only: no backend changes required
- Single data source export (one row at a time)
- Reuses existing `GET /api/telemetry` endpoint

## User Flow

1. User clicks the download icon (`Download`) in a data source row's action column
2. `ExportDataDialog` opens showing:
   - Data source info (source name, field name, data type)
   - Start datetime input
   - End datetime input (defaults: end = now, start = 24h ago)
   - Export button (disabled while loading)
3. On export click:
   - Call `GET /api/telemetry?source=...&metric=...&start=...&end=...&limit=1000`
   - Generate file in-browser based on data type
   - Trigger browser download
4. Dialog closes after download starts

## Export Formats

| Data Type | Format | File Content |
|-----------|--------|-------------|
| float, integer, string, boolean, array | `.xlsx` | Columns: `Timestamp | Value | Quality` |
| binary (images, blobs) | `.zip` | Individual files + `manifest.json` |

**File naming**: `{source_name}_{field_name}_{YYYYMMDD_HHmmss}.xlsx` or `.zip`

## Frontend Changes

### New Dependencies

- `xlsx` - Excel file generation (lightweight, no server needed)
- `jszip` - ZIP file generation for binary data
- `@shadcn/ui/calendar` - Date picker component (needs to be added)
- `date-fns` - Likely already available; verify

### New Files

1. **`web/src/components/data/ExportDataDialog.tsx`** - Dialog component
   - Props: `open`, `onOpenChange`, `source: UnifiedDataSourceInfo`
   - Contains datetime range inputs, export logic, loading state
   - Uses `UnifiedFormDialog` for consistency

### Modified Files

1. **`web/src/pages/data-explorer.tsx`**
   - Add `Download` icon import from lucide-react
   - Add download button to each row's action column (desktop table + mobile card)
   - Add `ExportDataDialog` with state management
   - Parse `source.id` into `source`/`metric` parts for telemetry API call

2. **`web/src/i18n/locales/en/data.json`** - New i18n keys for the dialog
3. **`web/src/i18n/locales/zh/data.json`** - Chinese translations

### i18n Keys

```
data:export.title - "Export Data"
data:export.startTime - "Start Time"
data:export.endTime - "End Time"
data:export.button - "Export"
data:export.exporting - "Exporting..."
data:export.noData - "No data found for the selected time range"
data:export.format.excel - "Excel (.xlsx)"
data:export.format.zip - "ZIP (.zip)"
```

## Implementation Details

### Data Fetching

```typescript
// Parse DataSourceId: "device:sensor1:temperature" → source="device:sensor1", metric="temperature"
const parts = source.id.split(':')
const sourcePart = `${parts[0]}:${parts[1]}`
const metricPart = parts.slice(2).join(':')

const result = await api.queryTelemetry(sourcePart, metricPart, startUnix, endUnix, 1000)
```

### Excel Generation (text data)

Using `xlsx` library:
- Sheet name: field display name
- Header row: Timestamp, Value, Quality
- Data rows from telemetry response
- Auto-column-width

### ZIP Generation (binary data)

Using `jszip` library:
- `manifest.json`: metadata with source info, time range, data point count
- Individual files named by timestamp (binary values from API as base64-decoded blobs)
- Note: current API returns `<binary>` placeholder for binary values — if actual binary data is needed, backend enhancement may be required later. For now, export whatever value the API returns.

### Date Time Input

Use native `<input type="datetime-local">` styled consistently. This avoids adding a heavy date picker library and works well across browsers. The inputs are simple start/end datetime pickers — no need for a full calendar popover.

## Error Handling

- No data in range: show toast/notification, no file download
- API error: show error message in dialog
- Large datasets: loading spinner during generation, no progress bar needed (max 1000 points)
