# Migration Guide: v0.6 → v0.7

## Overview

v0.7.0 is a quality-focused release with stability improvements, a complete UI design system overhaul, and comprehensive testing. There are no breaking API changes from v0.6.12.

## What's New

### Backend Stability

- **Error Handling** — 1000+ hot-path `unwrap()` calls replaced with safe error propagation across 8 crates, preventing production panics
- **API Input Validation** — All POST/PUT endpoints validate parameters before processing, returning 400 on invalid input
- **Settings Persistence** — Settings saved to `data/settings.redb`, survive server restarts
- **MQTT Unsubscription** — Custom MQTT topics can now be unsubscribed
- **Rule Engine Recovery** — Catch-all error recovery prevents scheduler crashes

### UI Design System

- **Aurora Background & Glass Morphism** — App-wide aurora gradient background with glass-style navigation and footer
- **OKLCH Color System** — CSS color tokens migrated from HSL to OKLCH for perceptually uniform colors
- **Design Tokens** — All hardcoded Tailwind palette colors replaced with semantic design tokens (text-success, bg-error-light, text-accent-orange, etc.)
- **Harmonized Accent Colors** — OKLCH-based category accents (purple, orange, teal, rose) with consistent light/dark variants
- **Typography** — Plus Jakarta Sans (Latin) + Noto Sans SC (CJK) fonts
- **Chart Palette** — Redesigned chart colors with better visual distinction and accessibility
- **UnifiedFormDialog** — Centralized dialog component with consistent z-index, portal, and backdrop handling
- **Design Specification** — Comprehensive `DESIGN_SPEC.md` documenting all UI patterns and conventions

### Frontend Polish

- **Skeleton Loading** — Consistent skeleton screens replace spinners on all pages
- **Pagination** — Standardized to 10 items per page across all lists
- **Toast Notifications** — All `alert()` dialogs replaced with styled toast notifications
- **Form Validation** — Agent, device, and rule editors validate input in real-time with inline error messages
- **Confirmation Dialogs** — Destructive operations (delete, reset) require explicit confirmation
- **Error Boundaries** — React Error Boundaries catch page render failures gracefully
- **Error Messages** — User-friendly messages for API failures instead of raw error text
- **Empty States** — All list pages show helpful guidance when empty
- **29 Dialog Migrations** — All form dialogs consolidated to UnifiedFormDialog
- **Dark Mode Fixes** — Dialog borders visible, z-index stacking correct for nested dialogs

### Performance

- Eliminated API polling storms, N+1 queries, and blocking call chains
- TTL-based fetch deduplication prevents redundant API calls
- Dashboard components load without error on first page visit

### Testing

- Comprehensive unit tests added to neomind-agent, neomind-storage, neomind-rules, neomind-messages, neomind-extension-runner, and neomind-api

## Breaking Changes

None. v0.7.0 is fully backward-compatible with v0.6.x.

## Configuration Changes

- Settings are now persisted in `data/settings.redb` (automatic migration)
- Event trigger cooldown default changed from 5s to 60s (configurable via settings)

## API Changes

### New Responses

- All POST/PUT endpoints now return `400 Bad Request` with descriptive error messages on invalid input

### Unchanged

- All existing endpoints maintain backward compatibility
- No removed endpoints or changed response formats
