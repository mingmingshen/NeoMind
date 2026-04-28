import React from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import type { Components } from 'react-markdown'
import { cn } from "@/lib/utils"

interface MarkdownMessageProps {
  content: string
  className?: string
  variant?: 'user' | 'assistant'
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
 * Markdown message renderer with support for:
 * - GitHub Flavored Markdown (GFM) via remark-gfm
 * - Code blocks with syntax highlighting
 * - Tables, lists, links, images
 * - Styled for chat interface
 * - Auto-scrolls during streaming, fully expands after
 */
export function MarkdownMessage({ content, className, variant = 'assistant' }: MarkdownMessageProps) {
  const displayContent = dedupeRepeatedContent(content)

  const components: Components = {
    // Custom code block rendering
    pre: ({ node, className, children, ...props }) => (
      <pre className={cn("overflow-x-auto", className)} {...(props as any)}>
        {children}
      </pre>
    ),
    // Custom inline code
    code: ({ node, className, children, ...props }) => {
      const isBlock = !!className
      if (!isBlock) {
        return (
          <code className={cn("bg-muted px-1 py-0.5 rounded text-[12px] font-mono text-foreground", className)} {...(props as any)}>
            {children}
          </code>
        )
      }
      return (
        <code className={cn("text-foreground", className)} {...(props as any)}>
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
        className={cn(
          // Base prose classes — use prose for structure, override size to 13px
          "prose max-w-none text-[13px]",
          // Text wrapping
          "break-words overflow-wrap-anywhere",
          "prose-p:leading-relaxed prose-p:my-1",
          "prose-headings:font-semibold prose-headings:my-2",
          "prose-h1:text-[15px] prose-h2:text-[13px] prose-h3:text-[12px]",
          "prose-a:text-primary prose-a:no-underline hover:prose-a:underline",
          "prose-strong:font-semibold",
          "prose-code:rounded prose-code:bg-muted prose-code:px-1 prose-code:py-0.5 prose-code:text-[12px] prose-code:font-mono",
          "prose-code:break-all prose-code:whitespace-pre-wrap",
          "prose-pre:bg-muted prose-pre:p-3 prose-pre:rounded-lg prose-pre:my-2",
          "prose-pre:overflow-x-auto prose-pre:max-w-full",
          "prose-pre:prose-code:bg-transparent prose-pre:prose-code:p-0 prose-pre:prose-code:text-foreground",
          "prose-blockquote:border-l-2 prose-blockquote:border-muted-foreground prose-blockquote:pl-3 prose-blockquote:italic",
          "prose-ul:my-1 prose-ul:pl-4 prose-ul:list-disc",
          "prose-ol:my-1 prose-ol:pl-4 prose-ol:list-decimal",
          "prose-li:my-0.5",
          "prose-table:my-2 prose-table:text-[12px]",
          "prose-th:px-2 prose-th:py-1 prose-th:border prose-th:border-border prose-th:bg-muted-50",
          "prose-td:px-2 prose-td:py-1 prose-td:border prose-td:border-border",
          "prose-hr:my-2 prose-hr:border-border"
          // Removed max height limit - messages now fully expand
        )}
        style={{ color: 'inherit' }}
        data-variant={variant}
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
          {displayContent}
        </ReactMarkdown>
      </div>
    </div>
  )
}
