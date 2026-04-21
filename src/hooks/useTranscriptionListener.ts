import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '../stores/appStore'

interface TranscriptionResultPayload {
  text: string
}

export function useTranscriptionListener() {
  const setLastTranscription = useAppStore((s) => s.setLastTranscription)

  useEffect(() => {
    const unlisten = listen<TranscriptionResultPayload>(
      'transcription-result',
      (event) => {
        setLastTranscription(event.payload.text)
      }
    )

    return () => {
      unlisten.then((f) => f())
    }
  }, [setLastTranscription])
}
