import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import ReactMarkdown from 'react-markdown'
import { open } from '@tauri-apps/plugin-shell'
import privacyDe from '../legal/privacy.de.md?raw'
import privacyEn from '../legal/privacy.en.md?raw'
import termsDe from '../legal/terms.de.md?raw'
import termsEn from '../legal/terms.en.md?raw'

type LegalTab = 'privacy' | 'terms'

interface Props {
  initialTab?: LegalTab
}

export function LegalPage({ initialTab = 'privacy' }: Props) {
  const { i18n, t } = useTranslation()
  const [tab, setTab] = useState<LegalTab>(initialTab)

  const isGerman = i18n.language.toLowerCase().startsWith('de')

  const content =
    tab === 'privacy'
      ? isGerman
        ? privacyDe
        : privacyEn
      : isGerman
        ? termsDe
        : termsEn

  return (
    <div className="legal-page">
      <div className="legal-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'privacy'}
          className={`legal-tab ${tab === 'privacy' ? 'active' : ''}`}
          onClick={() => setTab('privacy')}
        >
          {t('settings.legal.privacy', 'Privacy')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'terms'}
          className={`legal-tab ${tab === 'terms' ? 'active' : ''}`}
          onClick={() => setTab('terms')}
        >
          {t('settings.legal.terms', 'Terms')}
        </button>
      </div>

      <article className="legal-content">
        <ReactMarkdown
          components={{
            a: ({ href, children }) => (
              <a
                href={href}
                onClick={(e) => {
                  e.preventDefault()
                  if (href) open(href).catch(() => {})
                }}
              >
                {children}
              </a>
            ),
          }}
        >
          {content}
        </ReactMarkdown>
      </article>
    </div>
  )
}
