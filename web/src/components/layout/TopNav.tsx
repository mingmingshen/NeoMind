/**
 * TopNav - Top navigation bar
 * Desktop: icon buttons with tooltips
 * Mobile: scrollable text tab bar with underline indicator + swipe gestures
 */

import { useStore } from "@/store"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next"
import { Link, useLocation, useNavigate } from "react-router-dom"
import {
  MessageSquare,
  Cpu,
  Workflow,
  Puzzle,
  Settings,
  Wifi,
  WifiOff,
  LogOut,
  Bell,
  LayoutDashboard,
  BellRing,
  Bot,
  Check,
  Database,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { BrandLogoWithName } from "@/components/shared/BrandName"
import { Badge } from "@/components/ui/badge"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { ThemeToggle } from "./ThemeToggle"
import { useState, useEffect, useRef, useCallback, forwardRef } from "react"
import { setTopNavHeight } from "@/hooks/useVisualViewport"
import { useIsMobile } from "@/hooks/useMobile"

type PageType = "dashboard" | "visual-dashboard" | "data" | "devices" | "automation" | "agents" | "messages" | "extensions" | "settings"

interface NavItem {
  id: PageType
  path: string
  icon: React.ComponentType<{ className?: string }>
  labelKey: string
  /** Shorter label for mobile tab bar (falls back to labelKey if not set) */
  mobileLabelKey?: string
}

const navItems: NavItem[] = [
  { id: "dashboard", path: "/chat", labelKey: "nav.dashboard", mobileLabelKey: "navShort.dashboard", icon: MessageSquare },
  { id: "agents", path: "/agents", labelKey: "nav.agents", mobileLabelKey: "navShort.agents", icon: Bot },
  { id: "visual-dashboard", path: "/visual-dashboard", labelKey: "nav.visual-dashboard", mobileLabelKey: "navShort.visual-dashboard", icon: LayoutDashboard },
  { id: "devices", path: "/devices", labelKey: "nav.devices", mobileLabelKey: "navShort.devices", icon: Cpu },
  { id: "automation", path: "/automation", labelKey: "nav.automation", mobileLabelKey: "navShort.automation", icon: Workflow },
  { id: "data", path: "/data", labelKey: "nav.data", mobileLabelKey: "navShort.data", icon: Database },
  { id: "messages", path: "/messages", labelKey: "nav.messages", mobileLabelKey: "navShort.messages", icon: Bell },
  { id: "extensions", path: "/extensions", labelKey: "nav.extensions", mobileLabelKey: "navShort.extensions", icon: Puzzle },
  { id: "settings", path: "/settings", labelKey: "nav.settings", mobileLabelKey: "navShort.settings", icon: Settings },
]

export const TopNav = forwardRef<HTMLDivElement>((props, ref) => {
  const innerRef = useRef<HTMLDivElement>(null)
  const tabBarRef = useRef<HTMLDivElement>(null)
  const [indicatorStyle, setIndicatorStyle] = useState({ left: 0, width: 0 })
  const touchStartX = useRef(0)

  const isMobile = useIsMobile()
  const navigate = useNavigate()

  // Set the nav height in CSS variable after mount and on resize
  useEffect(() => {
    const updateNavHeight = () => {
      if (innerRef.current) {
        const height = innerRef.current.getBoundingClientRect().height
        setTopNavHeight(height)
      }
    }

    updateNavHeight()
    window.addEventListener('resize', updateNavHeight)
    return () => window.removeEventListener('resize', updateNavHeight)
  }, [])

  const { t, i18n } = useTranslation('common')
  const location = useLocation()
  const user = useStore((state) => state.user)
  const isConnected = useStore((state) => state.wsConnected)
  const logout = useStore((state) => state.logout)
  const alerts = useStore((state) => state.alerts)
  const fetchAlerts = useStore((state) => state.fetchAlerts)
  const acknowledgeAlert = useStore((state) => state.acknowledgeAlert)
  const [alertDropdownOpen, setAlertDropdownOpen] = useState(false)

  // Fetch alerts on mount and periodically
  useEffect(() => {
    fetchAlerts()
    const interval = setInterval(fetchAlerts, 30000)
    return () => clearInterval(interval)
  }, [fetchAlerts])

  // Count unacknowledged alerts
  const unreadCount = alerts.filter(a => !a.acknowledged && a.status !== 'resolved' && a.status !== 'acknowledged').length

  // Get current path without trailing slash for comparison
  const currentPath = location.pathname.endsWith('/') && location.pathname !== '/'
    ? location.pathname.slice(0, -1)
    : location.pathname

  const getUserInitials = (username: string) => {
    return username.slice(0, 2).toUpperCase()
  }

  const toggleLanguage = () => {
    const newLang = i18n.language === 'zh' ? 'en' : 'zh'
    i18n.changeLanguage(newLang)
  }

  const handleLogout = () => {
    logout()
  }

  const handleAcknowledgeAlert = async (alertId: string) => {
    await acknowledgeAlert(alertId)
  }

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical':
      case 'emergency':
        return 'text-error bg-error-light border-error'
      case 'warning':
        return 'text-warning bg-warning-light border-warning'
      case 'info':
      default:
        return 'text-info bg-info-light border-info'
    }
  }

  // Check if a nav item is currently active
  const isItemActive = useCallback((item: NavItem) => {
    return currentPath === item.path ||
      (item.path === '/chat' && currentPath === '/') ||
      currentPath.startsWith(`${item.path}/`)
  }, [currentPath])

  // Update underline indicator position and scroll active tab into view
  useEffect(() => {
    if (!isMobile || !tabBarRef.current) return

    const activeTab = tabBarRef.current.querySelector('[data-active="true"]') as HTMLElement | null
    if (activeTab) {
      setIndicatorStyle({
        left: activeTab.offsetLeft,
        width: activeTab.offsetWidth,
      })
      activeTab.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'nearest' })
    }
  }, [currentPath, isMobile])

  // Swipe handlers for tab bar
  const handleTabTouchStart = useCallback((e: React.TouchEvent) => {
    touchStartX.current = e.touches[0].clientX
  }, [])

  const handleTabTouchEnd = useCallback((e: React.TouchEvent) => {
    const deltaX = e.changedTouches[0].clientX - touchStartX.current
    if (Math.abs(deltaX) < 50) return

    const currentIndex = navItems.findIndex(item => isItemActive(item))
    if (deltaX < 0 && currentIndex >= 0 && currentIndex < navItems.length - 1) {
      navigate(navItems[currentIndex + 1].path)
    } else if (deltaX > 0 && currentIndex > 0) {
      navigate(navItems[currentIndex - 1].path)
    }
  }, [isItemActive, navigate])

  return (
    <TooltipProvider delayDuration={500}>
      <nav
        ref={innerRef}
        className="fixed top-0 left-0 right-0 z-20 bg-surface-glass backdrop-blur-xl border-b border-glass-border flex flex-col"
        style={{ paddingTop: 'env(safe-area-inset-top, 0px)' }}
      >
        {/* Main bar */}
        <div className="flex items-center px-4 sm:px-6 h-14">
          {/* Logo */}
          <Link to="/chat" className="flex shrink-0 items-center justify-center mr-4 md:mr-6">
            <BrandLogoWithName />
          </Link>

          {/* Desktop Navigation Icons */}
          <div className="hidden md:flex items-center gap-1.5">
            {navItems.map((item) => {
              const Icon = item.icon
              const isActive = isItemActive(item)

              return (
                <Tooltip key={item.id}>
                  <TooltipTrigger asChild>
                    <Link to={item.path}>
                      <Button
                        variant="ghost"
                        size="icon"
                        className={cn(
                          "w-11 h-11 rounded-lg transition-all",
                          isActive
                            ? "bg-muted text-primary hover:bg-muted-50"
                            : "text-muted-foreground hover:text-foreground hover:bg-muted-50"
                        )}
                      >
                        <Icon className="h-5 w-5" />
                      </Button>
                    </Link>
                  </TooltipTrigger>
                  <TooltipContent side="bottom" className="text-xs px-2 py-1">
                    {t(item.labelKey)}
                  </TooltipContent>
                </Tooltip>
              )
            })}
          </div>

          {/* Spacer */}
          <div className="flex-1 max-md:max-w-4" />

          {/* Right side: Status + Theme + Alerts + User */}
          <div className="ml-auto flex shrink-0 items-center gap-1.5 sm:gap-2.5">
            {/* Connection status */}
            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  className={cn(
                    "flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-colors",
                    isConnected
                      ? "bg-success-light text-success border border-success-light"
                      : "text-destructive bg-muted"
                  )}
                >
                  {isConnected ? (
                    <Wifi className="h-4 w-4" />
                  ) : (
                    <WifiOff className="h-4 w-4" />
                  )}
                  <span className="hidden sm:inline">
                    {isConnected ? t('connection.connected') : t('connection.disconnected')}
                  </span>
                </div>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                {isConnected ? t('connection.wsConnected') : t('connection.wsDisconnected')}
              </TooltipContent>
            </Tooltip>

            {/* Language toggle - hidden on mobile */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={toggleLanguage}
                  className="hidden sm:flex h-10 w-10 rounded-lg text-muted-foreground hover:text-foreground text-xs font-medium"
                >
                  {i18n.language === 'zh' ? '中' : 'EN'}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs sm:block">
                {i18n.language === 'zh' ? t('language.switchToEnglish') : t('language.switchToChinese')}
              </TooltipContent>
            </Tooltip>

            {/* Theme toggle */}
            <ThemeToggle />

            {/* Alerts notification */}
            <DropdownMenu open={alertDropdownOpen} onOpenChange={setAlertDropdownOpen}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="w-10 h-10 rounded-lg relative"
                    >
                      <BellRing className="h-4 w-4" />
                      {unreadCount > 0 && (
                        <Badge
                          variant="destructive"
                          className="absolute -top-0.5 -right-0.5 h-5 min-w-5 px-1 flex items-center justify-center text-xs"
                        >
                          {unreadCount > 99 ? '99+' : unreadCount}
                        </Badge>
                      )}
                    </Button>
                  </DropdownMenuTrigger>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  {t('alerts.title')}
                </TooltipContent>
              </Tooltip>
              <DropdownMenuContent align="end" className="w-80 max-h-80 overflow-y-auto">
                <div className="px-3 py-2 border-b">
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-sm">{t('alerts.title')}</span>
                    {unreadCount > 0 && (
                      <Badge variant="outline" className="text-xs">
                        {unreadCount} {t('alerts.unread')}
                      </Badge>
                    )}
                  </div>
                </div>
                {alerts.length === 0 ? (
                  <div className="py-8 text-center text-muted-foreground text-sm">
                    <Bell className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    {t('alerts.noAlerts')}
                  </div>
                ) : (
                  <div className="py-1">
                    {alerts.slice(0, 10).map((alert) => {
                      const isUnread = !alert.acknowledged && alert.status !== 'resolved' && alert.status !== 'acknowledged'
                      return (
                        <div
                          key={alert.id}
                          className={cn(
                            "px-3 py-1.5 border-b last:border-b-0 hover:bg-muted-50 transition-colors",
                            isUnread && "bg-muted-30"
                          )}
                        >
                          <div className="flex items-center gap-2">
                            <Badge
                              variant="outline"
                              className={cn(
                                "text-[10px] px-1 py-0 shrink-0 h-5 flex items-center",
                                getSeverityColor(alert.severity)
                              )}
                            >
                              {alert.severity}
                            </Badge>
                            {isUnread && (
                              <div className="w-1.5 h-1.5 rounded-full bg-info shrink-0" />
                            )}
                            <p className="text-xs font-medium truncate flex-1">{alert.title}</p>
                            {isUnread && (
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-5 w-5 shrink-0"
                                onClick={() => handleAcknowledgeAlert(alert.id)}
                                title={t('alerts.acknowledge')}
                              >
                                <Check className="h-4 w-4" />
                              </Button>
                            )}
                          </div>
                          <p className="text-[11px] text-muted-foreground truncate ml-7 mt-0.5" title={alert.message}>
                            {alert.message}
                          </p>
                        </div>
                      )
                    })}
                    {alerts.length > 10 && (
                      <div className="px-3 py-1.5 text-center text-xs text-muted-foreground">
                        {t('alerts.moreAlerts', { count: alerts.length - 10 })}
                      </div>
                    )}
                  </div>
                )}
              </DropdownMenuContent>
            </DropdownMenu>

            {/* User avatar with dropdown */}
            {user && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Avatar className="h-10 w-10 cursor-pointer rounded-lg">
                    <AvatarFallback className="bg-muted text-muted-foreground text-xs font-medium">
                      {getUserInitials(user.username)}
                    </AvatarFallback>
                  </Avatar>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-48">
                  <div className="px-3 py-2">
                    <p className="text-sm font-medium">{user.username}</p>
                  </div>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onClick={toggleLanguage}>
                    {i18n.language === 'zh' ? 'English' : '中文'}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={handleLogout} className="text-destructive focus:text-destructive">
                    <LogOut className="h-4 w-4 mr-2" />
                    {t('logout')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            )}
          </div>
        </div>

        {/* Mobile: Scrollable text tab bar with underline indicator */}
        {isMobile && (
          <div
            className="relative border-b border-border/50"
            onTouchStart={handleTabTouchStart}
            onTouchEnd={handleTabTouchEnd}
          >
            <div
              ref={tabBarRef}
              className="relative flex overflow-x-auto scrollbar-none px-3"
            >
              {navItems.map((item) => {
                const isActive = isItemActive(item)
                return (
                  <Link
                    key={item.id}
                    to={item.path}
                    data-active={isActive || undefined}
                    className={cn(
                      "flex-shrink-0 px-4 py-2.5 text-base whitespace-nowrap transition-all select-none",
                      isActive
                        ? "text-foreground font-bold"
                        : "text-muted-foreground active:text-foreground"
                    )}
                  >
                    {t(item.mobileLabelKey || item.labelKey)}
                  </Link>
                )
              })}
              {/* Animated underline indicator */}
              <div
                className="absolute bottom-0 h-[3px] bg-primary transition-all duration-250 ease-out rounded-full"
                style={{ left: indicatorStyle.left, width: indicatorStyle.width }}
              />
            </div>
          </div>
        )}
      </nav>
    </TooltipProvider>
  )
})

TopNav.displayName = 'TopNav'
