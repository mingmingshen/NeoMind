/**
 * GlobalChatFab - Floating action button + full-screen chat overlay
 *
 * Shows a FAB on all non-chat pages. Clicking expands to a full-screen chat overlay
 * with smooth scale-up animation from the FAB position.
 * Minimize button collapses back to FAB with reverse animation.
 */

import { useState, useEffect, useRef } from "react"
import { useLocation } from "react-router-dom"
import { useTranslation } from "react-i18next"
import { MessageSquare, Minimize2 } from "lucide-react"
import { PanelChatView } from "./PanelChatView"
import { notifyInfo } from "@/lib/notify"
import { cn } from "@/lib/utils"

type PanelState = "closed" | "opening" | "open" | "closing"

export function GlobalChatFab() {
  const [panelState, setPanelState] = useState<PanelState>("closed")
  const [isStreaming, setIsStreaming] = useState(false)
  const location = useLocation()
  const { t } = useTranslation("chat")
  const fabRef = useRef<HTMLButtonElement>(null)

  const isOpen = panelState === "open" || panelState === "opening"

  // Detect chat pages: /, /chat, /chat/:sessionId
  const isChatPage = location.pathname === "/" || location.pathname.startsWith("/chat")

  // Auto-close on /chat navigation (delay if streaming)
  useEffect(() => {
    if (isChatPage && isOpen) {
      if (isStreaming) {
        notifyInfo(t("streamInProgress"))
        return
      }
      handleClose()
    }
  }, [isChatPage, isOpen, isStreaming, t])

  const handleOpen = () => {
    setPanelState("opening")
    // Let CSS animation play, then mark as fully open
    requestAnimationFrame(() => {
      setTimeout(() => setPanelState("open"), 300)
    })
  }

  const handleClose = () => {
    setPanelState("closing")
    setTimeout(() => setPanelState("closed"), 250)
  }

  // Hide FAB entirely on chat pages
  if (isChatPage) return null

  return (
    <>
      {/* Floating action button */}
      <button
        ref={fabRef}
        onClick={isOpen ? handleClose : handleOpen}
        aria-label={isOpen ? t("closePanel") : t("openPanel")}
        className={cn(
          "fixed bottom-6 right-6 z-50",
          "w-14 h-14 rounded-full",
          "bg-info text-primary-foreground",
          "shadow-lg hover:shadow-xl",
          "flex items-center justify-center",
          "transition-all duration-300 ease-out",
          "safe-bottom",
          isOpen
            ? "scale-0 opacity-0 pointer-events-none"
            : "scale-100 opacity-100 hover:scale-110"
        )}
      >
        <MessageSquare className="h-6 w-6" />
      </button>

      {/* Full-screen overlay backdrop */}
      <div
        className={cn(
          "fixed inset-0 z-[90]",
          "bg-black/50 backdrop-blur-sm",
          "transition-opacity duration-300 ease-out",
          panelState === "closed"
            ? "opacity-0 pointer-events-none"
            : "opacity-100"
        )}
        onClick={isStreaming ? undefined : handleClose}
      />

      {/* Full-screen chat panel */}
      <div
        className={cn(
          "fixed z-[100]",
          // Origin from bottom-right (FAB position)
          "origin-bottom-right",
          // Smooth transitions
          "transition-all duration-300 ease-out",
          // State-dependent positioning & sizing
          panelState !== "closed"
            ? "inset-0 sm:inset-4 md:inset-8 rounded-none sm:rounded-2xl opacity-100 scale-100"
            : "bottom-6 right-6 w-14 h-14 rounded-full opacity-0 scale-0 pointer-events-none",
          "bg-surface-glass backdrop-blur-xl",
          "border border-glass-border",
          "shadow-2xl",
          "flex flex-col overflow-hidden"
        )}
      >
        {/* Only render content when visible */}
        {panelState !== "closed" && (
          <PanelChatView
            onClose={handleClose}
            onStreamingChange={setIsStreaming}
            showMinimize
          />
        )}
      </div>
    </>
  )
}
