/**
 * useUpdateCheck Hook
 *
 * Hook for checking and managing application updates.
 * Automatically checks for updates on mount and provides
 * manual check functionality.
 */

import { useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '@/store'
import type { UpdateInfo, UpdateProgress } from '@/store/slices/updateSlice'

const UPDATE_CHECK_INTERVAL = 24 * 60 * 60 * 1000 // 24 hours

export interface UpdateCheckOptions {
  /** Auto-check on mount (default: true) */
  autoCheck?: boolean
  /** Interval for auto-checking in ms (default: 24 hours) */
  checkInterval?: number
  /** Callback when update is available */
  onUpdateAvailable?: (info: UpdateInfo) => void
  /** Callback on error */
  onError?: (error: string) => void
}

export interface UseUpdateCheckReturn {
  /** Check for updates manually */
  checkUpdate: () => Promise<void>
  /** Download and install available update */
  downloadAndInstall: () => Promise<void>
  /** Get current app version */
  getAppVersion: () => Promise<string>
  /** Relaunch the application */
  relaunchApp: () => Promise<void>
}

/**
 * Hook for managing application updates
 */
export function useUpdateCheck(options: UpdateCheckOptions = {}): UseUpdateCheckReturn {
  const {
    autoCheck = true,
    checkInterval = UPDATE_CHECK_INTERVAL,
    onUpdateAvailable,
    onError,
  } = options

  const { t } = useTranslation(['common', 'settings'])
  const { updateStatus, updateInfo, downloadProgress, setUpdateStatus, setUpdateInfo, setDownloadProgress, setError } = useAppStore()

  const intervalRef = useRef<ReturnType<typeof setInterval>>()
  const unlistenRef = useRef<(() => void) | null>(null)

  // Use refs to store the latest callbacks without triggering re-renders
  const onUpdateAvailableRef = useRef(onUpdateAvailable)
  const onErrorRef = useRef(onError)

  // Keep refs in sync with latest props
  useEffect(() => {
    onUpdateAvailableRef.current = onUpdateAvailable
    onErrorRef.current = onError
  }, [onUpdateAvailable, onError])

  /**
   * Check for available updates
   */
  const checkUpdate = useCallback(async () => {
    try {
      setUpdateStatus('checking')
      setError(null)

      const info = await invoke<UpdateInfo>('check_update')

      if (info.available) {
        setUpdateInfo(info)
        setUpdateStatus('available')
        // Use ref to get latest callback without including it in dependencies
        onUpdateAvailableRef.current?.(info)
      } else {
        setUpdateStatus('up-to-date')
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      setError(errorMessage)
      setUpdateStatus('error')
      onErrorRef.current?.(errorMessage)
    }
  }, [setUpdateStatus, setUpdateInfo, setError])

  /**
   * Download and install the available update
   */
  const downloadAndInstall = useCallback(async () => {
    try {
      setUpdateStatus('downloading')
      setError(null)

      await invoke('download_and_install')

      setUpdateStatus('installing')
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      setError(errorMessage)
      setUpdateStatus('error')
      onErrorRef.current?.(errorMessage)
      throw error
    }
  }, [setUpdateStatus, setError])

  /**
   * Get the current app version
   */
  const getAppVersion = useCallback(async (): Promise<string> => {
    try {
      return await invoke<string>('get_app_version')
    } catch (error) {
      console.error('Failed to get app version:', error)
      return 'unknown'
    }
  }, [])

  /**
   * Relaunch the application
   */
  const relaunchApp = useCallback(async () => {
    try {
      await invoke('relaunch_app')
    } catch (error) {
      console.error('Failed to relaunch app:', error)
      throw error
    }
  }, [])

  // Set up update progress listener
  useEffect(() => {
    const setupListener = async () => {
      try {
        const unlisten = await listen<UpdateProgress>('update-progress', (event) => {
          setDownloadProgress(event.payload)
        })
        unlistenRef.current = unlisten
      } catch (error) {
        console.error('Failed to set up update progress listener:', error)
      }
    }

    setupListener()

    return () => {
      unlistenRef.current?.()
    }
  }, [setDownloadProgress])

  // Auto-check on mount
  useEffect(() => {
    if (autoCheck) {
      // Check on mount
      checkUpdate()

      // Set up interval for periodic checks
      intervalRef.current = setInterval(() => {
        checkUpdate()
      }, checkInterval)

      return () => {
        clearInterval(intervalRef.current)
      }
    }
  }, [autoCheck, checkInterval, checkUpdate])

  return {
    checkUpdate,
    downloadAndInstall,
    getAppVersion,
    relaunchApp,
  }
}

export default useUpdateCheck
