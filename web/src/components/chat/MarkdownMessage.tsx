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
 * Markdown message renderer with support for:
 * - GitHub Flavored Markdown (GFM) via remark-gfm
 * - Code blocks with syntax highlighting
 * - Tables, lists, links, images
 * - Styled for chat interface
 */
export function MarkdownMessage({ content, className, variant = 'assistant' }: MarkdownMessageProps) {
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
    <div className={cn(
      // Base prose classes
      "prose prose-sm dark:prose-invert max-w-none",
      // Override prose text color to inherit from parent
      variant === 'user' && "prose-p:text-inherit prose-li:text-inherit prose-span:text-inherit prose-strong:text-inherit",
      "prose-p:leading-relaxed prose-p:my-1",
      "prose-headings:font-semibold prose-headings:my-2",
      "prose-h1:text-base prose-h2:text-sm prose-h3:text-xs",
      "prose-a:text-primary prose-a:no-underline hover:prose-a:underline",
      "prose-strong:font-semibold",
      "prose-code:rounded prose-code:bg-muted prose-code:px-1 prose-code:py-0.5 prose-code:text-xs prose-code:font-mono",
      "prose-pre:bg-muted prose-pre:p-3 prose-pre:rounded-lg prose-pre:my-2",
      "prose-pre:prose-code:bg-transparent prose-pre:prose-code:p-0",
      "prose-blockquote:border-l-2 prose-blockquote:border-muted-foreground prose-blockquote:pl-3 prose-blockquote:italic",
      "prose-ul:my-1 prose-ul:pl-4 prose-ul:list-disc",
      "prose-ol:my-1 prose-ol:pl-4 prose-ol:list-decimal",
      "prose-li:my-0.5",
      "prose-table:my-2 prose-table:text-xs",
      "prose-th:px-2 prose-th:py-1 prose-th:border prose-th:border-border prose-th:bg-muted/50",
      "prose-td:px-2 prose-td:py-1 prose-td:border prose-td:border-border",
      "prose-hr:my-2 prose-hr:border-border",
      className
    )}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </ReactMarkdown>
    </div>
  )
}
