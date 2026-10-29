import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'

export type ModelListKind = 'ai' | 'transcription'

/**
 * Loads the models a provider currently offers for the stored key.
 * `models` stays null until a fetch succeeds — callers fall back to their
 * static list in that case (no key yet, offline, provider without listing).
 */
export function useProviderModels(
  kind: ModelListKind,
  provider: string,
  customEndpoint: string,
  enabled: boolean,
) {
  const [models, setModels] = useState<string[] | null>(null)
  const [loading, setLoading] = useState(false)
  const [failed, setFailed] = useState(false)
  const [reloadToken, setReloadToken] = useState(0)

  useEffect(() => {
    setModels(null)
    setFailed(false)
    if (!enabled) return

    let cancelled = false
    setLoading(true)
    const command = kind === 'ai' ? 'list_ai_models' : 'list_transcription_models'
    // Debounced so typing a custom endpoint doesn't fire a request per key.
    const timer = setTimeout(() => {
      invoke<string[]>(command, { provider, customEndpoint: customEndpoint || null })
        .then((list) => {
          if (cancelled) return
          if (list.length > 0) setModels(list)
          else setFailed(true)
        })
        .catch(() => {
          if (!cancelled) setFailed(true)
        })
        .finally(() => {
          if (!cancelled) setLoading(false)
        })
    }, 400)
    return () => {
      cancelled = true
      clearTimeout(timer)
      setLoading(false)
    }
  }, [kind, provider, customEndpoint, enabled, reloadToken])

  const reload = useCallback(() => setReloadToken((n) => n + 1), [])

  return { models, loading, failed, reload }
}
