/**
 * ChatInput Component
 *
 * Isolated input component to prevent re-renders of the entire chat page
 * when typing. Uses local state for the input field.
 */

import { memo, useCallback, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Send, X, Image as ImageIcon, Loader2, Zap, ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  DropdownMenuLabel,
} from '@/components/ui/dropdown-menu'

import { cn } from '@/lib/utils'
import type { ConnectionState } from '@/lib/websocket'
import type { ChatImage, LlmBackendInstance } from '@/types'

interface ChatInputProps {
  // Callbacks
  onSend: (message: string, images?: ChatImage[]) => void
  onCancel: () => void

  // State
  isStreaming: boolean
  disabled?: boolean

  // Model selector
  backends: LlmBackendInstance[]
  activeBackendId: string | null
  onBackendChange: (id: string) => void
  supportsMultimodal: boolean

  // Connection
  connectionState: ConnectionState
  onManualReconnect: () => void
  isDesktop: boolean
}

/**
 * Image preview component
 */
const ImagePreviews = memo(function ImagePreviews({
  images,
  onRemove
}: {
  images: ChatImage[]
  onRemove: (index: number) => void
}) {
  if (images.length === 0) return null

  return (
    <div className="flex flex-wrap gap-2 mb-2">
      {images.map((img, idx) => (
        <div key={idx} className="relative group">
          <img
            src={img.data}
            alt={`Image ${idx + 1}`}
            className="h-16 w-16 object-cover rounded-lg border border-border"
          />
          <button
            type="button"
            onClick={() => onRemove(idx)}
            className="absolute -top-1.5 -right-1.5 h-5 w-5 rounded-full bg-destructive text-error-foreground flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      ))}
    </div>
  )
})

/**
 * Model selector dropdown
 */
const ModelSelector = memo(function ModelSelector({
  backends,
  activeBackendId,
  onBackendChange,
  t
}: {
  backends: LlmBackendInstance[]
  activeBackendId: string | null
  onBackendChange: (id: string) => void
  t: (key: string) => string
}) {
  if (backends.length === 0) return null

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 sm:h-7 px-1.5 sm:px-2 rounded-lg text-muted-foreground hover:text-foreground text-xs gap-1 max-w-[120px] sm:max-w-[140px]"
        >
          <Zap className="h-4 w-4 shrink-0" />
          <span className="truncate">
            {backends.find(b => b.id === activeBackendId)?.name ||
             backends.find(b => b.id === activeBackendId)?.model ||
             t('chat:input.selectModel')}
          </span>
          <ChevronDown className="h-4 w-4 shrink-0" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-64 max-h-[50vh] overflow-y-auto">
        <DropdownMenuLabel className="text-xs text-muted-foreground">
          {t('chat:input.selectLLMModel')}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {backends.map((backend) => (
          <DropdownMenuItem
            key={backend.id}
            onClick={() => onBackendChange(backend.id)}
            className={cn(
              "flex items-center gap-2 py-2",
              backend.id === activeBackendId && "bg-muted"
            )}
          >
            <div className={cn(
              "w-1.5 h-1.5 rounded-full shrink-0",
              backend.healthy ? "bg-success" : "bg-muted-foreground"
            )} />
            <div className="flex-1 min-w-0">
              <div className="font-medium truncate text-sm">{backend.name}</div>
              <div className="text-xs text-muted-foreground truncate">
                {backend.model}
              </div>
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
})

/**
 * Main ChatInput component with memo to prevent unnecessary re-renders
 */
export const ChatInput = memo(function ChatInput({
  onSend,
  onCancel,
  isStreaming,
  disabled,
  backends,
  activeBackendId,
  onBackendChange,
  supportsMultimodal,
  connectionState,
  onManualReconnect,
  isDesktop,
}: ChatInputProps) {
  const { t } = useTranslation()
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Local state - this is the key to preventing re-renders
  const [input, setInput] = useState('')
  const [attachedImages, setAttachedImages] = useState<ChatImage[]>([])
  const [isUploadingImage, setIsUploadingImage] = useState(false)

  // Handle image selection
  const handleImageSelect = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files
    if (!files || files.length === 0) return

    setIsUploadingImage(true)
    try {
      const newImages: ChatImage[] = []
      for (const file of Array.from(files)) {
        const reader = new FileReader()
        const dataUrl = await new Promise<string>((resolve, reject) => {
          reader.onload = () => resolve(reader.result as string)
          reader.onerror = reject
          reader.readAsDataURL(file)
        })
        newImages.push({ data: dataUrl, mimeType: file.type })
      }
      setAttachedImages(prev => [...prev, ...newImages])
    } finally {
      setIsUploadingImage(false)
      e.target.value = ''
    }
  }, [])

  // Remove image
  const handleRemoveImage = useCallback((index: number) => {
    setAttachedImages(prev => prev.filter((_, i) => i !== index))
  }, [])

  // Handle send
  const handleSend = useCallback(() => {
    const trimmedInput = input.trim()
    if ((!trimmedInput && attachedImages.length === 0) || isStreaming) return

    onSend(trimmedInput, attachedImages.length > 0 ? attachedImages : undefined)
    setInput('')
    setAttachedImages([])

    // Reset textarea height
    if (inputRef.current) {
      inputRef.current.style.height = '40px'
    }
  }, [input, attachedImages, isStreaming, onSend])

  // Handle key down
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }, [handleSend])

  // Handle cancel
  const handleCancel = useCallback(() => {
    onCancel()
  }, [onCancel])

  const canSend = (input.trim() || attachedImages.length > 0) && !isStreaming

  return (
    <div className="bg-background sm:static fixed bottom-[var(--keyboard-offset,0px)] left-0 right-0 z-40 px-2.5 sm:px-4 pt-2 sm:pt-3 pb-8 sm:pb-4 safe-bottom"
      style={{ paddingBottom: 'max(2rem, env(safe-area-inset-bottom, 12px))' }}>
      <div className="max-w-3xl mx-auto">
        {/* Image previews */}
        <ImagePreviews images={attachedImages} onRemove={handleRemoveImage} />

        {/* Single unified input box — everything lives inside one container */}
        <div className="rounded-2xl border border-input bg-card shadow-sm transition-colors">
          {/* Textarea — fills the top of the box, borderless */}
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t('chat:input.placeholder')}
            rows={1}
            className={cn(
              "w-full block px-4 pt-3 pb-1 resize-none text-base bg-transparent",
              "placeholder:text-muted-foreground",
              "focus:outline-none",
              "max-h-32 scroll-mb-32"
            )}
            style={{ minHeight: "44px" }}
            onInput={(e) => {
              const target = e.target as HTMLTextAreaElement
              target.style.height = "auto"
              target.style.height = Math.min(target.scrollHeight, 128) + "px"
            }}
            disabled={disabled}
          />

          {/* Bottom toolbar — left: image + model, right: send. All inside the box. */}
          <div className="flex items-center gap-1 px-2 pb-2">
            {/* Image upload */}
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              multiple
              className="hidden"
              onChange={handleImageSelect}
              disabled={isStreaming || !supportsMultimodal}
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => fileInputRef.current?.click()}
              disabled={isStreaming || !supportsMultimodal}
              className={cn(
                "h-8 w-8 rounded-lg flex-shrink-0 text-muted-foreground hover:text-foreground",
                !supportsMultimodal && "opacity-50"
              )}
              title={supportsMultimodal ? t('chat:model.addImage') : t('chat:model.notSupportImage')}
            >
              {isUploadingImage ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : attachedImages.length > 0 ? (
                <div className="relative">
                  <ImageIcon className="h-4.5 w-4.5" />
                  <span className="absolute -top-1 -right-1 bg-primary text-primary-foreground text-[10px] rounded-full h-4 w-4 flex items-center justify-center font-semibold tabular-nums">
                    {attachedImages.length}
                  </span>
                </div>
              ) : (
                <ImageIcon className="h-4.5 w-4.5" />
              )}
            </Button>

            {/* Model selector */}
            <ModelSelector
              backends={backends}
              activeBackendId={activeBackendId}
              onBackendChange={onBackendChange}
              t={t}
            />

            <div className="flex-1" />

            {/* Send or Cancel button */}
            {isStreaming ? (
              <Button
                type="button"
                onClick={handleCancel}
                className={cn(
                  "h-8 w-8 rounded-lg flex-shrink-0 p-0",
                  "bg-destructive hover:bg-destructive-hover text-destructive-foreground"
                )}
                title="Cancel request"
              >
                <X className="h-4 w-4" />
              </Button>
            ) : (
              <Button
                type="button"
                onClick={handleSend}
                disabled={!canSend}
                className={cn(
                  "h-8 w-8 rounded-lg flex-shrink-0 p-0",
                  "bg-primary hover:bg-primary-hover text-primary-foreground",
                  !canSend && "opacity-40"
                )}
              >
                <Send className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  )
})
