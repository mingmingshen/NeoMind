import { useState } from 'react'
import {
  Camera,
  Bot,
  User,
  AlertCircle,
  Loader2,
  Clock,
  Cpu,
  Database,
} from 'lucide-react'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import type { AnalystMessage } from './types'

interface AnalystMessageBubbleProps {
  message: AnalystMessage
  streamingContent?: string
}

export function AnalystMessageBubble({ message, streamingContent }: AnalystMessageBubbleProps) {
  const [fullscreenImage, setFullscreenImage] = useState<string | null>(null)
  // Streaming indicator is controlled by streamingMsgId (via AnalystTimeline),
  // NOT by message.isStreaming which is never cleared from the messages array.
  // streamingContent is undefined when this message is not the active streaming target.
  const isStreaming = streamingContent !== undefined
  const displayContent = streamingContent || message.content

  const formatTime = (ts: number) =>
    new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })

  const formatDuration = (ms: number) =>
    ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`

  const hasImages = message.images && message.images.length > 0

  // Image-only entry (legacy standalone image messages)
  if (message.type === 'image') {
    return (
      <>
        <div className="flex items-start gap-2">
          <div className="w-6 h-6 rounded-md bg-blue-500/20 flex items-center justify-center shrink-0 mt-0.5">
            <Camera className="h-4 w-4 text-blue-500" />
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

  // Data entry — unified bubble for images + text
  if (message.type === 'data') {
    const lines = message.content ? message.content.split('\n').filter(Boolean) : []
    return (
      <>
        <div className="flex items-start gap-2">
          <div className="w-6 h-6 rounded-md bg-amber-500/20 flex items-center justify-center shrink-0 mt-0.5">
            <Database className="h-4 w-4 text-amber-500" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex justify-between items-center">
              <span className="text-[10px] text-muted-foreground truncate">
                {message.dataSource || 'Data source'}
              </span>
              <span className="text-[10px] text-muted-foreground">{formatTime(message.timestamp)}</span>
            </div>
            <div className="mt-1 rounded-lg px-3 py-1.5 bg-amber-500/6 border border-amber-500/15 max-w-[280px] overflow-hidden">
              {/* Inline images */}
              {hasImages && (
                <div className="flex gap-1 mb-1 flex-wrap">
                  {message.images!.map((img, i) => (
                    <div
                      key={i}
                      className="rounded overflow-hidden border border-border/30 cursor-pointer max-w-[120px]"
                      onClick={() => setFullscreenImage(img)}
                    >
                      <img
                        src={img}
                        alt={`Image ${i + 1}`}
                        className="w-full h-auto max-h-[80px] object-cover"
                      />
                    </div>
                  ))}
                </div>
              )}
              {/* Text data lines */}
              {lines.map((line, i) => (
                <p key={i} className="text-xs text-foreground/80 font-mono leading-relaxed break-all">
                  {line.includes(':')
                    ? <>
                        <span className="text-foreground/50">{line.split(':').slice(0, -1).join(':')}:</span>
                        <span className="text-foreground/90">{line.split(':').slice(-1)[0]}</span>
                      </>
                    : line
                  }
                </p>
              ))}
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
          <Bot className="h-4 w-4 text-purple-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex justify-between items-center">
            <span className="text-[10px] text-purple-500 flex items-center gap-1">
              <Cpu className="h-2.5 w-2.5" />
              {message.modelName || 'AI'}
            </span>
            {isStreaming ? (
              <span className="text-[10px] text-purple-500 flex items-center gap-1">
                <Loader2 className="h-2.5 w-2.5 animate-spin" />
                Analyzing...
              </span>
            ) : (
              <span className="text-[10px] text-muted-foreground flex items-center gap-1">
                <Clock className="h-2.5 w-2.5" />
                {message.duration ? formatDuration(message.duration) : formatTime(message.timestamp)}
              </span>
            )}
          </div>
          <div className="mt-1 rounded-lg px-3 py-2 bg-purple-500/6 border border-purple-500/15 max-w-[320px]">
            {isStreaming ? (
              <div className="flex items-center gap-1.5">
                <Loader2 className="h-4 w-4 animate-spin text-purple-500" />
                <span className="text-xs text-muted-foreground">Waiting for response...</span>
              </div>
            ) : (
              <p className="text-xs text-foreground/90 leading-relaxed whitespace-pre-wrap">
                {displayContent}
              </p>
            )}
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
          <AlertCircle className="h-4 w-4 text-red-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="rounded-lg px-3 py-2 bg-red-500/10 border border-red-500/20 max-w-[320px]">
            <p className="text-xs text-red-500">{message.content}</p>
          </div>
        </div>
      </div>
    )
  }

  return null
}
