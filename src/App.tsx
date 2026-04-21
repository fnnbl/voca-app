import { useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useTranslation } from 'react-i18next'
import { useAppStore } from './stores/appStore'
import { useAppStateListener } from './hooks/useAppState'
import { useErrorListener } from './hooks/useErrorListener'
import { useTranscriptionListener } from './hooks/useTranscriptionListener'
import { useSettings } from './hooks/useSettings'
import { useShortcutFallback } from './hooks/useShortcutFallback'
import { DEFAULT_SHORTCUT } from './types'
import { SettingsPage } from './pages/SettingsPage'
import { OnboardingPage } from './pages/OnboardingPage'
import type { Settings } from './types'

export default function App() {
  const { i18n } = useTranslation()
  const { settings } = useAppStore()
  const { save } = useSettings()

  useAppStateListener()
  useErrorListener()
  useTranscriptionListener()
  useShortcutFallback(settings?.shortcuts?.key ?? DEFAULT_SHORTCUT)

  useEffect(() => {
    const lang = settings?.general?.language ?? 'de'
    i18n.changeLanguage(lang)
  }, [settings, i18n])

  useEffect(() => {
    const theme = settings?.general?.theme ?? 'system'

    function applyCss(isDark: boolean) {
      if (isDark) {
        document.documentElement.setAttribute('data-theme', 'dark')
      } else {
        document.documentElement.removeAttribute('data-theme')
      }
    }

    function applyTheme(isDark: boolean) {
      applyCss(isDark)
      invoke('set_window_theme', { theme: isDark ? 'dark' : 'light' }).catch(() => {})
    }

    if (theme === 'system') {
      const win = getCurrentWindow()
      // First reset title bar to OS control (None), THEN read — otherwise
      // win.theme() would still return whatever we last set explicitly.
      invoke('set_window_theme', { theme: 'system' })
        .catch(() => {})
        .finally(() => {
          win.theme()
            .then((t) => applyCss(t === 'dark'))
            .catch(() => applyCss(window.matchMedia('(prefers-color-scheme: dark)').matches))
        })
      let cleanup: (() => void) | null = null
      win.onThemeChanged(({ payload: t }) => applyCss(t === 'dark'))
        .then((fn) => { cleanup = fn })
      return () => { cleanup?.() }
    }

    applyTheme(theme === 'dark')
  }, [settings])

  if (!settings) {
    return (
      <div className="h-screen bg-surface flex items-center justify-center">
        <p className="text-sm text-text-muted">Loading…</p>
      </div>
    )
  }

  if (!settings.general.onboardingCompleted) {
    return <OnboardingPage settings={settings} onComplete={save} />
  }

  return (
    <SettingsPage
      settings={settings}
      onSave={(updated: Settings) => save(updated)}
    />
  )
}
