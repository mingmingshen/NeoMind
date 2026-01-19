// ThinkingBlock component - displays AI thinking process
import { useState } from "react"
import { Brain, ChevronDown, ChevronUp } from "lucide-react"
import { cn } from "@/lib/utils"

interface ThinkingBlockProps {
  thinking: string
  isStreaming?: boolean
  defaultExpanded?: boolean
}

export function ThinkingBlock({
  thinking,
  isStreaming = false,
  defaultExpanded = false
}: ThinkingBlockProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded)

  if (!thinking) return null

  return (
    <div className={cn(
      "thinking-block rounded-lg border overflow-hidden transition-colors",
      isExpanded ? "border-blue-200 bg-blue-50/50" : "border-muted bg-muted/30"
    )}>
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-muted/50 transition-colors"
      >
        <Brain className={cn(
          "h-4 w-4",
          isStreaming ? "animate-pulse text-blue-500" : "text-muted-foreground"
        )} />
        <span className="text-sm font-medium text-muted-foreground">
          思考过程
        </span>
        {isStreaming && (
          <span className="text-xs text-muted-foreground/70">
            思考中...
          </span>
        )}
        <span className="ml-auto text-xs text-muted-foreground">
          {thinking.length} 字符
        </span>
        {isExpanded ? (
          <ChevronUp className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        )}
      </button>

      {isExpanded && (
        <div className="px-3 pb-3">
          <div className="prose prose-sm max-w-none text-sm">
            <div className={cn(
              "thinking-content whitespace-pre-wrap break-words",
              "text-gray-700 dark:text-gray-300"
            )}>
              {thinking}
              {isStreaming && <span className="animate-pulse">▌</span>}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
