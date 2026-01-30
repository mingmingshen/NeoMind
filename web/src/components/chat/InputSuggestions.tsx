/**
 * InputSuggestions - Context-aware suggestions while typing
 * Shows relevant prompts based on current input
 * Fetches suggestions from backend API for dynamic, context-aware recommendations
 */

import { useMemo, useEffect, useState, useCallback } from "react"
import {
  Lightbulb,
  Cpu,
  Zap,
  AlertTriangle,
  Settings,
  TrendingUp,
  History,
  Bot,
  LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

// Icon mapping for backend icon names
const ICON_MAP: Record<string, LucideIcon> = {
  Lightbulb,
  Cpu,
  Zap,
  AlertTriangle,
  Settings,
  TrendingUp,
  History,
  Bot,
}

interface BackendSuggestion {
  id: string
  label: string
  prompt: string
  icon: string
  category: string
}

interface SuggestionsResponse {
  suggestions: BackendSuggestion[]
  context: {
    timestamp: number
    learned_patterns_count: number
  }
}

interface Suggestion {
  id: string
  label: string
  prompt: string
  icon: React.ReactNode
  category: string
}

// Fallback predefined suggestions (used when API fails)
const FALLBACK_SUGGESTIONS: BackendSuggestion[] = [
  {
    id: "device-list",
    label: "查看所有设备",
    prompt: "查看所有设备状态",
    icon: "Cpu",
    category: "device"
  },
  {
    id: "automation-list",
    label: "查看自动化规则",
    prompt: "查看所有自动化规则",
    icon: "Zap",
    category: "automation"
  },
  {
    id: "alert-list",
    label: "查看告警",
    prompt: "查看当前告警",
    icon: "AlertTriangle",
    category: "alert"
  },
  {
    id: "help",
    label: "帮助",
    prompt: "你能做什么",
    icon: "Lightbulb",
    category: "general"
  }
]

interface InputSuggestionsProps {
  input: string
  onSelect: (prompt: string) => void
  visible: boolean
}

export function InputSuggestions({ input, onSelect, visible }: InputSuggestionsProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [backendSuggestions, setBackendSuggestions] = useState<BackendSuggestion[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [apiError, setApiError] = useState(false)

  // Fetch suggestions from backend API when component mounts or visibility changes
  const fetchSuggestions = useCallback(async () => {
    setIsLoading(true)
    setApiError(false)
    try {
      const response = await fetch(`/api/suggestions?input=${encodeURIComponent(input)}&limit=20`)
      if (response.ok) {
        const data: SuggestionsResponse = await response.json()
        setBackendSuggestions(data.suggestions)
      } else {
        throw new Error('API request failed')
      }
    } catch (error) {
      console.error('Failed to fetch suggestions:', error)
      setApiError(true)
      // Use fallback suggestions on error
      setBackendSuggestions(FALLBACK_SUGGESTIONS)
    } finally {
      setIsLoading(false)
    }
  }, [input])

  // Fetch suggestions when visible and input changes significantly
  useEffect(() => {
    if (visible) {
      fetchSuggestions()
    }
  }, [visible, fetchSuggestions])

  // Convert backend suggestions to frontend format with icon components
  const allSuggestions: Suggestion[] = useMemo(() => {
    return backendSuggestions.map(s => {
      const IconComponent = ICON_MAP[s.icon] || Lightbulb
      return {
        id: s.id,
        label: s.label,
        prompt: s.prompt,
        icon: <IconComponent className="h-4 w-4" />,
        category: s.category
      }
    })
  }, [backendSuggestions])

  // Additional client-side filtering for responsiveness
  const filteredSuggestions = useMemo(() => {
    // Backend already filtered by input, but we can add additional client-side filtering
    // if needed for instant responsiveness
    return allSuggestions
  }, [allSuggestions])

  // Reset selected index when filtered suggestions change
  useEffect(() => {
    setSelectedIndex(0)
  }, [filteredSuggestions])

  // Handle keyboard navigation
  useEffect(() => {
    if (!visible) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault()
        setSelectedIndex(prev =>
          prev < filteredSuggestions.length - 1 ? prev + 1 : prev
        )
      } else if (e.key === "ArrowUp") {
        e.preventDefault()
        setSelectedIndex(prev => (prev > 0 ? prev - 1 : 0))
      } else if (e.key === "Enter" && filteredSuggestions.length > 0) {
        e.preventDefault()
        onSelect(filteredSuggestions[selectedIndex].prompt)
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [visible, filteredSuggestions, selectedIndex, onSelect])

  if (!visible || filteredSuggestions.length === 0) {
    return null
  }

  return (
    <div className="absolute bottom-full left-0 right-0 mb-2 z-10">
      <div className={cn(
        "bg-[var(--popover)] border border-[var(--border)] rounded-lg",
        "shadow-lg overflow-hidden",
        "animate-in slide-in-from-bottom-2 duration-200"
      )}>
        {/* Header with loading indicator */}
        {input.trim() && (
          <div className="px-3 py-2 border-b border-[var(--border)] flex items-center justify-between">
            <p className="text-xs text-muted-foreground">
              按 <kbd className="px-1 py-0.5 rounded bg-[var(--muted)] text-[var(--muted-foreground)]">↑↓</kbd> 导航，
              <kbd className="px-1 py-0.5 rounded bg-[var(--muted)] text-[var(--muted-foreground)] ml-1">Enter</kbd> 选择
            </p>
            {isLoading && (
              <span className="text-xs text-muted-foreground">加载中...</span>
            )}
          </div>
        )}

        {/* Pattern suggestions indicator */}
        {!input.trim() && backendSuggestions.some(s => s.category === "agent") && (
          <div className="px-3 py-1.5 border-b border-[var(--border)] bg-[var(--muted)]/30">
            <p className="text-xs text-muted-foreground flex items-center gap-1.5">
              <History className="h-3 w-3" />
              基于历史操作的建议
            </p>
          </div>
        )}

        {/* Suggestions list */}
        <div className="max-h-48 overflow-y-auto">
          {filteredSuggestions.map((suggestion, index) => (
            <button
              key={suggestion.id}
              onClick={() => onSelect(suggestion.prompt)}
              onMouseEnter={() => setSelectedIndex(index)}
              className={cn(
                "w-full flex items-center gap-3 px-3 py-2.5 text-left",
                "transition-colors duration-150",
                index === selectedIndex
                  ? "bg-[var(--accent)] text-[var(--accent-foreground)]"
                  : "hover:bg-[var(--accent)]/50"
              )}
            >
              <div className={cn(
                "flex-shrink-0 p-1.5 rounded-md",
                index === selectedIndex
                  ? "bg-[var(--accent)]/30"
                  : "bg-[var(--muted)]"
              )}>
                {suggestion.icon}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{suggestion.label}</p>
                <p className="text-xs text-muted-foreground truncate">{suggestion.prompt}</p>
              </div>
            </button>
          ))}

          {/* Loading skeleton */}
          {isLoading && filteredSuggestions.length === 0 && (
            <>
              {[1, 2, 3].map((i) => (
                <div key={i} className="flex items-center gap-3 px-3 py-2.5">
                  <div className="h-6 w-6 rounded-md bg-[var(--muted)] animate-pulse" />
                  <div className="flex-1">
                    <div className="h-4 w-32 bg-[var(--muted)] animate-pulse rounded mb-1" />
                    <div className="h-3 w-24 bg-[var(--muted)] animate-pulse rounded" />
                  </div>
                </div>
              ))}
            </>
          )}
        </div>
      </div>
    </div>
  )
}
