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

  // Mobile drill-down view
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
          <div className="flex flex-col h-full overflow-hidden">
            {/* Mobile section header with back button — fixed */}
            <div className="flex items-center gap-2 mb-4 shrink-0">
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
            {/* Content area — scrolls independently */}
            <div className="flex-1 min-h-0 overflow-y-auto">
              {renderSection()}
            </div>
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
      <div className="flex gap-6 h-full overflow-hidden">
        <SettingsNav
          sections={sections}
          activeSection={activeSection}
          onSectionChange={handleSectionChange}
        />
        <div className="flex-1 min-w-0 overflow-y-auto pr-1">
          {renderSection()}
        </div>
      </div>
    </PageLayout>
  )
}
