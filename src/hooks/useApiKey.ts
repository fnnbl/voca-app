import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export type KeyType = 'whisper_api_key' | 'ai_enhancement_api_key'

export function useApiKey(keyType: KeyType = 'whisper_api_key') {
  const [apiKey, setApiKey] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    invoke<string | null>('get_api_key', { keyType })
      .then(setApiKey)
      .catch(() => setApiKey(null))
      .finally(() => setLoading(false))
  }, [keyType])

  async function save(value: string): Promise<void> {
    await invoke('save_api_key', { keyType, value })
    setApiKey(value)
  }

  async function remove(): Promise<void> {
    await invoke('delete_api_key', { keyType })
    setApiKey(null)
  }

  return { apiKey, loading, save, remove }
}
