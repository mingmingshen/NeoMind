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
 * >
 *   <div>Content here</div>
 * </PageLayout>
 */
export function PageLayout({
  children,
  title,
  subtitle,
  actions,
  maxWidth = 'full',
  className,
  borderedHeader = false,
}: PageLayoutProps) {
  return (
    <div className={cn('mx-auto p-6 md:p-8 pb-24 space-y-6', maxWidthClass[maxWidth], className)}>
      {title && (
        <PageHeader
          title={title}
          description={subtitle}
          actions={actions}
          variant={borderedHeader ? 'bordered' : 'default'}
        />
      )}
      {children}
    </div>
  )
}
