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

const WAVE_BARS = 14

// Timings for the first-run reveal ceremony — shown once when the onboarding
// Test step mounts. Bubble auto-dismisses after BUBBLE_MS or on the first
// actual recording, whichever comes first. If the user stays idle for
// WIGGLE_AFTER_MS, the pill does a single attention wiggle. Wiggle comes
// early enough that it reliably fires before most users have stopped
// reading the onboarding copy; the bubble survives through the wiggle.
const REVEAL_POP_MS = 780
const BUBBLE_MS = 12000
const WIGGLE_AFTER_MS = 7000
const WIGGLE_DURATION_MS = 900

export default function StatusBar() {
  const { t } = useTranslation()
  const [appState, setAppState] = useState<AppState>('idle')
  const [elapsed, setElapsed] = useState(0)
  const [levels, setLevels] = useState<number[]>([])
  const startRef = useRef<number | null>(null)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // First-run reveal state
  const [revealing, setRevealing] = useState(false)
  const [bubbleText, setBubbleText] = useState<string | null>(null)
  const [wiggle, setWiggle] = useState(false)
  const bubbleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const wiggleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

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

  // Reveal ceremony: fires once per onboarding completion. Payload carries
  // the bubble text in whatever locale the main window is using, so the pill
  // never has to own its own i18n state.
  useEffect(() => {
    const unlisten = listen<{ bubble?: string }>('pill-animate-reveal', (e) => {
      setRevealing(true)
      // Drop the revealing flag after the pop-in animation ends so the
      // keyframe class doesn't restart on unrelated re-renders.
      setTimeout(() => setRevealing(false), REVEAL_POP_MS)

      const text = e.payload?.bubble ?? null
      if (text) {
        setBubbleText(text)
        if (bubbleTimerRef.current) clearTimeout(bubbleTimerRef.current)
        bubbleTimerRef.current = setTimeout(() => setBubbleText(null), BUBBLE_MS)
      }

      if (wiggleTimerRef.current) clearTimeout(wiggleTimerRef.current)
      wiggleTimerRef.current = setTimeout(() => {
        setWiggle(true)
        setTimeout(() => setWiggle(false), WIGGLE_DURATION_MS)
      }, WIGGLE_AFTER_MS)
    })
    return () => {
      unlisten.then((fn) => fn())
      if (bubbleTimerRef.current) clearTimeout(bubbleTimerRef.current)
      if (wiggleTimerRef.current) clearTimeout(wiggleTimerRef.current)
    }
  }, [])

  useEffect(() => {
    const unlisten = listen<{ level: number }>('audio-level', (e) => {
      setLevels((prev) => {
        const next = prev.length >= WAVE_BARS ? prev.slice(1) : prev.slice()
        next.push(e.payload.level)
        return next
      })
    })
    return () => { unlisten.then((fn) => fn()) }
  }, [])

  useEffect(() => {
    if (appState === 'recording') {
      startRef.current = Date.now()
      timerRef.current = setInterval(() => {
        setElapsed(Math.floor((Date.now() - (startRef.current ?? Date.now())) / 1000))
      }, 1000)
      // First real recording cancels the onboarding ceremony.
      setBubbleText(null)
      if (bubbleTimerRef.current) {
        clearTimeout(bubbleTimerRef.current)
        bubbleTimerRef.current = null
      }
      if (wiggleTimerRef.current) {
        clearTimeout(wiggleTimerRef.current)
        wiggleTimerRef.current = null
      }
    } else {
      if (timerRef.current) clearInterval(timerRef.current)
      setElapsed(0)
      startRef.current = null
      setLevels([])
    }
    return () => { if (timerRef.current) clearInterval(timerRef.current) }
  }, [appState])

  function formatTime(s: number) {
    const m = Math.floor(s / 60)
    const sec = s % 60
    return `${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
  }

  const revealClass = [
    'pill-outer',
    revealing ? 'is-revealing' : '',
    wiggle ? 'is-wiggling' : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <div className={revealClass}>
      {bubbleText && (
        <div className="pill-bubble" role="status" aria-live="polite">
          {bubbleText}
        </div>
      )}
      {appState === 'idle' ? (
        <div key="collapsed" className="pill-collapsed" />
      ) : appState === 'recording' ? (
        <div key="recording" className="pill-wave">
          <div className="rec-label">
            <span className="ember" />
            REC
          </div>
          <Waveform levels={levels} bars={WAVE_BARS} />
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

function Waveform({ levels, bars }: { levels: number[]; bars: number }) {
  const padded = Array.from({ length: bars }, (_, i) => {
    const offset = levels.length - bars + i
    return offset >= 0 ? levels[offset] : 0
  })
  return (
    <div className="waveform">
      {padded.map((level, i) => (
        <span
          key={i}
          className="bar"
          style={{ height: `${Math.max(16, level * 100)}%` }}
        />
      ))}
    </div>
  )
}
