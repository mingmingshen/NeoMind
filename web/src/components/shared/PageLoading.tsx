import { Loader2 } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface PageLoadingProps {
  /** Optional text below the spinner */
  text?: string
  className?: string
  /** Size variant */
  size?: 'sm' | 'md' | 'lg'
}

const sizeStyles = {
  sm: 'h-5 w-5',
  md: 'h-8 w-8',
  lg: 'h-10 w-10',
}

/**
 * Centered loading indicator for page content
 *
 * Uses flex-1 to fill the parent's flex column (PageLayout's scroll container),
 * ensuring the spinner is vertically & horizontally centered regardless of
 * browser/WebView (Chrome, Tauri WKWebView, etc.).
 */
export function PageLoading({ text, className, size = 'md' }: PageLoadingProps) {
  return (
    <div className={cn('flex flex-col items-center justify-center flex-1 px-4', className)}>
      <div className="relative">
        <div className="absolute inset-0 blur-xl bg-primary/20 rounded-full" />
        <Loader2 className={cn('animate-spin text-primary relative', sizeStyles[size])} />
      </div>
      {text && (
        <p className="mt-4 text-sm text-muted-foreground animate-pulse">{text}</p>
      )}
    </div>
  )
}
