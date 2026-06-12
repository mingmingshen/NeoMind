/**
 * OnboardingDialog — Full-screen getting-started wizard
 *
 * Shows system capabilities, guides users through first-time setup steps
 * (configure LLM backend, connect devices), and introduces additional features.
 */

import { useEffect } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { useNavigate } from "react-router-dom"
import { Rocket, Cpu, Sparkles, Check, X, Workflow, LayoutDashboard, Bot, Bell } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { OnboardingStatus } from "@/hooks/useOnboarding"

interface OnboardingDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  status: OnboardingStatus | null
  onDismiss: () => void
}

export function OnboardingDialog({ open, onOpenChange, status, onDismiss }: OnboardingDialogProps) {
  const { t } = useTranslation("common")
  const navigate = useNavigate()

  // Lock body scroll + handle Escape
  useEffect(() => {
    if (!open) return
    const prev = document.body.style.overflow
    document.body.style.overflow = "hidden"
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onOpenChange(false)
    }
    window.addEventListener("keydown", onKey)
    return () => {
      document.body.style.overflow = prev
      window.removeEventListener("keydown", onKey)
    }
  }, [open, onOpenChange])

  if (!open || !status) return null

  const handleAction = (path: string) => {
    onOpenChange(false)
    navigate(path)
  }

  const handleDismiss = () => {
    onDismiss()
    onOpenChange(false)
  }

  const root = typeof document !== "undefined"
    ? document.getElementById("dialog-root") || document.body
    : null
  if (!root) return null

  return createPortal(
    <div className="fixed inset-0 z-[100] flex flex-col bg-bg-90 backdrop-blur-xl overflow-y-auto">
      {/* Close button */}
      <button
        onClick={() => onOpenChange(false)}
        className="absolute top-4 right-4 z-10 w-9 h-9 rounded-lg flex items-center justify-center text-muted-foreground hover:bg-muted-30 transition-colors"
        aria-label="Close"
      >
        <X className="w-5 h-5" />
      </button>

      {/* Content */}
      <div className="flex-1 flex flex-col items-center px-6 sm:px-10 lg:px-16 py-12 sm:py-16 min-h-full">
        {/* Header */}
        <div className="w-12 h-12 rounded-xl bg-primary/10 flex items-center justify-center mb-3">
          <Rocket className="w-6 h-6 text-primary" />
        </div>
        <h1 className="text-2xl sm:text-3xl font-bold text-foreground mb-2 text-center">
          {t("onboarding.title")}
        </h1>
        <p className="text-muted-foreground text-sm sm:text-base max-w-2xl text-center mb-10">
          {t("onboarding.subtitle")}
        </p>

        {/* Core Steps — side by side on desktop */}
        <div className="w-full max-w-4xl grid grid-cols-1 md:grid-cols-2 gap-4 mb-8">
          <StepCard
            icon={<Sparkles className="w-5 h-5" />}
            title={t("onboarding.steps.llm.title")}
            description={t("onboarding.steps.llm.description")}
            purpose={t("onboarding.steps.llm.purpose")}
            completed={status.steps.llm.completed}
            completedLabel={t("onboarding.completed")}
            actionLabel={t("onboarding.steps.llm.action")}
            onAction={() => handleAction("/settings?tab=llm")}
          />
          <StepCard
            icon={<Cpu className="w-5 h-5" />}
            title={t("onboarding.steps.device.title")}
            description={t("onboarding.steps.device.description")}
            purpose={t("onboarding.steps.device.purpose")}
            completed={status.steps.device.completed}
            completedLabel={t("onboarding.completed")}
            actionLabel={t("onboarding.steps.device.action")}
            onAction={() => handleAction("/devices")}
          />
        </div>

        {/* Capability Overview */}
        <div className="w-full max-w-4xl mb-8">
          <div className="flex items-center gap-2 mb-4">
            <div className="flex-1 h-px bg-border" />
            <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
              {t("onboarding.moreCapabilities")}
            </span>
            <div className="flex-1 h-px bg-border" />
          </div>
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
            <CapabilityCard
              icon={<Workflow className="w-4 h-4" />}
              title={t("onboarding.capabilities.automation.title")}
              description={t("onboarding.capabilities.automation.description")}
            />
            <CapabilityCard
              icon={<LayoutDashboard className="w-4 h-4" />}
              title={t("onboarding.capabilities.dashboard.title")}
              description={t("onboarding.capabilities.dashboard.description")}
            />
            <CapabilityCard
              icon={<Bot className="w-4 h-4" />}
              title={t("onboarding.capabilities.agent.title")}
              description={t("onboarding.capabilities.agent.description")}
            />
            <CapabilityCard
              icon={<Bell className="w-4 h-4" />}
              title={t("onboarding.capabilities.notification.title")}
              description={t("onboarding.capabilities.notification.description")}
            />
          </div>
        </div>

        {/* Bottom hint */}
        <div className="w-full max-w-4xl mb-6">
          <div className="rounded-xl bg-muted-30 p-4 text-center">
            <p className="text-sm text-muted-foreground">
              {t("onboarding.hint")}
            </p>
          </div>
        </div>

        {/* Dismiss button */}
        <Button variant="ghost" size="sm" onClick={handleDismiss} className="text-muted-foreground">
          {t("onboarding.dismiss")}
        </Button>
      </div>
    </div>,
    root,
  )
}

// ── Sub-components ──

function StepCard({
  icon,
  title,
  description,
  purpose,
  completed,
  completedLabel,
  actionLabel,
  onAction,
}: {
  icon: React.ReactNode
  title: string
  description: string
  purpose: string
  completed: boolean
  completedLabel: string
  actionLabel: string
  onAction: () => void
}) {
  return (
    <div
      className={cn(
        "rounded-xl border p-5 transition-colors",
        completed
          ? "border-success/30 bg-success-light/30"
          : "border-border bg-card hover:border-primary/30"
      )}
    >
      <div className="flex items-start gap-3 mb-3">
        <div className={cn(
          "w-9 h-9 rounded-lg flex items-center justify-center shrink-0",
          completed ? "bg-success/10 text-success" : "bg-primary/10 text-primary"
        )}>
          {completed ? <Check className="w-5 h-5" /> : icon}
        </div>
        <div className="flex-1 min-w-0">
          <h3 className={cn("font-semibold text-sm", completed && "text-muted-foreground line-through")}>
            {title}
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
            {description}
          </p>
        </div>
      </div>
      {!completed && (
        <>
          <p className="text-xs text-muted-foreground mb-3 pl-12 leading-relaxed">
            {purpose}
          </p>
          <div className="flex justify-end">
            <Button size="sm" onClick={onAction} className="gap-1.5">
              {actionLabel}
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
              </svg>
            </Button>
          </div>
        </>
      )}
      {completed && (
        <div className="flex justify-end">
          <span className="text-xs font-medium text-success">
            <Check className="w-3.5 h-3.5 inline mr-1" />
            {completedLabel}
          </span>
        </div>
      )}
    </div>
  )
}

function CapabilityCard({
  icon,
  title,
  description,
}: {
  icon: React.ReactNode
  title: string
  description: string
}) {
  return (
    <div className="flex items-start gap-2.5 p-3 rounded-lg bg-muted-30">
      <div className="w-7 h-7 rounded-md bg-primary/10 flex items-center justify-center shrink-0 text-primary">
        {icon}
      </div>
      <div className="min-w-0">
        <h4 className="text-xs font-semibold text-foreground">{title}</h4>
        <p className="text-xs text-muted-foreground leading-relaxed mt-0.5">{description}</p>
      </div>
    </div>
  )
}
