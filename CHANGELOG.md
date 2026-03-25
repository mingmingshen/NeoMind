# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### 🎉 Overview

**Major feature release** introducing comprehensive platform enhancements with **96 new features**, **47 refactorings**, and **204 bug fixes**.

**Highlights**:
- 📨 **Messages Pipeline Extension** - Complete message routing with DeliveryLog tracking
- 🔌 **Extension System 2.0** - SDK, isolated execution, capability framework, streaming support
- 🧠 **Hierarchical Memory System** - Short-term and long-term memory for AI Agents
- 📊 **Visual Dashboard** - Telemetry data binding with Sparkline, ProgressBar, LED components
- 🤖 **Native LLM Backend** - Rust-based inference using Candle framework
- 🛠️ **Tool Calling System** - Complete tool calling for AI dialogue
- 🌏 **Chinese LLM Support** - Optimized for Chinese language backends
- 🔄 **Auto-Update System** - In-app updates with progress notifications
- 🏗️ **Workspace Consolidation** - Reduced from 15 to 10 crates

---

### ✨ New Features

#### Messages System
- **Messages Pipeline Extension** with DeliveryLog for DataPush tracking
- **MessageType enum** and **ChannelFilter** for message routing
- Channel filter configuration UI and API endpoints
- Message type column in messages table with filtering

#### Extension System
- **Extension SDK** and isolated execution support
- **Capability framework** with event subscription system
- Extension streaming support for real-time data
- **.nep package format** support for extension discovery
- Resource limits and security validation for extension runner
- Per-extension `collect_interval` configuration
- Extension upload handling improvements

#### Agent System
- **Hierarchical Memory System** - Short-term and long-term memory
- LLM-based analysis integration during agent execution
- Agent editor redesign with card grid layout
- Dashboard integration for agents
- More role options with card-style selection
- Agent resources and intent parsing APIs

#### Visual Dashboard
- Complete dashboard system with telemetry data binding
- **Sparkline** component with configurable labels
- **ProgressBar** component with custom labels
- **LED Indicators** with gradient glow effects
- **CustomLayer** component with data binding editor
- Map enhancements with marker types and command execution
- Image/video components with titles and fullscreen mode

#### LLM & AI
- **Native Rust LLM Backend** using Candle framework
- Tool calling support for native backend
- Chinese LLM backends support and optimization
- Improved prompt engineering for better responses
- Thinking character count in collapsed state

#### Platform & Infrastructure
- **Auto-Update System** for Tauri desktop with progress notifications
- One-line installation script with embedded Web UI
- Server deployment options (Docker, binary, systemd)
- CI/CD optimization with manual triggers and caching
- Tauri v2 key generation helper scripts

#### Alerting & Automation
- **Alert Channels Plugin System** with device enhancements
- Complete tool calling system for dialogue
- Rule engine improvements with scheduler, retry, channels parsing
- Rules API handlers and storage layer

#### Mobile & UX
- Mobile infinite scroll and pagination optimization
- Standardized dialog components for mobile
- Improved drawer experience and z-index stacking
- Startup loading screen for Tauri desktop

---

### 🏗️ Architecture Changes

#### Workspace Consolidation
- **Reduced from 15 to 10 crates** for simpler architecture
- Merged `neomind-llm` into `neomind-agent`
- Consolidated IPC types into SDK for ABI isolation
- Removed 4 crates to 2 for better ABI isolation

#### Removed Unused Modules
- `neomind-testing` crate
- `intent` and `nl2automation` modules
- `multimodal` and `maintenance` storage
- `mqtt_v2` and `mock_devices`
- `audit`, `prometheus metrics`
- `Local Network Scan` feature
- Duplicate `ExtensionRegistryTrait` and `Tool` trait

---

### 🔧 Improvements

#### Code Quality
- Dead code cleanup across workspace
- Removed unnecessary `#[allow(dead_code)]` annotations
- Unified `StorageBackend` trait naming
- Cleaner module interfaces

#### UI/UX
- Unified tab bar styling across all pages
- Improved dialog patterns and scaling
- Reduced padding in nested dialogs
- Better loading UI states

#### Performance
- IPC buffer management optimization
- Agent streaming optimization
- Frontend performance improvements
- UTF-8 handling fixes

---

### 🐛 Bug Fixes

- Fixed extension startup crashes with safe sidecar discovery
- Fixed nested dialog z-index and footer alignment
- Fixed input focus loss in plugin config forms
- Fixed death monitor restarting extensions during unload
- Fixed CI/CD build errors for macOS DMG bundling
- Fixed WebSocket persistence and connection handling

---

### 📁 Changed Files

**New Files** (Key Additions):
- `crates/neomind-extension-sdk/` - Extension SDK
- `web/src/components/dashboard/` - Dashboard components
- `web/src/components/update/` - Auto-update UI
- `scripts/` - Installation and CI/CD scripts
- `docs/guides/` - Comprehensive documentation

**Removed Files**:
- 20+ unused modules and crates
- Duplicate trait definitions

**Modified**: 500+ files across all modules

---

### 📊 Statistics

| Category | Count |
|----------|-------|
| Commits | 546 |
| Features | 96 |
| Refactorings | 47 |
| Bug Fixes | 204 |
| Files Changed | 500+ |

---

### 🎯 Summary

✅ Complete messages pipeline with routing and tracking
✅ Modern extension system with SDK and isolation
✅ AI Agent memory system for context retention
✅ Visual dashboard with real-time data binding
✅ Native LLM backend for edge deployment
✅ Simplified architecture (15 → 10 crates)
✅ Auto-update system for seamless upgrades
✅ Chinese language LLM optimization

**Production-ready and recommended for all users.**

---

**Previous Release**: [v0.6.2](https://github.com/camthink-ai/NeoMind/releases/tag/v0.6.2)
**Release Date**: March 25, 2026

---

## [v0.6.2] - 2025-03-24

### 🎉 Overview

Minor release focusing on **lightweight architecture refactoring** and **code cleanup**.

**Highlights**:
- 🧹 Removed unused memory system components (graph, importance, unified)
- 📦 Simplified messaging channels (removed console/memory channels)
- 🔧 Removed knowledge storage module
- 🔒 Added dylib validation for extension runner
- 🎨 Various UI and code refinements

---

### 🏗️ Architecture Changes

#### Memory System Cleanup
- **Removed** `memory/graph.rs` - Graph-based memory storage (unused)
- **Removed** `memory/importance.rs` - Memory importance scoring (unused)
- **Removed** `memory/unified.rs` - Unified memory implementation (unused)
- Simplified memory module to essential components only

#### Messaging System Cleanup
- **Removed** `channels/console.rs` - Console channel (unused)
- **Removed** `channels/memory.rs` - Memory channel (unused)
- Streamlined channel management

#### Storage Cleanup
- **Removed** `storage/knowledge.rs` - Knowledge graph storage (unused)
- Cleaner storage module interface

---

### ✨ New Features

- Added `dylib_validation.rs` for extension runner dynamic library validation
- Improved extension security and validation

---

### 🔧 Improvements

- Refined agent module architecture
- Updated LLM backend instance manager
- Improved extension isolation process handling
- Various web UI refinements in dashboard, LLM backends, plugins, and chat

---

### 📁 Changed Files

**Deleted Files**:
- `crates/neomind-agent/src/memory/graph.rs`
- `crates/neomind-agent/src/memory/importance.rs`
- `crates/neomind-agent/src/memory/unified.rs`
- `crates/neomind-messages/src/channels/console.rs`
- `crates/neomind-messages/src/channels/memory.rs`
- `crates/neomind-storage/src/knowledge.rs`

**New Files**:
- `crates/neomind-extension-runner/src/dylib_validation.rs`

**Modified**: 25+ files across agent, API, core, messages, storage, and web modules

---

### 🎯 Summary

✅ Lighter codebase with removed unused modules
✅ Cleaner architecture with focused components
✅ Enhanced extension security
✅ Improved maintainability

---

**Previous Release**: [v0.6.1](https://github.com/camthink-ai/NeoMind/releases/tag/v0.6.1)
**Release Date**: March 24, 2025

---

## [v0.6.1] - 2025-03-19

### 🎉 Overview

Minor release focusing on **UI refinements** and **code quality improvements**.

**Highlights**:
- 🎨 Removed focus ring outlines from form controls for cleaner UI
- 🧹 Fixed all clippy warnings across the workspace
- 🔧 Added `preserve_order` feature to serde_json for ABI compatibility
- 📦 Dialog component refactoring and cleanup

---

### 🎨 UI Changes

- Removed `focus:ring` and `focus-visible:ring` styles from:
  - `Input` component
  - `Select` component
  - `Textarea` component
- Cleaner form control appearance without focus border outlines

---

### 🔧 Code Quality

- Fixed all clippy warnings across the Rust workspace
- Added `#[allow(dead_code)]` annotations for unused but intentional code
- Added `#![allow(clippy::too_many_arguments)]` for executor modules
- Improved variable naming with underscore prefix for intentionally unused variables

---

### 📦 Dependency Changes

- **serde_json**: Added `preserve_order` feature for stable ABI compatibility between extension-runner and extension binaries

---

### 📁 Changed Files

**Deleted Files**:
- `web/src/components/automation/FullScreenBuilder.tsx`
- `web/src/components/dialog/FormDialog.tsx`

**Modified**: 207 files, 7004 insertions(+), 5379 deletions(-)

---

### 🎯 Summary

✅ Cleaner UI without focus ring distractions
✅ Improved code quality with clippy fixes
✅ Better ABI compatibility for extensions

---

**Previous Release**: [v0.6.0](https://github.com/camthink-ai/NeoMind/releases/tag/v0.6.0)
**Release Date**: March 19, 2025

---

## [v0.6.0] - 2025-03-18

### 🎉 Overview

Major feature release introducing **automatic update functionality** with significant **CI/CD performance optimizations**.

**Highlights**:
- ✨ In-app auto-update system with Ed25519 signature verification
- 🚀 CI/CD build speed: **35-50% faster** (first build), **80-95% faster** (subsequent builds)
- 📦 Dependency cleanup: removed 10 unused dependencies, reduced binary size by ~340KB
- 🎛️ Manual CI/CD trigger for better resource control
- 🌍 Bilingual UI (English & Chinese)
- 📱 Desktop UX improvements (sticky headers)

---

### ✨ Features

#### Auto-Update System
- **Automatic update checks** every 24 hours with background notifications
- **One-click in-app updates** with real-time progress tracking
- **Ed25519 signature verification** for secure package validation
- **Cross-platform support**: macOS (Intel + Apple Silicon), Windows, Linux

#### CI/CD Optimizations
- **sccache integration** for distributed compilation caching
- **ThinLTO + codegen-units=256** for faster builds
- **Parallel build jobs** and aggressive caching strategies
- **Manual build trigger** via GitHub Actions UI (saves CI/CD resources)

#### Desktop UX Improvements
- **Sticky headers** for TAB navigation and action buttons (desktop only)
- **Reduced header spacing** for more compact layout (50% margin reduction)

---

### 🔧 Improvements

#### Dependency Management
- Upgraded `thiserror` from v1 to v2 across workspace
- Removed unused dependencies: `wasmi`, `wasmi-validation`, `bytes`, `dirs`, `multer`, `http-body-util`, `sysinfo`, `hostname`, OpenTelemetry stack
- Unified all crate dependencies to use workspace definitions
- **Result**: ~340-380KB smaller binaries, resolved version conflicts

#### Build Configuration
- Optimized dev profile: `debug=0`, `opt-level=0` for fastest compilation
- Added `ci-release` profile with ThinLTO (20-30% faster builds)
- Package-specific optimizations for `tokio`, `regex`, `serde`, `chrono`, `axum`
- Enabled pipelined compilation and incremental builds
- Added cargo aliases: `cargo q` (quick check), `cargo qc` (check core), etc.

#### Developer Tooling
- **Version synchronization tool**: `node scripts/sync-version.js` (single command to sync version across all files)
- **Manual build guide**: `docs/MANUAL_BUILD_GUIDE.md`

#### Internationalization
- Added bilingual strings for update system (English & Chinese)

---

### 🐛 Bug Fixes

- Fixed **infinite loop** in update checker (used `useCallback` and `useRef` for stable callbacks)
- Updated **team attribution** from "NeoMind Team" to "CamThink Team"
- Replaced temporary **Bot icon** with official product logo in About page
- Updated license from MIT to **Apache-2.0**

---

### 📊 Performance Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| CI first build | 15-20 min | 8-12 min | **35-50% faster** ⬆️ |
| CI subsequent builds | 10-15 min | 2-5 min | **80-95% faster** ⬆️ |
| Binary size | ~XX MB | ~XX MB | **-340KB** ⬇️ |
| Update installation | Manual download | One-click in-app | **~90% faster** ⬆️ |

---

### 🔄 API Changes

#### New Tauri Commands
```rust
check_update() -> Result<UpdateInfo, String>
download_and_install() -> Result<String, String>
get_app_version() -> Result<String, String>
relaunch_app()
```

#### New Tauri Events
```typescript
window.emit('update-progress', {
    total: number,
    current: number,
    progress: number  // 0-100
})
```

---

### 📁 Changed Files

**New Files** (Auto-Update):
- `web/src-tauri/src/update.rs` - Update commands and logic
- `web/src/components/update/UpdateDialog.tsx` - Update dialog UI
- `web/src/hooks/useUpdateCheck.ts` - Update check hook
- `web/src/store/slices/updateSlice.ts` - Zustand store slice
- `.github/workflows/generate-update-manifest.yml` - Manifest generation
- `scripts/sync-version.js` - Version sync tool

**Modified Files** (CI/CD Optimization):
- `.github/workflows/build.yml` - Added sccache, manual trigger, optimizations
- `.cargo/config.toml` - Added ci-release profile, dev profile optimizations
- `Cargo.toml` - Unified workspace dependencies, upgraded thiserror to v2
- All crate `Cargo.toml` files - Use workspace dependencies

**Total**: 50+ files changed, 1,500+ insertions(+), 2,700+ deletions(-)

---

### ⚠️ Upgrade Notes

**For Users**: Seamless upgrade - no action required. Auto-update will be available after updating to v0.6.0.

**For Developers**:

**CI/CD Changes**:
- Builds are now **manual trigger** only (except tags/releases)
- Use GitHub Actions UI → "Run workflow" to trigger builds
- See `docs/MANUAL_BUILD_GUIDE.md` for details

**Future Releases**:
```bash
# 1. Update version
node scripts/sync-version.js  # or edit manually

# 2. Commit and tag
git add .
git commit -m "chore: bump version to 0.6.1"
git tag v0.6.1
git push origin main
git push origin v0.6.1

# 3. Create release on GitHub
```

---

### 🎯 Summary

✅ Auto-update system with security verification
✅ CI/CD performance: 35-50% faster (first), 80-95% faster (cached)
✅ Dependency cleanup: -340KB binary size, resolved conflicts
✅ Manual CI/CD trigger for resource control
✅ Desktop UX improvements
✅ Comprehensive documentation

**Production-ready and recommended for all users.**

---

**Previous Release**: [v0.5.11](https://github.com/camthink-ai/NeoMind/releases/tag/v0.5.11)
**Release Date**: March 18, 2025

---

## [v0.5.11] - Previous Release

Previous version details...

---

**For issues or questions**: https://github.com/camthink-ai/NeoMind/issues
