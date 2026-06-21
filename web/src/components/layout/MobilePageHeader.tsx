/**
 * MobilePageHeader - Per-page sticky top bar (mobile-only)
 *
 * Layout: [☰ drawer] [leftExtra?] [title — centered] [actions?]
 * The center holds ONLY the title. No subtitle, no extra content.
 *
 * The bar is h-12 (48px) + safe-area-top, sticky to the top of the scroll
 * container, with bg-background that extends under the status bar.
 *
 * Desktop layout does not render this component.
 */

import { type ReactNode } from "react"
import { useTranslation } from "react-i18next"
import { Menu } from "lucide-react"
import { cn } from "@/lib/utils"
import { useIsMobile } from "@/hooks/useMobile"
import { useMobileNav } from "@/store/mobileNav"
import { Button } from "@/components/ui/button"

export interface MobilePageHeaderProps {
  /** Center title (required). */
  title: ReactNode
  /** Right slot for page actions (add, search, refresh). */
  actions?: ReactNode
  /** Hide the hamburger (default: false). Useful when a custom back button replaces it. */
  hideMenu?: boolean
  /** Optional left slot rendered AFTER the hamburger (e.g., back chevron on sub-pages). */
  leftExtra?: ReactNode
  className?: string
}

export function MobilePageHeader({
  title,
  actions,
  hideMenu = false,
  leftExtra,
  className,
}: MobilePageHeaderProps) {
  const isMobile = useIsMobile()
  const { t } = useTranslation("common")
  const setOpen = useMobileNav((s) => s.setOpen)
  if (!isMobile) return null

  return (
    <div
      className={cn(
        // 3-column grid: [left 1fr][title auto][right 1fr]. Equal side
        // columns make the title visually centered regardless of how many
        // action buttons either side has — flex-1 + text-center was offset
        // when the action stack was wider than the hamburger.
        "safe-top sticky top-0 z-30 grid h-12 grid-cols-[1fr_auto_1fr] items-center gap-1 border-b border-border bg-background px-2",
        className,
      )}
    >
      {/* Left slot: hamburger + leftExtra (e.g. back chevron) */}
      <div className="flex min-w-0 items-center gap-0.5 justify-self-start">
        {!hideMenu && (
          <Button
            variant="ghost"
            size="icon"
            className="h-9 w-9 shrink-0"
            onClick={() => setOpen(true)}
            aria-label={t("system.menu")}
          >
            <Menu className="h-5 w-5" />
          </Button>
        )}
        {leftExtra}
      </div>
      {/* Center title — clamp width so very long titles truncate instead of
          pushing the side columns and breaking centering. */}
      <span className="min-w-0 max-w-[65vw] truncate text-center text-sm font-semibold text-foreground">
        {title}
      </span>
      {/* Right slot: actions */}
      <div className="flex min-w-0 items-center gap-0.5 justify-self-end">
        {actions}
      </div>
    </div>
  )
}
