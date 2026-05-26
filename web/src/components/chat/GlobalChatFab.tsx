/**
 * GlobalChatFab - Floating action button + side panel for quick AI chat access
 *
 * Shows a FAB on all non-chat pages. Clicking opens a Sheet with PanelChatView.
 * Auto-closes when navigating to /chat. Respects active streaming state.
 */

import { useState, useEffect } from "react"
import { useLocation } from "react-router-dom"
import { useTranslation } from "react-i18next"
import { MessageSquare } from "lucide-react"
import { Sheet, SheetContent } from "@/components/ui/sheet"
import { PanelChatView } from "./PanelChatView"
import { notifyInfo } from "@/lib/notify"
import { cn } from "@/lib/utils"

export function GlobalChatFab() {
  const [open, setOpen] = useState(false)
  const [isStreaming, setIsStreaming] = useState(false)
  const location = useLocation()
  const { t } = useTranslation("chat")

  // Detect chat pages: /, /chat, /chat/:sessionId
  const isChatPage = location.pathname === "/" || location.pathname.startsWith("/chat")

  // Auto-close on /chat navigation (delay if streaming)
  useEffect(() => {
    if (isChatPage && open) {
      if (isStreaming) {
        notifyInfo(t("streamInProgress"))
        return
      }
      setOpen(false)
    }
  }, [isChatPage, open, isStreaming, t])

  // Hide FAB entirely on chat pages
  if (isChatPage) return null

  return (
    <>
      {/* Floating action button */}
      {!open && (
        <button
          onClick={() => setOpen(true)}
          aria-label={t("openPanel")}
          className={cn(
            "fixed bottom-6 right-6 z-40",
            "w-14 h-14 rounded-full",
            "bg-info text-primary-foreground",
            "shadow-lg hover:shadow-xl",
            "flex items-center justify-center",
            "hover:scale-105 transition-all duration-200",
            "animate-in zoom-in-0 fade-in-0 duration-300",
            "safe-bottom"
          )}
        >
          <MessageSquare className="h-6 w-6" />
        </button>
      )}

      {/* Side panel */}
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent
          className={cn(
            "z-[100]",
            "bg-surface-glass backdrop-blur-xl",
            "w-screen md:w-[400px] md:max-w-[400px]",
            "p-0 gap-0",
            "flex flex-col h-full"
          )}
        >
          <PanelChatView
            onClose={() => setOpen(false)}
            onStreamingChange={setIsStreaming}
          />
        </SheetContent>
      </Sheet>
    </>
  )
}
