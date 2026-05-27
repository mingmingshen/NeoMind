/**
 * GlobalChatFab - Floating action button + full-screen chat overlay
 *
 * Shows a FAB on all non-chat pages. Clicking expands to a full-screen chat overlay
 * with smooth scale-up animation from the FAB position.
 * Minimize button collapses back to FAB with reverse animation.
 */

import { useState, useEffect, useRef, useCallback } from "react"
import { useLocation } from "react-router-dom"
import { useTranslation } from "react-i18next"
import { MessageSquare, Minimize2 } from "lucide-react"
import { PanelChatView, PANEL_SESSION_KEY } from "./PanelChatView"
import { notifyInfo } from "@/lib/notify"
import { cn } from "@/lib/utils"
import { useStore } from "@/store"
import { selectChatActions } from "@/store/selectors"

type PanelState = "closed" | "opening" | "open" | "closing"

export function GlobalChatFab() {
  const [panelState, setPanelState] = useState<PanelState>("closed")
  const [isStreaming, setIsStreaming] = useState(false)
  // Persist panel session — survives panel unmount AND page refresh
  const panelSessionIdRef = useRef<string | null>(
    localStorage.getItem(PANEL_SESSION_KEY)
  )
  const { createSession } = useStore(selectChatActions)
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
    // Re-sync from localStorage in case panel session was reset by PanelChatView
    const stored = localStorage.getItem(PANEL_SESSION_KEY)
    if (stored) panelSessionIdRef.current = stored
    else panelSessionIdRef.current = null

    setPanelState("opening")
    // Let CSS animation play, then mark as fully open
    requestAnimationFrame(() => {
      setTimeout(() => setPanelState("open"), 300)
    })
  }

  // Panel calls this once to get a persistent session
  const ensurePanelSession = useCallback(async (): Promise<string> => {
    if (panelSessionIdRef.current) return panelSessionIdRef.current
    const id = await createSession()
    if (id) {
      panelSessionIdRef.current = id
      localStorage.setItem(PANEL_SESSION_KEY, id)
    }
    return id!
  }, [createSession])

  const handleClose = () => {
    setPanelState("closing")
    setTimeout(() => setPanelState("closed"), 250)
  }

  // Hide FAB entirely on chat pages
  if (isChatPage) return null

  return (
    <>
      {/* Floating action button — glass + glow */}
      <button
        ref={fabRef}
        onClick={isOpen ? handleClose : handleOpen}
        aria-label={isOpen ? t("closePanel") : t("openPanel")}
        className={cn(
          "fixed bottom-20 right-6 z-50",
          "w-14 h-14 rounded-full",
          "flex items-center justify-center",
          "transition-all duration-300 ease-out",
          "safe-bottom",
          // Glass background with brand orange
          "bg-accent-orange-bg backdrop-blur-xl",
          "border border-accent-orange/30",
          "text-accent-orange",
          // Glow ring
          "shadow-[0_0_24px_var(--accent-orange-bg),0_0_48px_var(--accent-orange-bg)]",
          "hover:shadow-[0_0_32px_var(--accent-orange),0_0_64px_var(--accent-orange-bg)]",
          "hover:border-accent-orange/50",
          isOpen
            ? "scale-0 opacity-0 pointer-events-none"
            : "scale-100 opacity-100 hover:scale-105"
        )}
      >
        <MessageSquare className="h-5 w-5" />
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
            : "bottom-20 right-6 w-14 h-14 rounded-full opacity-0 scale-0 pointer-events-none",
          "backdrop-blur-2xl",
          "border border-glass-border",
          "shadow-2xl",
          "flex flex-col overflow-hidden"
        )}
        style={{ backgroundColor: "var(--surface-glass)", backdropFilter: "blur(40px) saturate(1.8)" }}
      >
        {/* Only render content when visible */}
        {panelState !== "closed" && (
          <PanelChatView
            onClose={handleClose}
            onStreamingChange={setIsStreaming}
            ensureSession={ensurePanelSession}
            showMinimize
          />
        )}
      </div>
    </>
  )
}
