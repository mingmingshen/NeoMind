# NeoMind - Edge AI Platform for IoT

## Tech Stack
- **Backend**: Rust (Axum, event-driven)
- **Frontend**: React 18 + TypeScript + Tailwind CSS + Zustand
- **Desktop**: Tauri 2.x

## Development Commands

```bash
# Rust Backend (project root)
cargo build && cargo test && cargo run -p neomind-api  # port: 9375

# Tauri Desktop (web/)
cd web && npm install && npm run tauri:dev

# Web Frontend (web/)
npm run dev && npm run build
```

## Project Structure

```
NeoMind/
├── crates/           # Rust workspace (api, agent, llm, core, devices, storage, memory)
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

## Documentation

For detailed information, see:
- **API**: `/api/docs` (Swagger) or `docs/guides/en/14-api.md`
- **LLM**: `docs/guides/en/02-llm.md`
- **Agents**: `docs/guides/en/03-agent.md`
- **Devices**: `docs/guides/en/04-devices.md`
- **Storage**: `docs/guides/en/10-storage.md`
- **Extensions**: `docs/guides/en/16-extension-dev.md`
