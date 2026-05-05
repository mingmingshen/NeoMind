import { ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { PageHeader } from '@/components/layout/PageHeader'
import { useIsMobile } from '@/hooks/useMobile'

export interface PageLayoutProps {
  children: ReactNode
  /** Optional page title, rendered via PageHeader */
  title?: string
  /** Optional secondary description text below the title */
  subtitle?: string
  /** Optional actions area rendered on the right of the header (buttons, filters, etc.) */
  actions?: ReactNode
  /** Optional footer content (e.g., pagination bar fixed at bottom) */
  footer?: ReactNode
  /** Optional fixed header content (e.g., tabs) - rendered between title and scrollable content */
  headerContent?: ReactNode
  maxWidth?: 'md' | 'lg' | 'xl' | '2xl' | 'full'
  className?: string
  /** Whether to render a subtle bottom border under the header */
  borderedHeader?: boolean
  /** Whether to hide footer on mobile (for infinite scroll) */
  hideFooterOnMobile?: boolean
  /** Whether to fix actions bar on mobile (don't scroll with content) */
  fixedActionsOnMobile?: boolean
  /** Whether to remove scroll container padding (for full-bleed children like detail views) */
  noPadding?: boolean
  /** Whether page has a bottom tab navigation bar (mobile) - adds extra bottom padding */
  hasBottomNav?: boolean
}

const maxWidthClass = {
  md: 'max-w-4xl',
  lg: 'max-w-6xl',
  xl: 'max-w-7xl',
  '2xl': 'max-w-7xl',
  full: 'max-w-full',
}

/**
 * Standard page layout container
 *
 * Provides consistent padding, max-width, and optional header across all pages.
 *
 * @example
 * <PageLayout
 *   title="Devices"
 *   subtitle="Manage all connected devices"
 *   actions={<Button size="sm">Refresh</Button>}
 *   maxWidth="xl"
 *   footer={<Pagination />}
 *   hideFooterOnMobile
 * >
 *   <div>Content here</div>
 * </PageLayout>
 */
export function PageLayout({
  children,
  title,
  subtitle,
  actions,
  footer,
  headerContent,
  maxWidth = 'full',
  className,
  borderedHeader = false,
  hideFooterOnMobile = false,
  fixedActionsOnMobile = false,
  noPadding = false,
  hasBottomNav = false,
}: PageLayoutProps) {
  const isMobile = useIsMobile()

  // Determine if footer should be shown
  const showFooter = footer && !(isMobile && hideFooterOnMobile)

  // Bottom spacer height: uses inline style to avoid CSS specificity issues
  // with safe-bottom/pb-* classes overriding each other
  const bottomSpacerHeight = hasBottomNav && isMobile
    ? 'calc(8rem + env(safe-area-inset-bottom, 0px))'  // bottom nav + safe area
    : showFooter
      ? '14rem'                                          // footer clearance (224px)
      : isMobile
        ? 'calc(1.5rem + env(safe-area-inset-bottom, 0px))'
        : '2rem'

  return (
    <div className="flex flex-col h-full">
      {title && !(isMobile) && (
        <div className="shrink-0">
          <div className={cn('w-full px-4 pt-4 pb-2 sm:px-6 sm:pt-5 sm:pb-3 md:px-8 md:pt-6 md:pb-3', maxWidthClass[maxWidth], className)}>
            <PageHeader
              title={title}
              description={subtitle}
              actions={actions}
              variant={borderedHeader ? 'bordered' : 'default'}
            />
          </div>
        </div>
      )}
      {/* Fixed header content (e.g., tabs) - outside scroll container */}
      {headerContent && (
        <div className="shrink-0">
          {headerContent}
        </div>
      )}
      {/* Content area */}
      <div className={cn('flex-1 flex flex-col min-h-0', className)}>
        {/* Scrollable content */}
        <div
          className={cn(
            'flex-1 flex flex-col overflow-auto',
            !noPadding && 'px-4 sm:px-6 md:px-8',
            !noPadding && isMobile && 'pt-1',
          )}
          data-page-scroll-container
        >
          <div className={cn('mx-auto w-full flex flex-col min-h-full', maxWidthClass[maxWidth])}>
            {children}
            {/* Bottom spacer: ensures content isn't hidden behind fixed footer/nav */}
            {!noPadding && (
              <div className="shrink-0" style={{ height: bottomSpacerHeight }} />
            )}
          </div>
        </div>
      </div>
      {/* Fixed footer with glass morphism effect */}
      {showFooter ? (
        <div className="fixed bottom-0 left-0 right-0 bg-surface-glass backdrop-blur-xl border-t border-glass-border safe-bottom z-10">
          <div className={cn('w-full px-4 py-4 sm:px-6 sm:py-5 md:px-8', maxWidthClass[maxWidth], className)}>
            {footer}
          </div>
        </div>
      ) : isMobile && hideFooterOnMobile && footer ? (
        /* Hidden mount point for footer content (e.g., Pagination) so hooks like
           useWindowScrollLoad still run for mobile infinite scroll */
        <div className="hidden">{footer}</div>
      ) : null}
    </div>
  )
}
