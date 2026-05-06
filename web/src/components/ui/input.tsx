import * as React from "react"
import { cn } from "@/lib/utils"

export interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, value: valueProp, onChange: onChangeProp, onCompositionStart: compStartProp, onCompositionEnd: compEndProp, ...props }, ref) => {
    // IME-safe: buffer value locally during composition so the controlled
    // value doesn't interrupt CJK / pinyin input.  This is especially
    // needed in Tauri (WebKit) where React's built-in composition
    // suppression doesn't always work.
    const composingRef = React.useRef(false)
    const [buffer, setBuffer] = React.useState(valueProp)

    // For password fields, IME composition produces intermediate
    // characters that bypass the browser's ● masking, causing
    // garbled display.  Disable the IME buffer for password inputs.
    const isPassword = type === 'password'

    // Sync from parent when value changes externally (and not composing)
    React.useEffect(() => {
      if (!composingRef.current) {
        setBuffer(valueProp)
      }
    }, [valueProp])

    return (
      <input
        type={type}
        className={cn(
          "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 aria-[invalid=true]:border-destructive aria-[invalid=true]:focus-visible:ring-destructive/20",
          className
        )}
        ref={ref}
        value={isPassword ? valueProp : buffer}
        onChange={(e) => {
          if (!isPassword) {
            setBuffer(e.target.value)
          }
          if (!composingRef.current) {
            onChangeProp?.(e)
          }
        }}
        onCompositionStart={(e) => {
          composingRef.current = true
          compStartProp?.(e)
        }}
        onCompositionEnd={(e) => {
          composingRef.current = false
          const v = (e.target as HTMLInputElement).value
          if (!isPassword) {
            setBuffer(v)
          }
          // Fire parent onChange with final value via a synthetic change event
          onChangeProp?.({ target: { value: v } } as React.ChangeEvent<HTMLInputElement>)
          compEndProp?.(e)
        }}
        {...props}
      />
    )
  }
)
Input.displayName = "Input"

export { Input }
