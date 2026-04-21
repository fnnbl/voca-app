import { useEffect, useRef } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '../stores/appStore'
import type { AppError } from '../types'

const ERROR_RESET_MS = 5000

export function useErrorListener() {
  const { setError, setAppState } = useAppStore()
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    const unlisten = listen<AppError>('error-occurred', (event) => {
      setError(event.payload)
      setAppState('error')

      if (timerRef.current) clearTimeout(timerRef.current)
      timerRef.current = setTimeout(() => {
        setError(null)
        setAppState('idle')
      }, ERROR_RESET_MS)
    })

    return () => {
      unlisten.then((f) => f())
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [setError, setAppState])
}
