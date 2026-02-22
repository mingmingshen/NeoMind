import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { logError } from "@/lib/errors"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Label } from "@/components/ui/label"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Clock,
  Check,
  Info,
  Loader2,
  Globe,
} from "lucide-react"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import { useGlobalTimezone } from "@/hooks/useTimeFormat"
import { COMMON_TIMEZONES } from "@/lib/time"

type Language = "zh" | "en"
type TimeFormat = "12h" | "24h"

interface Preferences {
  language: Language
  timeFormat: TimeFormat
  // Keep timeZone for backward compatibility
  timeZone?: "local" | "utc"
}

const PREFERENCES_KEY = "neomind_preferences"

// Default preferences
const defaultPreferences: Preferences = {
  language: "zh",
  timeFormat: "24h",
}

// Load preferences from localStorage
function loadPreferences(): Preferences {
  try {
    const saved = localStorage.getItem(PREFERENCES_KEY)
    if (saved) {
      const parsed = JSON.parse(saved)
      // Remove legacy theme field if present
      delete parsed.theme
      return { ...defaultPreferences, ...parsed }
    }
  } catch (e) {
    logError(e, { operation: 'Load preferences' })
  }
  return defaultPreferences
}

// Save preferences to localStorage
function savePreferences(pref: Preferences) {
  try {
    localStorage.setItem(PREFERENCES_KEY, JSON.stringify(pref))
  } catch (e) {
    logError(e, { operation: 'Save preferences' })
  }
}

export function PreferencesTab() {
  const { t, i18n } = useTranslation(["common", "settings"])
  const { handleError } = useErrorHandler()
  const { toast } = useToast()
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences)
  const [hasChanges, setHasChanges] = useState(false)

  // Global timezone for scheduling (separate from UI display)
  const {
    timezone: globalTimezone,
    isLoading: timezoneLoading,
    updateTimezone,
    availableTimezones,
    refresh: refreshTimezone,
  } = useGlobalTimezone()

  // Update preferences
  const updatePreference = <K extends keyof Preferences>(
    key: K,
    value: Preferences[K]
  ) => {
    setPreferences((prev) => ({ ...prev, [key]: value }))
    setHasChanges(true)
  }

  // Save all preferences
  const handleSave = () => {
    savePreferences(preferences)
    i18n.changeLanguage(preferences.language)
    setHasChanges(false)

    toast({
      title: t("settings:settingsSaved"),
    })
  }

  // Reset to defaults
  const handleReset = () => {
    setPreferences(defaultPreferences)
    setHasChanges(true)
  }

  const languageOptions = [
    { value: "zh" as Language, label: "简体中文" },
    { value: "en" as Language, label: "English" },
  ]

  const timeFormatOptions = [
    { value: "12h" as TimeFormat, label: "12小时制 (12:00 PM)" },
    { value: "24h" as TimeFormat, label: "24小时制 (14:00)" },
  ]

  return (
    <div className="space-y-6">
      {/* Actions */}
      {hasChanges && (
        <div className="flex items-center justify-between p-4 bg-muted/50 rounded-lg">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Info className="h-4 w-4" />
            <span>{t("settings:unsavedChanges")}</span>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={handleReset}>
              {t("common:reset")}
            </Button>
            <Button size="sm" onClick={handleSave}>
              <Check className="h-4 w-4 mr-1" />
              {t("settings:saveSettings")}
            </Button>
          </div>
        </div>
      )}

      {/* Language & Region Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5 text-blue-500" />
            {t("settings:languageRegion")}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Language */}
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-sm font-medium">
                {t("settings:language")}
              </Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("settings:languageDesc")}
              </p>
            </div>
            <Select
              value={preferences.language}
              onValueChange={(v) => updatePreference("language", v as Language)}
            >
              <SelectTrigger className="w-[180px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {languageOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      {/* Time Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="h-5 w-5 text-green-500" />
            {t("settings:timeSettings")}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Time Format */}
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-sm font-medium">
                {t("settings:timeFormat")}
              </Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("settings:timeFormatDesc")}
              </p>
            </div>
            <Select
              value={preferences.timeFormat}
              onValueChange={(v) => updatePreference("timeFormat", v as TimeFormat)}
            >
              <SelectTrigger className="w-[180px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {timeFormatOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* System Timezone */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="flex-1">
                <Label className="text-sm font-medium">
                  {t("settings:systemTimezone")}
                </Label>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {t("settings:systemTimezoneDesc")}
                </p>
              </div>
              <div className="flex items-center gap-2">
                {timezoneLoading && (
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                )}
                <Select
                  value={globalTimezone}
                  onValueChange={async (value) => {
                    try {
                      await updateTimezone(value)
                      toast({
                        title: t("settings:timezoneUpdated"),
                      })
                    } catch (e) {
                      toast({
                        title: t("settings:timezoneUpdateFailed"),
                        variant: "destructive",
                      })
                    }
                  }}
                  disabled={timezoneLoading}
                >
                  <SelectTrigger className="w-[280px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {(availableTimezones.length > 0 ? availableTimezones : COMMON_TIMEZONES).map(
                      (tz) => (
                        <SelectItem key={tz.id} value={tz.id}>
                          {tz.name}
                        </SelectItem>
                      )
                    )}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>

          {/* Current Time Preview */}
          <div className="pt-4 border-t">
            <div className="text-center p-4 bg-muted/50 rounded-lg">
              <div className="text-xs text-muted-foreground mb-1">
                {t("settings:currentTime")}
              </div>
              <div className="text-2xl font-mono font-medium">
                {formatTimeInTimezone(globalTimezone, preferences.timeFormat)}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Info */}
      <div className="text-sm text-muted-foreground text-center py-4">
        <p>{t("settings:preferencesInfo")}</p>
      </div>
    </div>
  )
}

// Format time in a specific timezone (IANA format like "Asia/Shanghai")
function formatTimeInTimezone(timezone: string, format: TimeFormat = "24h"): string {
  try {
    const now = new Date()
    const formatter = new Intl.DateTimeFormat("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      timeZone: timezone,
      hour12: format === "12h",
    })
    return formatter.format(now)
  } catch {
    return new Date().toLocaleTimeString()
  }
}

// Export hook for using preferences
export function usePreferences() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences)

  const updatePreferences = (updates: Partial<Preferences>) => {
    const newPrefs = { ...preferences, ...updates }
    setPreferences(newPrefs)
    savePreferences(newPrefs)
  }

  return { preferences, updatePreferences }
}
