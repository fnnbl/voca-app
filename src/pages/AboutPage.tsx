import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-shell'
import { version as appVersion } from '../../package.json'

const BMC_URL = 'https://buymeacoffee.com/fnnbl'

interface Props {
  onOpenLegal?: (tab: 'privacy' | 'terms') => void
}

export function AboutPage({ onOpenLegal }: Props) {
  const { t } = useTranslation()

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
