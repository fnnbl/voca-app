import { useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/appStore'
import type { Settings } from '../types'

export function useSettings() {
  const { settings, setSettings } = useAppStore()

  useEffect(() => {
    invoke<Settings>('get_settings').then(setSettings).catch(console.error)
  }, [setSettings])

  async function save(updated: Settings): Promise<void> {
    await invoke('save_settings', { settings: updated })
    setSettings(updated)
  }

  return { settings, save }
}
