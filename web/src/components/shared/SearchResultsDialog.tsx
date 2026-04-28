import { getPortalRoot } from '@/lib/portal'
import { useEffect, useState, useCallback } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Search, Loader2, FileText, Wrench, AlertTriangle, X } from "lucide-react"
import { api } from "@/lib/api"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import type { SearchResult } from "@/types"
import { useNavigate } from "react-router-dom"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { dialogHeader } from '@/design-system/tokens/size'

interface SearchResultsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  initialQuery?: string
}

export function SearchResultsDialog({ open, onOpenChange, initialQuery = "" }: SearchResultsDialogProps) {
  const { t } = useTranslation('common')
  const { handleError } = useErrorHandler()
  const navigate = useNavigate()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [query, setQuery] = useState(initialQuery)
  const [results, setResults] = useState<SearchResult[]>([])
  const [loading, setLoading] = useState(false)
  const [hasSearched, setHasSearched] = useState(false)

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    if (open && initialQuery) {
      setQuery(initialQuery)
      performSearch(initialQuery)
    } else if (!open) {
      setQuery("")
      setResults([])
      setHasSearched(false)
    }
  }, [open, initialQuery])

  const performSearch = async (searchQuery: string) => {
    if (!searchQuery.trim()) return

    setLoading(true)
    setHasSearched(true)
    try {
      const response = await api.globalSearch(searchQuery)
      setResults(response.results)
    } catch (error) {
      handleError(error, { operation: 'Global search', showToast: false })
    } finally {
      setLoading(false)
    }
  }

  const handleResultClick = (result: SearchResult) => {
    onOpenChange(false)
    switch (result.type) {
      case "device":
        navigate("/devices")
        break
      case "rule":
        navigate("/automation")
        break
      case "alert":
        navigate("/alerts")
        break
    }
  }

  const handleSearch = () => {
    performSearch(query)
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      performSearch(query)
    }
  }

  const handleClose = useCallback(() => {
    onOpenChange(false)
  }, [onOpenChange])

  const getTypeIcon = (type: SearchResult["type"]) => {
    const icons = {
      device: <Wrench className="h-4 w-4" />,
      rule: <FileText className="h-4 w-4" />,
      alert: <AlertTriangle className="h-4 w-4" />,
    }
    return icons[type] || <FileText className="h-4 w-4" />
  }

  const getTypeLabel = (type: SearchResult["type"]) => {
    const labels = {
      device: t('search.types.device', { defaultValue: 'Device' }),
      rule: t('search.types.rule', { defaultValue: 'Rule' }),
      alert: t('search.types.alert', { defaultValue: 'Alert' }),
    }
    return labels[type] || type
  }

  const getTypeColor = (type: SearchResult["type"]) => {
    const colors = {
      device: "bg-info-light text-info border-info",
      rule: "bg-accent-purple-light text-accent-purple border-accent-purple-light",
      alert: "bg-error-light text-error border-error",
    }
    return colors[type] || "bg-muted text-muted-foreground border-border"
  }

  // Group results by type
  const groupedResults = results.reduce((acc, result) => {
    if (!acc[result.type]) {
      acc[result.type] = []
    }
    acc[result.type].push(result)
    return acc
  }, {} as Record<string, SearchResult[]>)

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-50 bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className={dialogHeader}
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <Search className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('search.title', { defaultValue: 'Global Search' })}</h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {t('search.description', { defaultValue: 'Search devices, rules, alerts' })}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <div className="space-y-4">
      {/* Search Input */}
      <div className="flex gap-2">
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t('placeholders.search')}
          className="flex-1"
          autoFocus
        />
        <Button onClick={handleSearch} disabled={loading || !query.trim()}>
          {loading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
        </Button>
      </div>

      {/* Results */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : hasSearched && results.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Search className="h-12 w-12 text-muted-foreground mb-4" />
          <p className="text-lg font-medium">{t('search.noResults', { defaultValue: 'No results found' })}</p>
          <p className="text-sm text-muted-foreground">
            {t('search.tryDifferent', { defaultValue: 'Try different keywords' })}
          </p>
        </div>
      ) : !hasSearched ? (
        <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
          <Search className="h-12 w-12 mb-4 opacity-50" />
          <p>{t('search.placeholder', { defaultValue: 'Enter keywords and click search' })}</p>
        </div>
      ) : (
        <div className="space-y-4">
          {Object.entries(groupedResults).map(([type, typeResults]) => (
            <div key={type}>
              <div className="flex items-center gap-2 mb-2">
                {getTypeIcon(type as SearchResult["type"])}
                <span className="font-medium text-sm">
                  {getTypeLabel(type as SearchResult["type"])}
                </span>
                <Badge variant="secondary" className="text-xs">
                  {typeResults.length}
                </Badge>
              </div>
              <div className="space-y-1 ml-6">
                {typeResults.map((result) => (
                  <button
                    key={result.id}
                    className="flex w-full items-center gap-3 rounded-md p-2 text-left text-sm hover:bg-accent transition-colors"
                    onClick={() => handleResultClick(result)}
                  >
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{result.title}</div>
                      {result.description && (
                        <div className="text-xs text-muted-foreground truncate">
                          {result.description}
                        </div>
                      )}
                    </div>
                    {result.relevance_score !== undefined && (
                      <Badge
                        variant="outline"
                        className={cn("text-xs", getTypeColor(result.type))}
                      >
                        {(result.relevance_score * 100).toFixed(0)}%
                      </Badge>
                    )}
                  </button>
                ))}
              </div>
            </div>
          ))}

          {hasSearched && results.length > 0 && (
            <div className="text-xs text-center text-muted-foreground pt-2 border-t">
              {t('search.foundCount', { count: results.length, defaultValue: `Found ${results.length} results` })}
            </div>
          )}
        </div>
      )}
    </div>
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} className="min-w-[80px]">
                {t('close')}
              </Button>
            </div>
          </div>
        </div>
      ) : null, getPortalRoot()
    )
  }

  // Desktop: Traditional dialog
  return createPortal(
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={handleClose}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-50',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[calc(100vh-2rem)]',
            'flex flex-col',
            'max-w-2xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <Search className="h-5 w-5 text-primary" />
                <h2 className="text-lg font-semibold leading-none truncate">
                  {t('search.title', { defaultValue: 'Global Search' })}
                </h2>
              </div>
              <p className="text-sm text-muted-foreground">
                {t('search.description', { defaultValue: 'Search devices, rules, workflows, and alerts' })}
              </p>
            </div>
            <button
              onClick={handleClose}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <div className="space-y-4">
      {/* Search Input */}
      <div className="flex gap-2">
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t('placeholders.search')}
          className="flex-1"
          autoFocus
        />
        <Button onClick={handleSearch} disabled={loading || !query.trim()}>
          {loading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
        </Button>
      </div>

      {/* Results */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : hasSearched && results.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Search className="h-12 w-12 text-muted-foreground mb-4" />
          <p className="text-lg font-medium">{t('search.noResults', { defaultValue: 'No results found' })}</p>
          <p className="text-sm text-muted-foreground">
            {t('search.tryDifferent', { defaultValue: 'Try different keywords' })}
          </p>
        </div>
      ) : !hasSearched ? (
        <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
          <Search className="h-12 w-12 mb-4 opacity-50" />
          <p>{t('search.placeholder', { defaultValue: 'Enter keywords and click search' })}</p>
        </div>
      ) : (
        <div className="space-y-4">
          {Object.entries(groupedResults).map(([type, typeResults]) => (
            <div key={type}>
              <div className="flex items-center gap-2 mb-2">
                {getTypeIcon(type as SearchResult["type"])}
                <span className="font-medium text-sm">
                  {getTypeLabel(type as SearchResult["type"])}
                </span>
                <Badge variant="secondary" className="text-xs">
                  {typeResults.length}
                </Badge>
              </div>
              <div className="space-y-1 ml-6">
                {typeResults.map((result) => (
                  <button
                    key={result.id}
                    className="flex w-full items-center gap-3 rounded-md p-2 text-left text-sm hover:bg-accent transition-colors"
                    onClick={() => handleResultClick(result)}
                  >
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{result.title}</div>
                      {result.description && (
                        <div className="text-xs text-muted-foreground truncate">
                          {result.description}
                        </div>
                      )}
                    </div>
                    {result.relevance_score !== undefined && (
                      <Badge
                        variant="outline"
                        className={cn("text-xs", getTypeColor(result.type))}
                      >
                        {(result.relevance_score * 100).toFixed(0)}%
                      </Badge>
                    )}
                  </button>
                ))}
              </div>
            </div>
          ))}

          {hasSearched && results.length > 0 && (
            <div className="text-xs text-center text-muted-foreground pt-2 border-t">
              {t('search.foundCount', { count: results.length, defaultValue: `Found ${results.length} results` })}
            </div>
          )}
        </div>
      )}
    </div>
          </div>
        </div>
      )}
    </>,
    getPortalRoot()
  )
}
