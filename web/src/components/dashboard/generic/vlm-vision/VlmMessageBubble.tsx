import { useState } from 'react'
import {
  Camera,
  Bot,
  User,
  AlertCircle,
  Loader2,
  Clock,
  Cpu,
} from 'lucide-react'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import type { VlmMessage } from './types'

interface VlmMessageBubbleProps {
  message: VlmMessage
  streamingContent?: string
}

export function VlmMessageBubble({ message, streamingContent }: VlmMessageBubbleProps) {
  const [fullscreenImage, setFullscreenImage] = useState<string | null>(null)
  const isStreaming = message.isStreaming || !!streamingContent
  const displayContent = streamingContent || message.content

  const formatTime = (ts: number) =>
    new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })

  const formatDuration = (ms: number) =>
    ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`

  // Image data source entry
  if (message.type === 'image') {
    return (
      <>
        <div className="flex items-start gap-2">
          <div className="w-6 h-6 rounded-md bg-blue-500/20 flex items-center justify-center shrink-0 mt-0.5">
            <Camera className="h-3 w-3 text-blue-500" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex justify-between items-center">
              <span className="text-[10px] text-muted-foreground truncate">
                {message.dataSource || 'Data source'}
              </span>
              <span className="text-[10px] text-muted-foreground">{formatTime(message.timestamp)}</span>
            </div>
            <div
              className="mt-1 rounded-lg overflow-hidden border border-border/50 bg-muted/30 cursor-pointer max-w-[240px]"
              onClick={() => setFullscreenImage(message.content)}
            >
              <img
                src={message.content}
                alt="Data source image"
                className="w-full h-auto max-h-[120px] object-cover"
              />
            </div>
          </div>
        </div>
        {fullscreenImage && (
          <Dialog open={!!fullscreenImage} onOpenChange={() => setFullscreenImage(null)}>
            <DialogContent className="max-w-4xl p-2">
              <img
                src={fullscreenImage}
                alt="Fullscreen"
                className="w-full h-auto max-h-[80vh] object-contain rounded-lg"
              />
            </DialogContent>
          </Dialog>
        )}
      </>
    )
  }

  // AI response
  if (message.type === 'ai') {
    return (
      <div className="flex items-start gap-2">
        <div className="w-6 h-6 rounded-md bg-purple-500/20 flex items-center justify-center shrink-0 mt-0.5">
          <Bot className="h-3 w-3 text-purple-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex justify-between items-center">
            <span className="text-[10px] text-purple-500 flex items-center gap-1">
              <Cpu className="h-2.5 w-2.5" />
              {message.modelName || 'VLM'}
            </span>
            {isStreaming ? (
              <span className="text-[10px] text-purple-500 flex items-center gap-1">
                <Loader2 className="h-2.5 w-2.5 animate-spin" />
                streaming...
              </span>
            ) : (
              <span className="text-[10px] text-muted-foreground flex items-center gap-1">
                <Clock className="h-2.5 w-2.5" />
                {message.duration ? formatDuration(message.duration) : formatTime(message.timestamp)}
              </span>
            )}
          </div>
          <div className="mt-1 rounded-lg px-3 py-2 bg-purple-500/6 border border-purple-500/15">
            <p className="text-xs text-foreground/90 leading-relaxed whitespace-pre-wrap">
              {displayContent}
              {isStreaming && <span className="text-purple-500">|</span>}
            </p>
          </div>
        </div>
      </div>
    )
  }

  // User follow-up
  if (message.type === 'user') {
    return (
      <div className="flex items-start gap-2 justify-end">
        <div className="flex-1 min-w-0 flex flex-col items-end">
          <div className="flex justify-between items-center w-full">
            <span className="text-[10px] text-muted-foreground">{formatTime(message.timestamp)}</span>
            <span className="text-[10px] text-purple-500/70 flex items-center gap-1">
              <User className="h-2.5 w-2.5" />
              User
            </span>
          </div>
          <div className="mt-1 rounded-lg px-3 py-2 bg-purple-500/10 border border-purple-500/20 max-w-[260px]">
            <p className="text-xs text-foreground/90 leading-relaxed">{message.content}</p>
          </div>
        </div>
      </div>
    )
  }

  // Error
  if (message.type === 'error') {
    return (
      <div className="flex items-start gap-2">
        <div className="w-6 h-6 rounded-md bg-red-500/20 flex items-center justify-center shrink-0 mt-0.5">
          <AlertCircle className="h-3 w-3 text-red-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="rounded-lg px-3 py-2 bg-red-500/10 border border-red-500/20">
            <p className="text-xs text-red-500">{message.content}</p>
          </div>
        </div>
      </div>
    )
  }

  return null
}