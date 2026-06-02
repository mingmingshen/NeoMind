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
            "bg-card hover:bg-muted-50 active:scale-[0.99]",
            "focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-2"
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
