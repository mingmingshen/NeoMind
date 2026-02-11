import React, { useRef, useState, useEffect } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import type { Components } from 'react-markdown'
import { ChevronDown } from 'lucide-react'
import { cn } from "@/lib/utils"

interface MarkdownMessageProps {
  content: string
  className?: string
  variant?: 'user' | 'assistant'
  isStreaming?: boolean
}

/**
 * Remove duplicated content when the same text appears twice in one message
 * (e.g. model repetition or backend sending same chunk twice).
 */
function dedupeRepeatedContent(content: string): string {
  const s = (content || '').trim()
  if (s.length < 2) return content
  const half = Math.floor(s.length / 2)
  const first = s.slice(0, half)
  const second = s.slice(half)
  if (first === second) return first
  return content
}

/**
 * Fade gradient indicator for scrollable content
 */
function ScrollFadeIndicator({
  show
}: {
  show: boolean
}) {
  if (!show) return null

  return (
    <div className="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-inherit to-transparent pointer-events-none opacity-50" />
  )
}

/**
 * Markdown message renderer with support for:
 * - GitHub Flavored Markdown (GFM) via remark-gfm
 * - Code blocks with syntax highlighting
 * - Tables, lists, links, images
 * - Styled for chat interface
 * - Auto-scrolls during streaming, fully expands after
 */
export function MarkdownMessage({ content, className, variant = 'assistant', isStreaming = false }: MarkdownMessageProps) {
  const displayContent = dedupeRepeatedContent(content)
  const containerRef = useRef<HTMLDivElement>(null)
  const [isOverflowing, setIsOverflowing] = useState(false)

  // Detect if content overflows
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const checkOverflow = () => {
      setIsOverflowing(container.scrollHeight > container.clientHeight)
    }

    checkOverflow()

    // Re-check on resize
    const resizeObserver = new ResizeObserver(checkOverflow)
    resizeObserver.observe(container)

    return () => {
      resizeObserver.disconnect()
    }
  }, [displayContent])

  // Auto-scroll to bottom during streaming
  useEffect(() => {
    if (isStreaming && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight
    }
  }, [displayContent, isStreaming])

  const components: Components = {
    // Custom code block rendering
    pre: ({ node, className, children, ...props }) => (
      <pre className={cn("overflow-x-auto", className)} {...(props as any)}>
        {children}
      </pre>
    ),
    // Custom inline code
    code: ({ node, className, children, ...props }) => {
      const inline = (props as any).inline
      if (inline) {
        return (
          <code className={cn("bg-muted px-1 py-0.5 rounded text-xs font-mono", className)} {...(props as any)}>
            {children}
          </code>
        )
      }
      return (
        <code className={cn(className)} {...(props as any)}>
          {children}
        </code>
      )
    },
    // Custom link rendering
    a: ({ node, className, children, href, ...props }) => (
      <a
        className={cn("text-primary hover:underline", className)}
        href={href as string}
        target="_blank"
        rel="noopener noreferrer"
        {...(props as any)}
      >
        {children}
      </a>
    ),
  }

  return (
    <div className={cn("relative", className)}>
      <div
        ref={containerRef}
        className={cn(
          // Custom scrollbar styling
          "scrollbar-thin scrollbar-thumb-muted-foreground/20 hover:scrollbar-thumb-muted-foreground/40 scrollbar-track-transparent",
          // Base prose classes
          "prose prose-sm max-w-none",
          // Text wrapping
          "break-words overflow-wrap-anywhere",
          "prose-p:leading-relaxed prose-p:my-1",
          "prose-headings:font-semibold prose-headings:my-2",
          "prose-h1:text-base prose-h2:text-sm prose-h3:text-xs",
          "prose-a:text-primary prose-a:no-underline hover:prose-a:underline",
          "prose-strong:font-semibold",
          "prose-code:rounded prose-code:bg-muted prose-code:px-1 prose-code:py-0.5 prose-code:text-xs prose-code:font-mono",
          "prose-code:break-all prose-code:whitespace-pre-wrap",
          "prose-pre:bg-muted prose-pre:p-3 prose-pre:rounded-lg prose-pre:my-2",
          "prose-pre:overflow-x-auto prose-pre:max-w-full",
          "prose-pre:prose-code:bg-transparent prose-pre:prose-code:p-0",
          "prose-blockquote:border-l-2 prose-blockquote:border-muted-foreground prose-blockquote:pl-3 prose-blockquote:italic",
          "prose-ul:my-1 prose-ul:pl-4 prose-ul:list-disc",
          "prose-ol:my-1 prose-ol:pl-4 prose-ol:list-decimal",
          "prose-li:my-0.5",
          "prose-table:my-2 prose-table:text-xs",
          "prose-th:px-2 prose-th:py-1 prose-th:border prose-th:border-border prose-th:bg-muted/50",
          "prose-td:px-2 prose-td:py-1 prose-td:border prose-td:border-border",
          "prose-hr:my-2 prose-hr:border-border",
          // Max height for long content with smooth scroll
          "max-h-[600px] overflow-y-auto"
        )}
        style={{ color: 'inherit' }}
        data-variant={variant}
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
          {displayContent}
        </ReactMarkdown>
      </div>

      {/* Subtle fade indicator when content is scrollable */}
      {isOverflowing && !isStreaming && (
        <ScrollFadeIndicator show={true} />
      )}
    </div>
  )
}
