import { useMemo } from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import {
  MessageSquare,
  Cpu,
  AlertTriangle,
  Workflow,
  Settings,
  ChevronLeft,
  ChevronRight,
  Brain,
  Puzzle,
  Bot,
  Activity,
} from "lucide-react"
import { useStore } from "@/store"

const navItemIcons = {
  dashboard: MessageSquare,
  devices: Cpu,
  alerts: AlertTriangle,
  automation: Workflow,
  events: Activity,
  decisions: Brain,
  plugins: Puzzle,
  settings: Settings,
}

export function DesktopSidebar() {
  const { t } = useTranslation('navigation')
  const { currentPage, setCurrentPage, sidebarOpen, toggleSidebar } = useStore()

  const navItems = useMemo(() => [
    { id: "dashboard" as const, icon: navItemIcons.dashboard },
    { id: "devices" as const, icon: navItemIcons.devices },
    { id: "automation" as const, icon: navItemIcons.automation },
    { id: "events" as const, icon: navItemIcons.events },
    { id: "decisions" as const, icon: navItemIcons.decisions },
    { id: "plugins" as const, icon: navItemIcons.plugins },
    { id: "settings" as const, icon: navItemIcons.settings },
  ], [])

  return (
    <aside
      className={cn(
        "hidden md:flex flex-col border-r bg-background",
        sidebarOpen ? "w-56" : "w-14"
      )}
    >
      {/* Header */}
      <div className="flex h-14 items-center justify-between border-b px-3">
        {sidebarOpen && (
          <div className="flex items-center gap-2">
            <Bot className="h-5 w-5" />
            <span className="text-sm font-semibold">
              NeoTalk
            </span>
          </div>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation()
            toggleSidebar()
          }}
          className="ml-auto rounded p-1 hover:bg-muted"
        >
          {sidebarOpen ? (
            <ChevronLeft className="h-4 w-4" />
          ) : (
            <ChevronRight className="h-4 w-4" />
          )}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 p-2 overflow-y-auto">
        {navItems.map((item) => {
          const Icon = item.icon
          const isActive = currentPage === item.id
          return (
            <button
              key={item.id}
              onClick={(e) => {
                e.stopPropagation()
                setCurrentPage(item.id)
              }}
              className={cn(
                "flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors",
                isActive
                  ? "bg-muted text-foreground font-medium"
                  : "text-muted-foreground hover:bg-muted/50"
              )}
            >
              <Icon className="h-4 w-4 shrink-0" />
              {sidebarOpen && (
                <span>{t(item.id)}</span>
              )}
            </button>
          )
        })}
      </nav>

      {/* Footer */}
      <div className="p-3 border-t">
        <div className={cn(
          "flex items-center gap-2 text-muted-foreground",
          sidebarOpen ? "justify-start" : "justify-center"
        )}>
          <div className="h-2 w-2 rounded-full bg-green-500" />
          {sidebarOpen && (
            <span className="text-xs">
              {t('systemRunning', { ns: 'common' })}
            </span>
          )}
        </div>
      </div>
    </aside>
  )
}

export function Sidebar() {
  return <DesktopSidebar />
}
