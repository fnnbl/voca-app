import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-shell'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { useAppStore } from '../stores/appStore'
import { version as appVersion } from '../../package.json'

const BMC_URL = 'https://buymeacoffee.com/fnnbl'

interface Props {
  onOpenLegal?: (tab: 'privacy' | 'terms') => void
}

type UpdaterState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'upToDate' }
  | { kind: 'updateAvailable'; version: string }
  | { kind: 'downloading'; received: number; total: number | null }
  | { kind: 'readyToInstall' }
  | { kind: 'error'; message: string }

export function AboutPage({ onOpenLegal }: Props) {
  const { t } = useTranslation()
  const updateInfo = useAppStore((s) => s.updateAvailable)
  const setUpdateAvailable = useAppStore((s) => s.setUpdateAvailable)
  const [state, setState] = useState<UpdaterState>({ kind: 'idle' })
  const [updateObj, setUpdateObj] = useState<Update | null>(null)

  async function performCheck() {
    setState({ kind: 'checking' })
    try {
      const update = await check()
      if (!update) {
        setState({ kind: 'upToDate' })
        setUpdateAvailable(null)
        setUpdateObj(null)
        return
      }
      setUpdateObj(update)
      setUpdateAvailable({ version: update.version, notes: update.body ?? null })
      setState({ kind: 'updateAvailable', version: update.version })
    } catch (e) {
      setState({ kind: 'error', message: String(e) })
    }
  }

  async function performInstall() {
    let upd = updateObj
    if (!upd) {
      try {
        const fetched = await check()
        if (!fetched) {
          setState({ kind: 'upToDate' })
          setUpdateAvailable(null)
          return
        }
        upd = fetched
        setUpdateObj(fetched)
      } catch (e) {
        setState({ kind: 'error', message: String(e) })
        return
      }
    }
    setState({ kind: 'downloading', received: 0, total: null })
    try {
      let total: number | null = null
      let received = 0
      await upd.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? null
          setState({ kind: 'downloading', received: 0, total })
        } else if (event.event === 'Progress') {
          received += event.data.chunkLength
          setState({ kind: 'downloading', received, total })
        } else if (event.event === 'Finished') {
          setState({ kind: 'readyToInstall' })
        }
      })
      setState({ kind: 'readyToInstall' })
    } catch (e) {
      setState({ kind: 'error', message: String(e) })
    }
  }

  function dismissAndIdle() {
    setState({ kind: 'idle' })
  }

  return (
    <div className="about-page">
      <div className="about-brand">
        <VocaWordmark />
      </div>

      {/* The two lines below are brand copy and intentionally untranslated —
          they're identity, not UI text. Same rationale as the VOCA wordmark
          itself staying "VOCA" across all locales. */}
      <h1 className="about-headline">Running on coffee and conviction.</h1>
      <p className="about-statement">
        Trying to prove you can still build great software without turning it
        into a monthly bill.
      </p>

      <div className="about-divider" />

      <p className="about-meta">
        VOCA v{appVersion} · open source · MIT License
      </p>

      <div className="about-updates">
        <UpdaterPanel
          state={state}
          preStagedUpdate={state.kind === 'idle' ? updateInfo : null}
          onCheck={performCheck}
          onInstall={performInstall}
          onLater={dismissAndIdle}
          onRestart={() => void relaunch()}
          onRetry={dismissAndIdle}
        />
      </div>

      <button
        type="button"
        className="about-bmc"
        onClick={() => open(BMC_URL).catch(() => {})}
      >
        <CoffeeIcon />
        <span>{t('settings.about.bmc', 'Buy me a coffee')}</span>
      </button>

      <div className="about-divider" />

      <p className="about-credits">
        {t('settings.about.credits', 'Built with Tauri, React, and whisper.cpp.')}
      </p>

      {onOpenLegal && (
        <nav className="about-legal-links" aria-label={t('settings.about.legalLinks', 'Legal')}>
          <button type="button" onClick={() => onOpenLegal('privacy')}>
            {t('settings.legal.privacy', 'Privacy')}
          </button>
          <span aria-hidden="true">·</span>
          <button type="button" onClick={() => onOpenLegal('terms')}>
            {t('settings.legal.terms', 'Terms')}
          </button>
        </nav>
      )}
    </div>
  )
}

interface PanelProps {
  state: UpdaterState
  preStagedUpdate: { version: string; notes: string | null } | null
  onCheck: () => void
  onInstall: () => void
  onLater: () => void
  onRestart: () => void
  onRetry: () => void
}

function UpdaterPanel({
  state,
  preStagedUpdate,
  onCheck,
  onInstall,
  onLater,
  onRestart,
  onRetry,
}: PanelProps) {
  const { t } = useTranslation()

  if (state.kind === 'checking') {
    return <div className="about-update-row muted">{t('settings.about.updates.checking')}</div>
  }
  if (state.kind === 'upToDate') {
    return (
      <div className="about-update-row">
        <span className="muted">{t('settings.about.updates.upToDate')}</span>
        <button type="button" className="about-update-link" onClick={onCheck}>
          {t('settings.about.updates.checkAgain')}
        </button>
      </div>
    )
  }
  if (state.kind === 'updateAvailable') {
    return (
      <div className="about-update-row">
        <span>{t('settings.about.updates.available', { version: state.version })}</span>
        <div className="about-update-actions">
          <button type="button" className="about-update-btn primary" onClick={onInstall}>
            {t('settings.about.updates.updateNow')}
          </button>
          <button type="button" className="about-update-btn ghost" onClick={onLater}>
            {t('settings.about.updates.later')}
          </button>
        </div>
      </div>
    )
  }
  if (state.kind === 'downloading') {
    const pct =
      state.total != null && state.total > 0
        ? Math.min(100, Math.round((state.received / state.total) * 100))
        : null
    return (
      <div className="about-update-row column">
        <span className="muted">
          {pct != null
            ? t('settings.about.updates.downloading', { pct })
            : t('settings.about.updates.downloadingNoSize')}
        </span>
        <div className="pbar-wrap" style={{ width: '100%' }}>
          <span className="pbar-fill" style={{ width: `${pct ?? 30}%` }} />
        </div>
      </div>
    )
  }
  if (state.kind === 'readyToInstall') {
    return (
      <div className="about-update-row">
        <span>{t('settings.about.updates.ready')}</span>
        <button type="button" className="about-update-btn primary" onClick={onRestart}>
          {t('settings.about.updates.restart')}
        </button>
      </div>
    )
  }
  if (state.kind === 'error') {
    return (
      <div className="about-update-row error">
        <span>{t('settings.about.updates.error')}</span>
        <button type="button" className="about-update-link" onClick={onRetry}>
          {t('settings.about.updates.retry')}
        </button>
      </div>
    )
  }

  // idle
  if (preStagedUpdate) {
    return (
      <div className="about-update-row">
        <span>{t('settings.about.updates.available', { version: preStagedUpdate.version })}</span>
        <div className="about-update-actions">
          <button type="button" className="about-update-btn primary" onClick={onInstall}>
            {t('settings.about.updates.updateNow')}
          </button>
          <button type="button" className="about-update-btn ghost" onClick={onLater}>
            {t('settings.about.updates.later')}
          </button>
        </div>
      </div>
    )
  }
  return (
    <div className="about-update-row">
      <button type="button" className="about-update-btn primary" onClick={onCheck}>
        {t('settings.about.updates.checkButton')}
      </button>
    </div>
  )
}

function VocaWordmark() {
  return (
    <div className="about-wordmark">
      <svg width={64} height={64} viewBox="0 0 100 100" fill="none" aria-hidden>
        <path d="M20 24 L50 76 L80 24" stroke="var(--v-ink)" strokeWidth="11" strokeLinecap="square" strokeLinejoin="miter"/>
        <circle cx="80" cy="24" r="8" fill="var(--v-ember)"/>
      </svg>
      <span>VOCA</span>
    </div>
  )
}

function CoffeeIcon() {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d="M2.5 6h8v4a3 3 0 0 1-3 3h-2a3 3 0 0 1-3-3V6z"/>
      <path d="M10.5 7h1a2 2 0 0 1 0 4h-1"/>
      <path d="M5 1.5s-1 1 0 2 0 2 0 2M7.5 1.5s-1 1 0 2 0 2 0 2"/>
    </svg>
  )
}
