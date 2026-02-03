import { useState } from "react"
import { useTranslation } from "react-i18next"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { AboutTab } from "./settings/AboutTab"
import { PreferencesTab } from "./settings/PreferencesTab"
import { Sliders, Info } from "lucide-react"

type SettingsTabValue = "about" | "preferences"

export function SettingsPage() {
  const { t } = useTranslation(["common", "settings"])
  const [activeTab, setActiveTab] = useState<SettingsTabValue>("preferences")

  const tabs = [
    { value: "preferences" as SettingsTabValue, label: t("settings:preferences"), icon: <Sliders className="h-4 w-4" /> },
    { value: "about" as SettingsTabValue, label: t("settings:about"), icon: <Info className="h-4 w-4" /> },
  ]

  return (
    <PageLayout
      title={t('settings:title')}
      subtitle={t('settings:description')}
      borderedHeader={false}
      maxWidth="lg"
    >
      <PageTabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(v) => setActiveTab(v as SettingsTabValue)}
      >
        {/* Preferences Tab */}
        <PageTabsContent value="preferences" activeTab={activeTab}>
          <PreferencesTab />
        </PageTabsContent>

        {/* About Tab */}
        <PageTabsContent value="about" activeTab={activeTab}>
          <AboutTab />
        </PageTabsContent>
      </PageTabs>
    </PageLayout>
  )
}
