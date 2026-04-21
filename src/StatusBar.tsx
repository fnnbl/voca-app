import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useTranslation } from 'react-i18next'

type AppState = 'idle' | 'recording' | 'processing' | 'inserting' | 'error'

const LABELS: Record<AppState, string> = {
  idle:       'state.idle',
  recording:  'state.recording',
  processing: 'state.processing',
  inserting:  'state.inserting',
  error:      'state.error',
}

export default function StatusBar() {
  const { t } = useTranslation()
  const [appState, setAppState] = useState<AppState>('idle')
  const [elapsed, setElapsed] = useState(0)
  const startRef = useRef<number | null>(null)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    getCurrentWebviewWindow().setIgnoreCursorEvents(true).catch(() => {})
  }, [])

  useEffect(() => {
    invoke<AppState>('get_app_state').then(setAppState).catch(() => {})
    const unlisten = listen<{ state: AppState }>('recording-state-changed', (e) => {
      setAppState(e.payload.state)
    })
    return () => { unlisten.then((fn) => fn()) }
  }, [])

  useEffect(() => {
    if (appState === 'recording') {
      startRef.current = Date.now()
      timerRef.current = setInterval(() => {
        setElapsed(Math.floor((Date.now() - (startRef.current ?? Date.now())) / 1000))
      }, 1000)
    } else {
      if (timerRef.current) clearInterval(timerRef.current)
      setElapsed(0)
      startRef.current = null
    }
    return () => { if (timerRef.current) clearInterval(timerRef.current) }
  }, [appState])

  function formatTime(s: number) {
    const m = Math.floor(s / 60)
    const sec = s % 60
    return `${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
  }

  return (
    <div className="pill-outer">
      {appState === 'idle' ? (
        <div key="collapsed" className="pill-collapsed" />
      ) : appState === 'recording' ? (
        <div key="recording" className="pill-wave">
          <div className="rec-label">
            <span className="ember" />
            REC
          </div>
          <Waveform />
          <span className="pill-timer">{formatTime(elapsed)}</span>
        </div>
      ) : (
        <div key={appState} className={`pill ${appState}`}>
          {appState === 'processing'
            ? <span className="pill-spinner" />
            : <span className="pill-dot" />}
          <span>{t(LABELS[appState])}</span>
        </div>
      )}
    </div>
  )
}

function Waveform({ bars = 14 }: { bars?: number }) {
  const seeds = Array.from({ length: bars }, (_, i) =>
    (Math.sin(i * 1.7) + Math.cos(i * 0.7)) * 0.5 + 0.5
  )
  return (
    <div className="waveform">
      {seeds.map((_, i) => (
        <span
          key={i}
          className="bar"
          style={{
            animationDelay: `${(i * 0.07) % 1.1}s`,
            animationDuration: `${0.9 + (i % 5) * 0.08}s`,
          }}
        />
      ))}
    </div>
  )
}
