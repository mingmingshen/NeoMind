import { ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { PageHeader } from '@/components/layout/PageHeader'

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
  maxWidth?: 'md' | 'lg' | 'xl' | '2xl' | 'full'
  className?: string
  /** Whether to render a subtle bottom border under the header */
  borderedHeader?: boolean
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
  maxWidth = 'full',
  className,
  borderedHeader = false,
}: PageLayoutProps) {
  return (
    <div className="flex flex-col h-full">
      {title && (
        <div className="shrink-0 bg-background">
          <div className={cn('w-full px-4 py-4 sm:px-6 sm:py-5 md:px-8 md:py-6', maxWidthClass[maxWidth], className)}>
            <PageHeader
              title={title}
              description={subtitle}
              actions={actions}
              variant={borderedHeader ? 'bordered' : 'default'}
            />
          </div>
        </div>
      )}
      {/* Content area - uses flex-col to push sticky elements to bottom when content is short */}
      <div className={cn('flex-1 flex flex-col min-h-0', className)}>
        {/* Scrollable content */}
        <div className={cn('flex-1 overflow-auto px-4 sm:px-6 md:px-8', footer ? 'pb-20' : 'pb-4 sm:pb-6')}>
          <div className={cn('mx-auto w-full space-y-6', maxWidthClass[maxWidth])}>
            {children}
          </div>
        </div>
      </div>
      {/* Fixed footer with glass morphism effect */}
      {footer && (
        <div className="fixed bottom-0 left-0 right-0 bg-gradient-to-t from-background via-background/95 to-background/80 backdrop-blur-md border-t border-border/30">
          <div className={cn('w-full px-4 py-3 sm:px-6 sm:py-4 md:px-8', maxWidthClass[maxWidth], className)}>
            {footer}
          </div>
        </div>
      )}
    </div>
  )
}
