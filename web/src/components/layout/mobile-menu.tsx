import { createContext, useContext, useState, useMemo, ReactNode } from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import { MessageSquare, Cpu, Workflow, Brain, Puzzle, Settings, Bot, Menu } from "lucide-react"
import { useStore } from "@/store"
import { Button } from "@/components/ui/button"
import {
  Sheet,
  SheetContent,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet"

const navItemIcons = {
  dashboard: MessageSquare,
  devices: Cpu,
  automation: Workflow,
  decisions: Brain,
  plugins: Puzzle,
  settings: Settings,
}

const MobileMenuContext = createContext<{
  isOpen: boolean
  setOpen: (open: boolean) => void
}>({
  isOpen: false,
  setOpen: () => {},
})

export function useMobileMenu() {
  return useContext(MobileMenuContext)
}

export function MobileMenuProvider({ children }: { children: ReactNode }) {
  const [isOpen, setOpen] = useState(false)

  return (
    <MobileMenuContext.Provider value={{ isOpen, setOpen }}>
      {children}
    </MobileMenuContext.Provider>
  )
}

export function MobileMenuTrigger() {
  const { setOpen } = useMobileMenu()

  return (
    <Button
      size="icon"
      variant="ghost"
      className="h-8 w-8 rounded-lg md:hidden"
      onClick={() => setOpen(true)}
    >
      <Menu className="h-4 w-4" />
    </Button>
  )
}

export function MobileMenuSheet() {
  const { isOpen, setOpen } = useMobileMenu()
  const { t } = useTranslation('navigation')
  const { currentPage, setCurrentPage } = useStore()

  const navItems = useMemo(() => [
    { id: "dashboard" as const, icon: navItemIcons.dashboard },
    { id: "devices" as const, icon: navItemIcons.devices },
    { id: "automation" as const, icon: navItemIcons.automation },
    { id: "decisions" as const, icon: navItemIcons.decisions },
    { id: "plugins" as const, icon: navItemIcons.plugins },
    { id: "settings" as const, icon: navItemIcons.settings },
  ], [])

  return (
    <Sheet open={isOpen} onOpenChange={setOpen}>
      <SheetContent side="left" className="w-56 p-0">
        <SheetTitle className="sr-only">导航菜单</SheetTitle>
        <SheetDescription className="sr-only">选择页面进行导航</SheetDescription>
        <div className="flex flex-col h-full">
          <div className="flex h-14 items-center gap-2 border-b px-4">
            <Bot className="h-5 w-5" />
            <span className="text-sm font-semibold">
              NeoTalk
            </span>
          </div>

          <nav className="flex-1 space-y-1 p-2 overflow-y-auto">
            {navItems.map((item) => {
              const Icon = item.icon
              const isActive = currentPage === item.id
              return (
                <button
                  key={item.id}
                  onClick={() => {
                    setCurrentPage(item.id)
                    setOpen(false)
                  }}
                  className={cn(
                    "flex w-full items-center gap-3 rounded-md px-3 py-2.5 text-sm transition-colors",
                    isActive
                      ? "bg-muted text-foreground font-medium"
                      : "text-muted-foreground hover:bg-muted/50"
                  )}
                >
                  <Icon className="h-4 w-4 shrink-0" />
                  <span>{t(item.id)}</span>
                </button>
              )
            })}
          </nav>

          <div className="p-3 border-t">
            <div className="flex items-center gap-2 text-muted-foreground text-xs">
              <div className="h-2 w-2 rounded-full bg-green-500" />
              <span>{t('systemRunning', { ns: 'common' })}</span>
            </div>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}
