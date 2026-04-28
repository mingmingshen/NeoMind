/**
 * ProgressStepper Component
 *
 * Unified step progress indicator for automation wizards.
 * Glassmorphism design with clean visual hierarchy.
 */

import { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useIsMobile } from '@/hooks/useMobile'

export type StepStatus = 'pending' | 'active' | 'completed'

export interface Step {
  id: string
  label: string
  shortLabel?: string
  icon?: ReactNode
  optional?: boolean
}

export interface ProgressStepperProps {
  steps: Step[]
  currentStep: string
  completedSteps: string[]
  onStepClick?: (stepId: string) => void
  className?: string
  /** Layout mode */
  mode?: 'horizontal' | 'vertical'
}

// ============================================================================
// Step Dot Component
// ============================================================================

function StepDot({
  status,
  icon,
  size = 'default',
}: {
  status: StepStatus
  icon?: ReactNode
  size?: 'default' | 'large'
}) {
  const sizeClasses = size === 'large'
    ? 'w-9 h-9 text-sm'
    : 'w-6 h-6 text-xs'

  return (
    <div
      className={cn(
        'rounded-full flex items-center justify-center font-medium transition-all shrink-0',
        sizeClasses,
        status === 'pending' && 'bg-black/5 dark:bg-white/10 text-muted-foreground',
        status === 'active' && [
          'bg-primary text-primary-foreground',
          'ring-4 ring-primary',
        ],
        status === 'completed' && 'bg-success text-white'
      )}
    >
      {status === 'completed' ? (
        <Check className={size === 'large' ? 'h-4.5 w-4.5' : 'h-4 w-4'} />
      ) : icon ? (
        icon
      ) : null}
    </div>
  )
}

// ============================================================================
// Vertical Stepper (Sidebar)
// ============================================================================

export interface VerticalStepperProps {
  steps: Step[]
  currentStep: string
  completedSteps: string[]
  onStepClick?: (stepId: string) => void
  className?: string
}

export function VerticalStepper({
  steps,
  currentStep,
  completedSteps,
  onStepClick,
  className,
}: VerticalStepperProps) {
  const isMobile = useIsMobile()

  const getStepStatus = (step: Step): StepStatus => {
    if (completedSteps.includes(step.id)) return 'completed'
    if (currentStep === step.id) return 'active'
    return 'pending'
  }

  return (
    <nav className={cn('p-4 space-y-1', className)}>
      {steps.map((step, index) => {
        const status = getStepStatus(step)
        const isClickable = onStepClick && (completedSteps.includes(step.id) || status === 'active')
        const isPast = index < steps.findIndex(s => s.id === currentStep)

        return (
          <div key={step.id} className="relative">
            <button
              onClick={() => isClickable && onStepClick?.(step.id)}
              disabled={!isClickable}
              className={cn(
                'w-full text-left rounded-xl transition-all',
                'flex items-center gap-3 px-3 py-3',
                status === 'active' && [
                  'bg-white/60 dark:bg-white/5',
                  'shadow-sm',
                  'border border-border',
                ],
                status !== 'active' && isClickable && 'hover:bg-black/5 dark:hover:bg-white/5',
                !isClickable && 'cursor-default opacity-60'
              )}
            >
              <StepDot status={status} icon={step.icon} size="large" />
              <div className="flex-1 min-w-0">
                <div className={cn(
                  'text-sm font-medium truncate',
                  status === 'active' && 'text-foreground',
                  status !== 'active' && 'text-muted-foreground'
                )}>
                  {step.shortLabel || step.label}
                </div>
              </div>
            </button>

            {/* Connector line */}
            {index < steps.length - 1 && (
              <div className="absolute left-[calc(1.5rem+18px)] top-[52px] h-3 w-px">
                <div
                  className={cn(
                    'h-full w-px transition-colors',
                    isPast || status === 'completed' ? 'bg-success' : 'bg-border'
                  )}
                />
              </div>
            )}
          </div>
        )
      })}
    </nav>
  )
}

// ============================================================================
// Horizontal Stepper (Top Bar)
// ============================================================================

export interface HorizontalStepperProps {
  steps: Step[]
  currentStep: string
  completedSteps: string[]
  onStepClick?: (stepId: string) => void
  className?: string
}

export function HorizontalStepper({
  steps,
  currentStep,
  completedSteps,
  onStepClick,
  className,
}: HorizontalStepperProps) {
  const { t } = useTranslation(['common'])
  const isMobile = useIsMobile()

  const getStepStatus = (step: Step): StepStatus => {
    if (completedSteps.includes(step.id)) return 'completed'
    if (currentStep === step.id) return 'active'
    return 'pending'
  }

  const currentIndex = steps.findIndex(s => s.id === currentStep)

  return (
    <div
      className={cn(
        'shrink-0 px-5 md:px-6 py-4',
        'border-b border-border',
        'bg-black/[0.02] dark:bg-white/[0.02]',
        className
      )}
    >
      <div className="flex items-center justify-center gap-2 md:gap-3 overflow-x-auto">
        {steps.map((step, i) => {
          const status = getStepStatus(step)
          const isClickable = onStepClick && (completedSteps.includes(step.id) || i === currentIndex + 1)
          const isPast = i < currentIndex

          return (
            <div key={step.id} className="flex items-center">
              <button
                onClick={() => isClickable && onStepClick?.(step.id)}
                disabled={!isClickable}
                className={cn(
                  'flex flex-col items-center gap-2 transition-all',
                  isClickable && 'hover:opacity-80 cursor-pointer',
                  !isClickable && 'cursor-default'
                )}
              >
                <StepDot status={status} icon={step.icon} />
                <span
                  className={cn(
                    'text-xs whitespace-nowrap',
                    status === 'active' && 'font-semibold text-foreground',
                    status === 'completed' && 'text-foreground',
                    status === 'pending' && 'text-muted-foreground'
                  )}
                >
                  {isMobile && step.shortLabel ? step.shortLabel : step.label}
                  {step.optional && (
                    <span className="text-muted-foreground ml-1">
                      ({t('common:optional')})
                    </span>
                  )}
                </span>
              </button>

              {/* Connector */}
              {i < steps.length - 1 && (
                <div
                  className={cn(
                    'h-0.5 w-8 md:w-12 mx-2 rounded-full transition-colors shrink-0',
                    (status === 'completed' || isPast) ? 'bg-green-500' : 'bg-border'
                  )}
                />
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}

// ============================================================================
// Main ProgressStepper Component
// ============================================================================

export function ProgressStepper({
  steps,
  currentStep,
  completedSteps,
  onStepClick,
  className,
  mode = 'horizontal',
}: ProgressStepperProps) {
  if (mode === 'vertical') {
    return (
      <VerticalStepper
        steps={steps}
        currentStep={currentStep}
        completedSteps={completedSteps}
        onStepClick={onStepClick}
        className={className}
      />
    )
  }

  return (
    <HorizontalStepper
      steps={steps}
      currentStep={currentStep}
      completedSteps={completedSteps}
      onStepClick={onStepClick}
      className={className}
    />
  )
}
