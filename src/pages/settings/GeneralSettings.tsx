import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { SettingRow } from '../../components/settings/SettingRow'
import { ShortcutRecorder } from '../../components/settings/ShortcutRecorder'
import { DEFAULT_SHORTCUT } from '../../types'
import type { Settings } from '../../types'

interface Props {
  settings: Settings
  onChange: (updated: Settings) => void
}

export function GeneralSettings({ settings, onChange }: Props) {
  const { t } = useTranslation()
  const currentShortcut = settings.shortcuts.key || DEFAULT_SHORTCUT
  const [autostart, setAutostart] = useState(false)
  const [audioDevices, setAudioDevices] = useState<string[]>([])

  useEffect(() => {
    invoke<boolean>('get_autostart').then(setAutostart).catch(() => {})
    invoke<string[]>('list_audio_devices').then(setAudioDevices).catch(() => {})
  }, [])

  async function handleAutostartToggle() {
    const next = !autostart
    try {
      await invoke('set_autostart', { enabled: next })
      setAutostart(next)
      onChange({ ...settings, general: { ...settings.general, autostart: next } })
    } catch (e) {
      console.error('Failed to set autostart:', e)
    }
  }

  async function handleShortcutChange(newShortcut: string) {
    try {
      await invoke('register_shortcut', { oldShortcut: currentShortcut, newShortcut })
      onChange({ ...settings, shortcuts: { key: newShortcut } })
    } catch (e) {
      console.error('Failed to register shortcut:', e)
    }
  }

  async function handleAudioDeviceChange(name: string) {
    const value = name === '' ? null : name
    await invoke('set_audio_device', { name: value }).catch(() => {})
    onChange({ ...settings, general: { ...settings.general, audioInputDevice: value } })
  }

  function setLanguage(lang: 'de' | 'en') {
    onChange({ ...settings, general: { ...settings.general, language: lang } })
  }

  function setTheme(theme: 'light' | 'dark' | 'system') {
    onChange({ ...settings, general: { ...settings.general, theme } })
  }

  function restartOnboarding() {
    onChange({ ...settings, general: { ...settings.general, onboardingCompleted: false } })
  }

  return (
    <div>
      <p className="page-eyebrow">einstellungen</p>
      <h1 className="page-title" style={{ marginBottom: 28 }}>{t('settings.nav.general')}</h1>

      <SettingRow label={t('settings.general.language')}>
        <div className="v-seg">
          {(['de', 'en'] as const).map((lang) => (
            <button
              key={lang}
              onClick={() => setLanguage(lang)}
              className={settings.general.language === lang ? 'is-active' : ''}
            >
              {lang === 'de' ? 'Deutsch' : 'English'}
            </button>
          ))}
        </div>
      </SettingRow>

      <SettingRow label={t('settings.general.theme')} description="">
        <div className="v-seg">
          {(['dark', 'light', 'system'] as const).map((th) => (
            <button
              key={th}
              onClick={() => setTheme(th)}
              className={(settings.general.theme ?? 'system') === th ? 'is-active' : ''}
            >
              {th === 'dark' ? t('settings.general.themeDark') : th === 'light' ? t('settings.general.themeLight') : t('settings.general.themeSystem')}
            </button>
          ))}
        </div>
      </SettingRow>

      <SettingRow label="Shortcut" description="">
        <div className="flex flex-col items-end gap-1">
          <ShortcutRecorder value={currentShortcut} onChange={handleShortcutChange} />
          <span className="text-[10px] text-text-muted">{t('settings.shortcut.description')}</span>
        </div>
      </SettingRow>

      <SettingRow label={t('settings.general.inputDevice')} description="">
        <select
          value={settings.general.audioInputDevice ?? ''}
          onChange={(e) => handleAudioDeviceChange(e.target.value)}
          className="w-56 px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text focus:outline-none focus:border-accent"
        >
          <option value="">{t('settings.general.inputDeviceDefault')}</option>
          {audioDevices.map((d) => (
            <option key={d} value={d}>{d}</option>
          ))}
        </select>
      </SettingRow>

      <SettingRow label={t('settings.general.autostart')} description="">
        <button
          role="switch"
          aria-checked={autostart}
          onClick={handleAutostartToggle}
          className={`v-switch${autostart ? ' on' : ''}`}
        />
      </SettingRow>

      <SettingRow label={t('settings.general.onboarding')} description="">
        <button
          onClick={restartOnboarding}
          className="px-3 py-1.5 text-xs border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
        >
          {t('settings.general.onboarding')}
        </button>
      </SettingRow>
    </div>
  )
}
