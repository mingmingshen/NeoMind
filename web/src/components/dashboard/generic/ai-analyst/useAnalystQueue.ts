import { useRef, useCallback, useState } from 'react'

const PROCESSING_TIMEOUT_MS = 120_000

export function useAnalystQueue(
  onProcess: (image: string) => void,
) {
  const [pending, setPending] = useState(0)
  const [isProcessing, setIsProcessing] = useState(false)
  const pendingImageRef = useRef<string | null>(null)
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const processNext = useCallback(() => {
    if (pendingImageRef.current) {
      const image = pendingImageRef.current
      pendingImageRef.current = null
      setPending(0)
      setIsProcessing(true)

      timeoutRef.current = setTimeout(() => {
        setIsProcessing(false)
        processNext()
      }, PROCESSING_TIMEOUT_MS)

      onProcess(image)
    } else {
      setIsProcessing(false)
    }
  }, [onProcess])

  const enqueue = useCallback((image: string) => {
    if (isProcessing) {
      pendingImageRef.current = image
      setPending(1)
    } else {
      setIsProcessing(true)
      setPending(0)

      timeoutRef.current = setTimeout(() => {
        setIsProcessing(false)
        processNext()
      }, PROCESSING_TIMEOUT_MS)

      onProcess(image)
    }
  }, [isProcessing, onProcess, processNext])

  const completeProcessing = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    setIsProcessing(false)
    processNext()
  }, [processNext])

  return { enqueue, completeProcessing, pending, isProcessing }
}