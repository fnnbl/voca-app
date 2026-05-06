import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { version as appVersion } from '../../package.json'
import { useAppStore } from '../stores/appStore'
import { GeneralSettings } from './settings/GeneralSettings'
import { TranscriptionSettings } from './settings/TranscriptionSettings'
import { AiSettings } from './settings/AiSettings'
import { SnippetsSettings } from './settings/SnippetsSettings'
import { DictionarySettings } from './settings/DictionarySettings'
import { FillersSettings } from './settings/FillersSettings'
import { HistoryPage } from './HistoryPage'
import { StatsPage } from './StatsPage'
import { AboutPage } from './AboutPage'
import { LegalPage } from './LegalPage'
import { DEFAULT_SHORTCUT } from '../types'
import type { Settings } from '../types'
import { formatShortcut } from '../shortcut/format'

type NavId = 'history' | 'stats' | 'transcription' | 'ai' | 'snippets' | 'dictionary' | 'fillers' | 'general' | 'about' | 'legal'
type LegalTab = 'privacy' | 'terms'

interface Props {
  settings: Settings
  onSave: (updated: Settings) => Promise<void>
}

export function SettingsPage({ settings, onSave }: Props) {
  const { t } = useTranslation()
  const [active, setActive] = useState<NavId>('history')
  const [legalTab, setLegalTab] = useState<LegalTab>('privacy')
  const error = useAppStore((s) => s.error)

  function openLegal(tab: LegalTab) {
    setLegalTab(tab)
    setActive('legal')
  }

  function handleChange(updated: Settings) {
    onSave(updated).catch(console.error)
  }

  function camelCode(code: string) {
    return code.toLowerCase().replace(/_([a-z])/g, (_: string, c: string) => c.toUpperCase())
  }

  const shortcutKey = settings.shortcuts?.key ?? DEFAULT_SHORTCUT

  return (
    <div className="shell">
      <aside className="shell-sidebar">
        <div className="shell-brand">
          <VocaLogoMark size={22} />
          <span className="mark">VOCA</span>
          <span className="version">v{appVersion.split('.').slice(0, 2).join('.')}</span>
        </div>

        <nav className="shell-nav">
          <NavItem id="history" active={active} onClick={setActive} icon={<HistoryIcon />}>
            History
          </NavItem>
          <NavItem id="stats" active={active} onClick={setActive} icon={<StatsIcon />}>
            Stats
          </NavItem>

          <div className="shell-nav-group">Einstellungen</div>

          <NavItem id="transcription" active={active} onClick={setActive} icon={<MicIcon />}>
            {t('settings.nav.transcription')}
          </NavItem>
          <NavItem id="ai" active={active} onClick={setActive} icon={<SparkleIcon />}>
            {t('settings.nav.ai')}
          </NavItem>
          <NavItem id="snippets" active={active} onClick={setActive} icon={<TextIcon />}>
            {t('settings.nav.snippets')}
          </NavItem>
          <NavItem id="dictionary" active={active} onClick={setActive} icon={<BookIcon />}>
            {t('settings.nav.dictionary')}
          </NavItem>
          <NavItem id="fillers" active={active} onClick={setActive} icon={<EraserIcon />}>
            {t('settings.nav.fillers')}
          </NavItem>
          <NavItem id="general" active={active} onClick={setActive} icon={<SettingsIcon />}>
            {t('settings.nav.general')}
          </NavItem>
          <NavItem id="about" active={active} onClick={setActive} icon={<InfoIcon />} variant="bottom">
            {t('settings.nav.about', 'About')}
          </NavItem>
        </nav>

        <div className="shell-foot">
          <span className="status-dot" />
          <span className="lbl">Bereit</span>
          <span className="shortcut-badge">{formatShortcut(shortcutKey)}</span>
        </div>
      </aside>

      <main className="shell-main">
        <div className="shell-main-pad">
          {error && (
            <div style={{ marginBottom: 20, padding: '12px 16px', borderRadius: 'var(--r-2)', background: 'var(--v-accent-soft)', border: '1px solid var(--v-danger)', color: 'var(--v-danger)', fontFamily: 'var(--f-mono)', fontSize: 12 }}>
              {t(`errors.${camelCode(error.code)}`, error.message)}
            </div>
          )}
          {active === 'history'       && <HistoryPage />}
          {active === 'stats'         && <StatsPage />}
          {active === 'transcription' && <TranscriptionSettings settings={settings} onChange={handleChange} />}
          {active === 'ai'            && <AiSettings settings={settings} onChange={handleChange} />}
          {active === 'snippets'      && <SnippetsSettings />}
          {active === 'dictionary'    && <DictionarySettings />}
          {active === 'fillers'       && <FillersSettings settings={settings} onChange={handleChange} />}
          {active === 'general'       && <GeneralSettings settings={settings} onChange={handleChange} />}
          {active === 'about'         && <AboutPage onOpenLegal={openLegal} />}
          {active === 'legal'         && <LegalPage initialTab={legalTab} />}
        </div>
      </main>
    </div>
  )
}

function NavItem({
  id, active, onClick, icon, kbd, children, variant,
}: {
  id: NavId
  active: NavId
  onClick: (id: NavId) => void
  icon: ReactNode
  kbd?: string
  children: ReactNode
  variant?: 'bottom'
}) {
  const variantClass = variant === 'bottom' ? ' is-bottom' : ''
  return (
    <button
      className={`shell-nav-item${active === id ? ' is-active' : ''}${variantClass}`}
      onClick={() => onClick(id)}
    >
      {icon}
      <span>{children}</span>
      {kbd && <span className="nav-kbd">{kbd}</span>}
    </button>
  )
}

/* ---- Brand Logo ---- */
function VocaLogoMark({ size = 22 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 100 100" fill="none" aria-hidden>
      <path d="M20 24 L50 76 L80 24" stroke="var(--v-ink)" strokeWidth="11" strokeLinecap="square" strokeLinejoin="miter"/>
      <circle cx="80" cy="24" r="8" fill="var(--v-ember)"/>
    </svg>
  )
}

/* ---- SVG Icons ---- */
function HistoryIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round"><path d="M2 8a6 6 0 1 0 1.76-4.24"/><path d="M2 2v3h3M8 5v3l2 1.5"/></svg>
}
function StatsIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"><path d="M2 13h12M4 10v3M8 7v6M12 4v9"/></svg>
}
function MicIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round"><rect x="6" y="2" width="4" height="8" rx="2"/><path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2M5.5 14h5"/></svg>
}
function SparkleIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round"><path d="M8 2l1.4 3.6L13 7l-3.6 1.4L8 12l-1.4-3.6L3 7l3.6-1.4L8 2z"/></svg>
}
function TextIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"><path d="M2 4h12M2 8h8M2 12h10"/></svg>
}
function BookIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round"><path d="M2.5 3v10l2-1h9V3zM4.5 3v9"/></svg>
}
function EraserIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round"><path d="M3.5 11.5l-1 1a1 1 0 0 0 0 1.4l0.6 0.6a1 1 0 0 0 1.4 0l7-7a1 1 0 0 0 0-1.4L9 3a1 1 0 0 0-1.4 0L3.5 7.1z"/><path d="M6 14h8"/></svg>
}
function SettingsIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"><circle cx="8" cy="8" r="1.5"/><path d="M8 2v1.5M8 12.5V14M13 8h-1.5M4.5 8H3M11.5 4.5l-1 1M5.5 10.5l-1 1M11.5 11.5l-1-1M5.5 5.5l-1-1"/></svg>
}
function InfoIcon() {
  return <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round"><circle cx="8" cy="8" r="6.25"/><path d="M8 7.25v3.5M8 5v0.25"/></svg>
}
