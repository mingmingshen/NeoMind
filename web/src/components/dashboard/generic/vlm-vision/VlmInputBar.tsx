import { useState, useCallback } from 'react'
import { Send, Loader2 } from 'lucide-react'
import { Textarea } from '@/components/ui/textarea'
import { Button } from '@/components/ui/button'

interface VlmInputBarProps {
  onSend: (text: string) => void
  disabled?: boolean
}

export function VlmInputBar({ onSend, disabled }: VlmInputBarProps) {
  const [text, setText] = useState('')

  const handleSend = useCallback(() => {
    if (!text.trim() || disabled) return
    onSend(text.trim())
    setText('')
  }, [text, disabled, onSend])

  return (
    <div className="shrink-0 p-3 border-t border-border/50 bg-muted/20">
      <div className="flex gap-2">
        <Textarea
          placeholder="Send a follow-up message..."
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault()
              handleSend()
            }
          }}
          className="min-h-[36px] max-h-[72px] h-9 text-xs placeholder:text-xs resize-none py-1.5"
          disabled={disabled}
        />
        <Button
          size="sm"
          onClick={handleSend}
          disabled={!text.trim() || disabled}
          className="h-9 px-3 shrink-0"
        >
          {disabled ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Send className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>
    </div>
  )
}