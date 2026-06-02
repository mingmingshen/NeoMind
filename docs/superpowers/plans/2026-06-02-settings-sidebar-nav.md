# Settings Sidebar Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Settings page horizontal tabs with a sidebar navigation (desktop) and card-based drill-down list (mobile).

**Architecture:** The main `settings.tsx` switches from `PageTabsBar`/`PageTabsContent`/`PageTabsBottomNav` to a split layout: `SettingsNav` sidebar + content area on desktop, `SettingsSectionList` card list + drill-down view on mobile. All existing section components remain unchanged.

**Tech Stack:** React 18, TypeScript, Tailwind CSS, lucide-react, `useIsMobile()` from `@/hooks/useMobile`

**Spec:** `docs/superpowers/specs/2026-06-02-settings-sidebar-nav-design.md`

---

## File Structure

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `web/src/pages/settings/SettingsNav.tsx` | Desktop sidebar nav component |
| Create | `web/src/pages/settings/SettingsSectionList.tsx` | Mobile card-list drill-down component |
| Modify | `web/src/pages/settings.tsx` | Main page: replace tabs with sidebar/section-list |
| Unchanged | `web/src/pages/settings/PreferencesTab.tsx` | — |
| Unchanged | `web/src/pages/settings/AboutTab.tsx` | — |
| Unchanged | `web/src/components/llm/UnifiedLLMBackendsTab.tsx` | — |
| Unchanged | `web/src/components/connections/UnifiedDeviceConnectionsTab.tsx` | — |

---

### Task 1: Create `SettingsNav` (desktop sidebar)

**Files:**
- Create: `web/src/pages/settings/SettingsNav.tsx`

- [ ] **Step 1: Write the `SettingsNav` component**

Create `web/src/pages/settings/SettingsNav.tsx`:

```tsx
import { ReactNode } from "react"
import { cn } from "@/lib/utils"
import { Cpu, Plug, Sliders, Info } from "lucide-react"
import { useTranslation } from "react-i18next"

export type SettingsSection = "llm" | "connections" | "preferences" | "about"

export interface SettingsSectionConfig {
  value: SettingsSection
  label: string
  icon: ReactNode
}

export function getSettingsSections(t: ReturnType<typeof useTranslation>["t"]): SettingsSectionConfig[] {
  return [
    { value: "llm", label: t("settings:llmBackends"), icon: <Cpu className="h-4 w-4" /> },
    { value: "connections", label: t("settings:deviceConnections"), icon: <Plug className="h-4 w-4" /> },
    { value: "preferences", label: t("settings:preferences"), icon: <Sliders className="h-4 w-4" /> },
    { value: "about", label: t("settings:about"), icon: <Info className="h-4 w-4" /> },
  ]
}

interface SettingsNavProps {
  sections: SettingsSectionConfig[]
  activeSection: SettingsSection
  onSectionChange: (section: SettingsSection) => void
}

export function SettingsNav({ sections, activeSection, onSectionChange }: SettingsNavProps) {
  return (
    <nav
      className="w-52 shrink-0 hidden md:block"
      role="tablist"
      aria-label="Settings sections"
    >
      <div className="space-y-1">
        {sections.map((section) => {
          const isActive = activeSection === section.value
          return (
            <button
              key={section.value}
              role="tab"
              aria-selected={isActive}
              onClick={() => onSectionChange(section.value)}
              className={cn(
                "flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                "border-l-2",
                isActive
                  ? "bg-muted border-primary text-foreground"
                  : "border-transparent text-muted-foreground hover:bg-muted-50 hover:text-foreground"
              )}
            >
              <span className="shrink-0">{section.icon}</span>
              <span>{section.label}</span>
            </button>
          )
        })}
      </div>
    </nav>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -30`
Expected: No errors referencing `SettingsNav.tsx`

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/settings/SettingsNav.tsx
git commit -m "feat(settings): add SettingsNav desktop sidebar component"
```

---

### Task 2: Create `SettingsSectionList` (mobile)

**Files:**
- Create: `web/src/pages/settings/SettingsSectionList.tsx`

- [ ] **Step 1: Write the `SettingsSectionList` component**

Create `web/src/pages/settings/SettingsSectionList.tsx`:

```tsx
import { cn } from "@/lib/utils"
import { ChevronRight } from "lucide-react"
import { SettingsSection, SettingsSectionConfig } from "./SettingsNav"

interface SettingsSectionListProps {
  sections: SettingsSectionConfig[]
  onSectionSelect: (section: SettingsSection) => void
}

export function SettingsSectionList({ sections, onSectionSelect }: SettingsSectionListProps) {
  return (
    <div className="md:hidden space-y-1" role="list" aria-label="Settings sections">
      {sections.map((section) => (
        <button
          key={section.value}
          role="listitem"
          onClick={() => onSectionSelect(section.value)}
          className={cn(
            "flex w-full items-center gap-3 rounded-lg px-4 py-3 text-left transition-colors",
            "bg-card hover:bg-muted-50 active:scale-[0.99]"
          )}
        >
          <span className="shrink-0 text-muted-foreground">{section.icon}</span>
          <span className="flex-1 text-sm font-medium">{section.label}</span>
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        </button>
      ))}
    </div>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -30`
Expected: No errors referencing `SettingsSectionList.tsx`

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/settings/SettingsSectionList.tsx
git commit -m "feat(settings): add SettingsSectionList mobile component"
```

---

### Task 3: Rewrite `settings.tsx` to use sidebar + section list

**Files:**
- Modify: `web/src/pages/settings.tsx` (full rewrite)

- [ ] **Step 1: Rewrite `settings.tsx`**

Replace the entire content of `web/src/pages/settings.tsx` with:

```tsx
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { useSearchParams } from "react-router-dom"
import { useStore } from "@/store"
import { useIsMobile } from "@/hooks/useMobile"
import { PageLayout } from "@/components/layout/PageLayout"
import { Button } from "@/components/ui/button"
import { ArrowLeft } from "lucide-react"
import { AboutTab } from "./settings/AboutTab"
import { PreferencesTab } from "./settings/PreferencesTab"
import { UnifiedLLMBackendsTab } from "@/components/llm/UnifiedLLMBackendsTab"
import { UnifiedDeviceConnectionsTab } from "@/components/connections"
import {
  SettingsNav,
  SettingsSection as SettingsSectionType,
  getSettingsSections,
} from "./settings/SettingsNav"
import { SettingsSectionList } from "./settings/SettingsSectionList"

type MobileView = "list" | "section"

export function SettingsPage() {
  const { t } = useTranslation(["common", "settings", "extensions"])
  const isMobile = useIsMobile()
  const [searchParams, setSearchParams] = useSearchParams()
  const sectionFromUrl = searchParams.get("tab") as SettingsSectionType | null

  const validSections: SettingsSectionType[] = ["llm", "connections", "preferences", "about"]
  const [activeSection, setActiveSection] = useState<SettingsSectionType>(() => {
    return sectionFromUrl && validSections.includes(sectionFromUrl) ? sectionFromUrl : "preferences"
  })

  // Mobile drill-down state
  const [mobileView, setMobileView] = useState<MobileView>(() => {
    return sectionFromUrl && validSections.includes(sectionFromUrl) ? "section" : "list"
  })

  // LLM Backend actions from store
  const createBackend = useStore((state) => state.createBackend)
  const updateBackend = useStore((state) => state.updateBackend)
  const deleteBackend = useStore((state) => state.deleteBackend)
  const testBackend = useStore((state) => state.testBackend)

  const sections = getSettingsSections(t)

  const handleSectionChange = (section: SettingsSectionType) => {
    setActiveSection(section)
    setSearchParams({ tab: section }, { replace: true })
    if (isMobile) {
      setMobileView("section")
    }
  }

  const handleMobileBack = () => {
    setMobileView("list")
    setSearchParams({}, { replace: true })
  }

  // Render the active section content
  const renderSection = () => {
    switch (activeSection) {
      case "llm":
        return (
          <UnifiedLLMBackendsTab
            onCreateBackend={createBackend}
            onUpdateBackend={updateBackend}
            onDeleteBackend={deleteBackend}
            onTestBackend={testBackend}
          />
        )
      case "connections":
        return <UnifiedDeviceConnectionsTab />
      case "preferences":
        return <PreferencesTab />
      case "about":
        return <AboutTab />
    }
  }

  // Mobile: section list view
  const mobileSectionLabel = sections.find((s) => s.value === activeSection)?.label

  if (isMobile) {
    return (
      <PageLayout title={t("settings:title")} borderedHeader={false}>
        {mobileView === "list" ? (
          <div>
            <h2 className="text-lg font-semibold mb-4">{t("settings:title")}</h2>
            <SettingsSectionList sections={sections} onSectionSelect={handleSectionChange} />
          </div>
        ) : (
          <div>
            {/* Mobile section header with back button */}
            <div className="flex items-center gap-2 mb-4">
              <Button
                variant="ghost"
                size="icon"
                onClick={handleMobileBack}
                aria-label={t("common:back")}
              >
                <ArrowLeft className="h-4 w-4" />
              </Button>
              <h2 className="text-base font-medium">{mobileSectionLabel}</h2>
            </div>
            {renderSection()}
          </div>
        )}
        {/* Bottom spacer for safe area on mobile */}
        <div style={{ height: "calc(2rem + env(safe-area-inset-bottom, 0px))" }} />
      </PageLayout>
    )
  }

  // Desktop: sidebar + content
  return (
    <PageLayout title={t("settings:title")} subtitle={t("settings:description")} borderedHeader={false}>
      <div className="flex gap-6">
        <SettingsNav
          sections={sections}
          activeSection={activeSection}
          onSectionChange={handleSectionChange}
        />
        <div className="flex-1 min-w-0">
          {renderSection()}
        </div>
      </div>
    </PageLayout>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -40`
Expected: No errors

- [ ] **Step 3: Verify the build passes**

Run: `cd web && npm run build 2>&1 | tail -20`
Expected: Build succeeds

- [ ] **Step 4: Manual visual check**

Run: `cd web && npm run dev`
Open `http://localhost:5173/settings` in browser. Verify:
- Desktop: sidebar on left with 4 items, clicking switches content area
- Desktop: active item has `bg-muted` + left border accent
- Mobile: card list with 4 rows, tapping drills into section
- Mobile: back arrow returns to list
- URL updates to `?tab=<section>` on both desktop and mobile
- Deep-link `?tab=llm` works on page load

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/settings.tsx
git commit -m "refactor(settings): replace horizontal tabs with sidebar navigation"
```

---

### Task 4: Cleanup and final verification

**Files:**
- Verify: `web/src/pages/settings.tsx`
- Verify: `web/src/pages/settings/SettingsNav.tsx`
- Verify: `web/src/pages/settings/SettingsSectionList.tsx`

- [ ] **Step 1: Verify no unused imports remain**

Check that `settings.tsx` no longer imports `PageTabsBar`, `PageTabsContent`, `PageTabsBottomNav`, or `PageTabs`.

Run: `cd web && npx tsc --noEmit --pretty`
Expected: No errors

- [ ] **Step 2: Run full build**

Run: `cd web && npm run build`
Expected: Build succeeds with no warnings about unused imports

- [ ] **Step 3: Verify no other pages are affected**

Grep for `PageTabsBar` usage in other pages to confirm it's still available:

Run: `grep -r "PageTabsBar" web/src/pages/ --include="*.tsx" -l`
Expected: Only pages OTHER than `settings.tsx` (e.g., dashboard, automation)

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: cleanup after settings sidebar navigation refactor"
```
(Only if there are unstaged changes — otherwise skip this step.)
