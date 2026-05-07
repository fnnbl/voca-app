import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface ShowPayload {
  version: string
  notes: string | null
}

const AUTO_DISMISS_MS = 10_000

export default function UpdateToast() {
  const { t } = useTranslation()
  const [version, setVersion] = useState<string | null>(null)

  useEffect(() => {
    const unlisten = listen<ShowPayload>('update-toast://show', (event) => {
      setVersion(event.payload.version)
    })
    return () => {
      void unlisten.then((fn) => fn())
    }
  }, [])

  useEffect(() => {
    if (!version) return
    const handle = window.setTimeout(() => {
      void invoke('dismiss_update_toast').catch(console.error)
    }, AUTO_DISMISS_MS)
    return () => window.clearTimeout(handle)
  }, [version])

  function handleAccept() {
    void invoke('accept_update_toast').catch(console.error)
  }

  function handleDismiss() {
    void invoke('dismiss_update_toast').catch(console.error)
  }

  if (!version) return null

  return (
    <div className="toast-frame">
      <div className="toast-bubble">
        <div className="toast-greeting">{t('toast.update.greeting')}</div>
        <div className="toast-body">{t('toast.update.body', { version })}</div>
        <div className="toast-actions">
          <button type="button" className="toast-btn toast-btn-primary" onClick={handleAccept}>
            {t('toast.update.update')}
          </button>
          <button type="button" className="toast-btn toast-btn-ghost" onClick={handleDismiss}>
            {t('toast.update.later')}
          </button>
        </div>
      </div>
      <div className="toast-tail" aria-hidden="true" />
    </div>
  )
}
