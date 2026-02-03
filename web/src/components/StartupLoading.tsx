import { useEffect, useState } from "react"
import { listen } from "@tauri-apps/api/event"
import { getVersion } from "@tauri-apps/api/app"
import { BrandLogoHorizontal } from "@/components/shared/BrandName"

interface BackendReadyEvent {
  status: "ready" | "timeout"
  port: number
}

/**
 * Startup loading screen for Tauri app.
 *
 * Shows a loading animation while waiting for the backend server to start.
 * The window is initially hidden in tauri.conf.json and shown by Rust
 * once the backend is ready.
 *
 * This component provides a smooth startup experience on all platforms,
 * especially Windows where backend startup can be slower.
 */
export function StartupLoading({ onReady }: { onReady: () => void }) {
  const [status, setStatus] = useState<"loading" | "ready" | "timeout">("loading")
  const [version, setVersion] = useState<string>("")

  useEffect(() => {
    let unlisten: (() => void) | null = null
    let timeoutId: number | null = null

    const checkBackendStatus = async () => {
      try {
        // Get app version
        const ver = await getVersion()
        setVersion(ver)
      } catch {
        // Ignore version fetch errors
      }

      // Listen for backend-ready event from Rust
      try {
        unlisten = await listen<BackendReadyEvent>("backend-ready", (event) => {
          console.log("Backend ready event:", event.payload)

          if (event.payload.status === "ready") {
            setStatus("ready")
            // Give UI a moment to render before switching to main app
            setTimeout(() => {
              onReady()
            }, 300)
          } else {
            setStatus("timeout")
            // Still proceed even on timeout - frontend has retry logic
            setTimeout(() => {
              onReady()
            }, 1000)
          }
        })
      } catch (error) {
        console.error("Failed to listen for backend-ready event:", error)
        // Fallback: proceed anyway after a delay
        timeoutId = setTimeout(() => {
          setStatus("timeout")
          onReady()
        }, 3000)
      }

      // Safety timeout: if we don't get the event within 35 seconds, proceed anyway
      timeoutId = setTimeout(() => {
        console.log("Startup timeout - proceeding to app")
        setStatus("timeout")
        onReady()
      }, 35000)
    }

    checkBackendStatus()

    return () => {
      if (unlisten) unlisten()
      if (timeoutId) clearTimeout(timeoutId)
    }
  }, [onReady])

  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-background overflow-hidden">
      {/* Animated background */}
      <div className="fixed inset-0">
        <div className="absolute inset-0 bg-gradient-to-br from-background via-background to-muted/10" />
        <div className="absolute inset-0" style={{
          backgroundImage: 'radial-gradient(circle, #80808015 1px, transparent 1px)',
          backgroundSize: '32px 32px'
        }} />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-primary/5 dark:bg-primary/10 rounded-full blur-3xl animate-pulse" style={{ animationDuration: '8s' }} />
      </div>

      {/* Main content */}
      <div className="relative z-10 flex flex-col items-center gap-6">
        {/* Logo */}
        <BrandLogoHorizontal className="h-12" />

        {/* Loading animation */}
        <div className="flex items-center gap-3">
          <div className="w-2 h-2 rounded-full bg-primary animate-bounce" style={{ animationDelay: '0ms' }} />
          <div className="w-2 h-2 rounded-full bg-primary animate-bounce" style={{ animationDelay: '150ms' }} />
          <div className="w-2 h-2 rounded-full bg-primary animate-bounce" style={{ animationDelay: '300ms' }} />
        </div>

        {/* Status text */}
        <p className="text-sm text-muted-foreground">
          {status === "loading" && "正在启动后端服务..."}
          {status === "ready" && "服务已就绪"}
          {status === "timeout" && "正在连接..."}
        </p>

        {/* Version info */}
        {version && (
          <p className="text-xs text-muted-foreground/50 mt-4">v{version}</p>
        )}
      </div>
    </div>
  )
}
