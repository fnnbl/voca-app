import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '../stores/appStore'
import type { AppState } from '../types'

interface RecordingStateChangedPayload {
  state: AppState
}

export function useAppStateListener() {
  const setAppState = useAppStore((s) => s.setAppState)

  useEffect(() => {
    const unlisten = listen<RecordingStateChangedPayload>(
      'recording-state-changed',
      (event) => {
        setAppState(event.payload.state)
      }
    )

    return () => {
      unlisten.then((f) => f())
    }
  }, [setAppState])
}
