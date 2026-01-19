import { useState } from "react"
import { useTranslation } from "react-i18next"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { AboutTab } from "./settings/AboutTab"
import { PreferencesTab } from "./settings/PreferencesTab"

type SettingsTabValue = "about" | "preferences"

export function SettingsPage() {
  const { t } = useTranslation(["common", "settings"])
  const [activeTab, setActiveTab] = useState<SettingsTabValue>("preferences")

  const tabs = [
    { value: "preferences" as SettingsTabValue, label: t("settings:preferences") },
    { value: "about" as SettingsTabValue, label: t("settings:about") },
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
