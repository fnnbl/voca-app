import { useEffect, useRef } from 'react'
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
import { detectOsLocale } from './i18n/detectOsLocale'
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

  // First-run UI language auto-detection. Fires at most once per app launch
  // and only when the stored language is still the unchanged 'de' default
  // and onboarding hasn't completed yet. A user who deliberately sets their
  // UI to DE after having finished onboarding (or to any non-DE language)
  // is never overridden.
  const autoDetectedRef = useRef(false)
  useEffect(() => {
    if (!settings || autoDetectedRef.current) return
    autoDetectedRef.current = true
    if (settings.general.onboardingCompleted) return
    if (settings.general.language !== 'de') return
    const detected = detectOsLocale()
    if (detected === 'de') return
    save({
      ...settings,
      general: { ...settings.general, language: detected },
      aiEnhancement: { ...settings.aiEnhancement, activePromptId: `default-${detected}` },
    })
  }, [settings, save])

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
