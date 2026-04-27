# NeoMind - Edge AI Platform for IoT

## Tech Stack
- **Backend**: Rust (Axum, event-driven)
- **Frontend**: React 18 + TypeScript + Tailwind CSS + Zustand
- **Desktop**: Tauri 2.x

## Development Commands

```bash
# Rust Backend (project root)
cargo build && cargo test && cargo run -p neomind-cli -- serve  # port: 9375

# Tauri Desktop (web/)
cd web && npm install && npm run tauri:dev

# Web Frontend (web/)
npm run dev && npm run build
```

## Project Structure

```
NeoMind/
├── crates/           # Rust workspace
│   ├── neomind-core/        # Core traits and types
│   ├── neomind-api/         # Web API server (Axum)
│   ├── neomind-agent/       # AI Agent with tool calling and LLM backends
│   ├── neomind-devices/     # Device management (MQTT)
│   ├── neomind-storage/     # Storage (redb)
│   ├── neomind-messages/    # Messaging system
│   ├── neomind-rules/       # Rule engine
│   ├── neomind-extension-sdk/     # Extension SDK
│   ├── neomind-extension-runner/  # Extension process isolation
│   └── neomind-cli/         # CLI tools
├── web/src/          # React frontend (components, pages, hooks, store, types)
├── docs/guides/      # User documentation (en/zh)
└── data/             # Runtime databases (telemetry.redb, sessions.redb, etc.)
```

## Key Rules

- **Ollama API**: Use `/api/chat` (native), NOT `/v1/chat/completions`
- **Tauri Environment**: API base is `http://localhost:9375/api`, WebSocket uses `ws://`
- **Time-series DB**: All metrics in `data/telemetry.redb`
- **DataSourceId Format**: `{type}:{id}:{field}` (e.g., `extension:weather:temp`)

## Code Conventions

- Rust: Follow standard Rust conventions, use `cargo fmt` and `cargo clippy`
- Frontend: ES modules, functional components, Zustand slices pattern
- Always run type checks after code changes

### Frontend UI Standards

- **Loading States**: All page-level loading must use **skeleton screens** (not spinners)
  - `ResponsiveTable`: built-in skeleton rows matching column structure (auto when `loading={true}`)
  - `LoadingState variant="page"`: card grid skeleton for non-table pages
  - Spinner (`Loader2`) only for inline/button/dialog-level loading, never for page content
- **Pagination**: Default page size is **10** across all pages (devices, agents, messages, data explorer, etc.)
- **Page Layout**: Use `PageLayout` with `PageTabsBar`/`PageTabsContent` pattern. Content grows naturally; `PageLayout`'s scroll container handles scrolling via `overflow-auto`
- **Fetch Deduplication**: Store-level `fetchCache` (TTL 10s) prevents redundant API calls. Pattern: `shouldFetch` → `markFetching` → API call → `markFetched`. Invalidate on mutations. WebSocket events use optimistic updates (`updateDeviceStatus`) instead of full refetch.

## Documentation

For detailed information, see:
- **API**: `docs/guides/en/14-api.md`
- **LLM**: `docs/guides/en/02-llm.md`
- **Agents**: `docs/guides/en/03-agent.md`
- **Devices**: `docs/guides/en/04-devices.md`
- **Storage**: `docs/guides/en/10-storage.md`
- **Extensions**: `docs/guides/en/extension-system.md`
