# Dashboards

> Build real-time monitoring dashboards with drag-and-drop widgets, live data binding, and public sharing.

---

## Opening a Dashboard

1. Click **Dashboard** in the top navigation bar (desktop) or scrollable tabs (mobile)
2. If you have existing dashboards, the default one opens automatically
3. To switch dashboards, use the sidebar on the left:
   - Click any dashboard name to open it
   - The active dashboard is highlighted
   - Each entry shows the widget count

If no dashboards exist yet, you will see an empty state prompting you to create one.

![Dashboard overview](../../img/dashboard_light.png)

> Numbered annotations:
> 1. Sidebar with dashboard list (collapse/expand with the chevron button)
> 2. Dashboard name in the header bar
> 3. **Edit** button (pencil icon) to enter layout editing mode
> 4. **Share** button to manage public links
> 5. Widget grid with live-updating data

---

## Creating a Dashboard

1. Open the sidebar by clicking the dashboard icon or swiping from the left edge (mobile)
2. Click the **"+ New Dashboard"** button at the bottom of the list
3. Type a name and press Enter or click the checkmark
4. The new dashboard opens immediately with an empty canvas

You can rename a dashboard at any time by clicking the pencil icon next to its name in the sidebar. To delete a dashboard, click the trash icon (a confirmation dialog will appear; the last remaining dashboard cannot be deleted).

---

## Adding Widgets

1. Click the **Edit** button (pencil icon) in the header bar to enter edit mode
2. Click the **"+ Add Widget"** button that appears in the header
3. The Widget Library panel slides in from the left, showing all available widgets organized by category:

| Category | Widgets |
|----------|---------|
| **Indicators** | Value Card, LED Indicator, Sparkline, Progress Bar |
| **Charts** | Line Chart, Area Chart, Bar Chart, Pie Chart |
| **Controls** | Command Button |
| **Display** | Image Display, Image History, Web Display, Markdown Display |
| **Spatial** | Map Display, Video Display, Custom Layer |
| **Business** | Agent Monitor, AI Analyst |

4. Use the search bar or category tabs to find the widget you need
5. Click a widget to add it to the dashboard -- it appears at the next available grid position with a default size

---

## Configuring a Widget

After adding a widget (or clicking an existing widget in edit mode), the configuration panel slides in from the right with these sections:

### Title

Set the display name shown in the widget header.

### Data Source

Bind the widget to live data. The data source selector provides three dropdowns:

1. **Source Type** -- choose from: Device, Telemetry, Metric, Command, Device Info, Extension, Extension Metric, Extension Command, System, Transform, AI Metric, Agent
2. **Entity** -- select the specific device or extension from a dropdown (auto-populated from your connected devices and installed extensions)
3. **Property/Metric** -- select or type the metric name (e.g., `temperature`, `humidity`)

Common source types:

| Source Type | Use Case | Example Binding |
|-------------|----------|-----------------|
| Device | Read a metric from a connected device | Device: `sensor-01` > `temperature` |
| Telemetry | Time-series history of a device metric | Telemetry: `sensor-01` > `humidity` |
| Extension | Data from an installed extension | Extension: `weather` > `temp` |
| System | Platform health metrics | System: `memory_percent` |

For device sources, the metric dropdown auto-populates from the device type template. If no template is defined, you can type the metric name manually.

### Display Options (for generic widgets)

- **Unit** -- suffix displayed after the value (e.g., "C", "%", "kPa")
- **Format** -- number format string (e.g., "0.00" for two decimal places)
- **Min / Max** -- axis range for chart widgets, or range for progress bars
- **Size** -- Small, Medium, or Large preset
- **Color** -- CSS color value for the widget accent
- **Prefix** -- text displayed before the value (e.g., "$")

### Actions (for generic widgets)

Configure click actions on the widget, such as navigating to a device detail page or triggering a command.

---

## Arranging the Layout

In edit mode, you can freely arrange widgets on the grid:

- **Move**: Click and drag any widget to reposition it
- **Resize**: Drag the corner or edge handles to change widget dimensions; grid snapping ensures clean alignment
- **Configure**: Click a widget to open its configuration panel
- **Delete**: Click the close button in the widget's top-right corner

Layout changes are saved automatically. The dashboard syncs to the server with a short debounce (500 ms), so rapid drag operations are batched into a single save.

To exit edit mode, click the **Done** button (checkmark icon) in the header bar.

---

## Sharing a Dashboard

1. Open the dashboard you want to share
2. Click the **Share** button in the header bar
3. The Share Manager dialog opens, showing existing share links (if any)
4. Click **"+ Create Link"** to generate a new share link
5. Configure the link:
   - **Permission**: Toggle between **Read-only** (viewers see live data but cannot interact) and **Interactive** (viewers can adjust time ranges and hover for details)
   - **Expiration**: Choose from Never, 1 hour, 24 hours, 3 days, 7 days, or 30 days
6. Click **Create** to generate the link
7. Copy the share URL from the list -- anyone with this link can view the dashboard without logging in

### Managing Share Links

The Share Manager lists all active links with their permission level, expiration date, and the full URL. For each link you can:

- **Copy** the URL using the copy button
- **Revoke** access using the trash button (the link immediately stops working)

Shared dashboards poll for live data every 30 seconds. Interactive mode allows widget interaction (time range changes, hover tooltips) while read-only mode shows a static live-updating view.

---

## Dark Mode

NeoMind follows your system theme preference. Toggle between light and dark mode using the **sun/moon icon** in the top navigation bar, accessible from any page.

![Dashboard in dark mode](../../img/dashboard_dark.png)

---

## Tips

- Start with **Value Card** widgets for key metrics, then add **Line Charts** for trends
- Use **Sparkline** for compact inline trends alongside value displays
- Group related widgets together (e.g., all greenhouse sensors in one area)
- The **AI Analyst** widget provides an inline chat for exploring dashboard data
- **Agent Monitor** widgets let you track AI agent execution status directly on the dashboard
- Share dashboards with **Interactive** mode for kiosks and large screens, **Read-only** for reports

---

[Previous: AI Agents](06-agents.md) | [Index](README.md) | [Next: Notifications](08-notifications.md)
