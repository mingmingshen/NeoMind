/**
 * Code Editor Component
 *
 * Wraps @uiw/react-codemirror with syntax highlighting for JavaScript/TypeScript.
 * Supports light/dark themes and provides a clean editing experience.
 */

import React, { useMemo } from 'react'
import CodeMirror, { ReactCodeMirrorRef } from '@uiw/react-codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'
import { EditorView } from '@codemirror/view'
import { cn } from '@/lib/utils'

export interface CodeEditorProps {
  /** Current code value */
  value: string
  /** Callback when code changes */
  onChange: (value: string) => void
  /** Editor height */
  height?: string
  /** Minimum height */
  minHeight?: string
  /** Maximum height */
  maxHeight?: string
  /** Placeholder text */
  placeholder?: string
  /** Whether to show line numbers */
  lineNumbers?: boolean
  /** Read-only mode */
  readOnly?: boolean
  /** CSS class name */
  className?: string
  /** Whether to use compact mode */
  compact?: boolean
}

export const CodeEditor = React.forwardRef<ReactCodeMirrorRef, CodeEditorProps>(
  ({
    value,
    onChange,
    height = 'auto',
    minHeight = '120px',
    maxHeight,
    placeholder,
    lineNumbers = true,
    readOnly = false,
    className,
    compact = false,
  }, ref) => {
    // Check for dark mode using data-theme attribute or class
    const isDark = useMemo(() => {
      if (typeof document === 'undefined') return false
      const root = document.documentElement
      return root.getAttribute('data-theme') === 'dark' ||
             document.documentElement.classList.contains('dark')
    }, [])

    return (
      <div className={cn(
        'relative rounded-lg border overflow-hidden transition-all',
        'focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2',
        // Light theme
        !isDark && 'bg-background border-input',
        // Dark theme
        isDark && 'bg-[#1e1e1e] border-[#3c3c3c]',
        readOnly && 'opacity-60 cursor-not-allowed',
        className
      )}>
        <CodeMirror
          ref={ref}
          value={value}
          height={height}
          minHeight={minHeight}
          maxHeight={maxHeight}
          extensions={[
            javascript({ jsx: true }),
            EditorView.lineWrapping,
            EditorView.theme({
              // Base editor styles
              '&': {
                fontSize: compact ? '13px' : '14px',
                color: isDark ? '#d4d4d4' : '#24292e',
              },
              // Scroller (main content area)
              '.cm-scroller': {
                fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
                lineHeight: '1.6',
              },
              // Content padding
              '.cm-content': {
                padding: compact ? '10px 14px' : '14px',
              },
              // Line spacing
              '.cm-line': {
                padding: '0 0',
              },
              // Gutter (line numbers)
              '.cm-gutters': {
                backgroundColor: 'transparent',
                color: isDark ? '#6e7681' : '#8b949e',
                border: 'none',
                paddingRight: '8px',
                marginRight: '4px',
                fontSize: compact ? '12px' : '13px',
              },
              // Current line highlight
              '.cm-activeLine': {
                backgroundColor: isDark ? 'rgba(88, 166, 255, 0.1)' : 'rgba(88, 166, 255, 0.08)',
              },
              '.cm-activeLineGutter': {
                backgroundColor: 'transparent',
                color: isDark ? '#d4d4d4' : '#24292e',
                fontWeight: '500',
              },
              // Selection
              '.cm-selectionBackground': {
                backgroundColor: isDark ? 'rgba(88, 166, 255, 0.25)' : 'rgba(88, 166, 255, 0.2)',
              },
              // Placeholder
              '.cm-placeholder': {
                color: isDark ? '#6e7681' : '#8b949e',
                fontStyle: 'italic',
              },
              // Cursor
              '.cm-cursorLayer': {
                color: isDark ? '#58a6ff' : '#24292e',
              },
              // Matching bracket
              '.cm-matchingBracket': {
                color: isDark ? '#58a6ff' : '#24292e',
                borderBottom: isDark ? '2px solid #58a6ff' : '2px solid #24292e',
              },
              // Keyword
              '.tok-keyword': {
                color: isDark ? '#ff7b72' : '#cf222e',
                fontWeight: '500',
              },
              // String
              '.tok-string': {
                color: isDark ? '#a5d6ff' : '#0a3069',
              },
              // Number
              '.tok-number': {
                color: isDark ? '#79c0ff' : '#0550ae',
              },
              // Comment
              '.tok-comment': {
                color: isDark ? '#8b949e' : '#6e7781',
                fontStyle: 'italic',
              },
              // Function/Variable
              '.tok-variableName': {
                color: isDark ? '#d4d4d4' : '#24292e',
              },
              '.tok-def': {
                color: isDark ? '#d2a8ff' : '#953800',
              },
              '.tok-propertyName': {
                color: isDark ? '#79c0ff' : '#0550ae',
              },
              // Operator
              '.tok-operator': {
                color: isDark ? '#ff7b72' : '#cf222e',
              },
              // Punctuation
              '.tok-punctuation': {
                color: isDark ? '#d4d4d4' : '#24292e',
              },
              // Boolean
              '.tok-bool': {
                color: isDark ? '#79c0ff' : '#0550ae',
              },
              // Null/Undefined
              '.tok-null': {
                color: isDark ? '#ff7b72' : '#cf222e',
              },
              // Scrollbar styling
              '&::-webkit-scrollbar': {
                width: '8px',
                height: '8px',
              },
              '&::-webkit-scrollbar-track': {
                background: isDark ? 'transparent' : 'transparent',
              },
              '&::-webkit-scrollbar-thumb': {
                background: isDark ? '#424242' : '#d1d5db',
                borderRadius: '4px',
              },
              '&::-webkit-scrollbar-thumb:hover': {
                background: isDark ? '#4f4f4f' : '#9ca3af',
              },
            }),
            ...(isDark ? [oneDark] : []),
          ]}
          onChange={onChange}
          placeholder={placeholder}
          readOnly={readOnly}
          basicSetup={{
            lineNumbers,
            highlightActiveLineGutter: true,
            highlightSpecialChars: false,
            foldGutter: false,
            drawSelection: true,
            dropCursor: false,
            allowMultipleSelections: true,
            indentOnInput: true,
            bracketMatching: true,
            closeBrackets: true,
            autocompletion: false,
            rectangularSelection: false,
            crosshairCursor: false,
            highlightActiveLine: true,
            highlightSelectionMatches: false,
            closeBracketsKeymap: true,
            searchKeymap: false,
            foldKeymap: false,
            completionKeymap: false,
            lintKeymap: false,
          }}
        />
      </div>
    )
  }
)

CodeEditor.displayName = 'CodeEditor'

// Legacy interface for backward compatibility
export interface LegacyCodeEditorProps {
  value: string
  onChange: (value: string) => void
  language?: "json" | "yaml"
  placeholder?: string
  className?: string
  readOnly?: boolean
  error?: string
}

export function LegacyCodeEditor({
  value,
  onChange,
  language = "json",
  placeholder = "",
  className,
  readOnly = false,
  error,
}: LegacyCodeEditorProps) {
  // For JSON/YAML, use the original textarea-based editor
  return (
    <div className={cn("relative", className)}>
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        readOnly={readOnly}
        placeholder={placeholder}
        className={cn(
          "w-full min-h-[200px] p-4 font-mono text-sm",
          "bg-muted border border-input rounded-md",
          "focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
          "resize-y overflow-auto",
          readOnly && "cursor-not-allowed opacity-50",
          error && "border-destructive"
        )}
        spellCheck={false}
      />
      {error && (
        <p className="text-xs text-destructive mt-1">{error}</p>
      )}
    </div>
  )
}
