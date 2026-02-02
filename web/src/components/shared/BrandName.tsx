/**
 * BrandName Components
 *
 * Provides consistent brand name display across the application.
 * Uses centralized i18n configuration for easy rebranding.
 *
 * Logo style inspired: black background, white text, orange accent
 * Supports light/dark themes
 */

import { useBrand, useBrandMessages } from '@/hooks/useBrand'
import { cn } from '@/lib/utils'

/**
 * Display the full brand name
 *
 * @example
 * ```tsx
 * <BrandName />
 * <BrandName className="text-xl font-bold" />
 * ```
 */
export function BrandName({ className }: { className?: string }) {
  const { name } = useBrand()
  return <span className={className}>{name}</span>
}

/**
 * Display the short brand name/acronym
 *
 * @example
 * ```tsx
 * <ShortBrandName />
 * <ShortBrandName className="text-xs" />
 * ```
 */
export function ShortBrandName({ className }: { className?: string }) {
  const { shortName } = useBrand()
  return <span className={cn('font-mono', className)}>{shortName}</span>
}

/**
 * Brand logo with short name
 *
 * @example
 * ```tsx
 * <BrandLogo />
 * ```
 */
export function BrandLogo({ className }: { className?: string }) {
  const { shortName } = useBrand()
  return (
    <div className={cn('w-9 h-9 rounded-xl bg-foreground flex items-center justify-center shadow-sm', className)}>
      <span className="text-background font-semibold text-sm">{shortName}</span>
    </div>
  )
}

/**
 * Styled brand name with logo-inspired design
 * - Light mode: white text with orange shadow/stroke
 * - Dark mode: black text with orange shadow/stroke
 *
 * @example
 * ```tsx
 * <StyledBrandName />
 * <StyledBrandName size="sm" />
 * ```
 */
export function StyledBrandName({
  size = 'base',
  className
}: {
  size?: 'sm' | 'base' | 'lg'
  className?: string
}) {
  const { name } = useBrand()

  const sizeClasses = {
    sm: 'text-sm',
    base: 'text-base',
    lg: 'text-lg'
  }

  return (
    <span
      className={cn(
        'inline-block font-black italic tracking-tight',
        // Light mode - black text
        'text-black',
        // Dark mode - white text
        'dark:text-white',
        // Orange shadow effect (3D) - top-right direction with #EB4C27
        'drop-shadow-[1.5px_-1.5px_0_#EB4C27]',
        // Transition
        'transition-all duration-200',
        sizeClasses[size],
        className
      )}
    >
      {name}
    </span>
  )
}

/**
 * Combined brand logo and name
 *
 * @example
 * ```tsx
 * <BrandLogoWithName />
 * ```
 */
export function BrandLogoWithName({
  logoClassName,
  nameClassName,
  showLogo = false,
  styled = true
}: {
  logoClassName?: string
  nameClassName?: string
  showLogo?: boolean
  styled?: boolean
}) {
  const { name } = useBrand()

  if (styled) {
    return (
      <StyledBrandName className={nameClassName} />
    )
  }

  return (
    <div className="flex items-center gap-2">
      <BrandLogo className={cn('sm:hidden', logoClassName)} />
      <BrandName className={cn(
        'font-bold text-lg tracking-tight text-foreground',
        nameClassName
      )} />
    </div>
  )
}
