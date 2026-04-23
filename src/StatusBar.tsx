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
// WIGGLE_AFTER_MS, the pill does a single attention wiggle.
const REVEAL_POP_MS = 820
const BUBBLE_MS = 12000
const WIGGLE_AFTER_MS = 7000
const WIGGLE_DURATION_MS = 900

const EMBER = '#C65441'
const PILL_COLLAPSED_BG = 'rgba(30,30,30,0.55)'
const PILL_COLLAPSED_SHADOW =
  '0 1px 4px rgba(0,0,0,0.35), 0 0 0 0.5px rgba(255,255,255,0.12)'
const EMBER_SHADOW =
  '0 0 42px 18px rgba(198,84,65,0.6), 0 0 0 0.5px rgba(255,255,255,0.15)'

const prefersReducedMotion = () =>
  typeof window !== 'undefined' &&
  window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

export default function StatusBar() {
  const { t } = useTranslation()
  const [appState, setAppState] = useState<AppState>('idle')
  const [elapsed, setElapsed] = useState(0)
  const [levels, setLevels] = useState<number[]>([])
  const startRef = useRef<number | null>(null)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // Reveal ceremony state
  const [bubbleText, setBubbleText] = useState<string | null>(null)
  const bubbleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const wiggleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const outerRef = useRef<HTMLDivElement | null>(null)
  const contentRef = useRef<HTMLDivElement | null>(null)

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

  // Reveal ceremony: fires once per onboarding completion. We drive both
  // the outer swoop and the child's ember-to-dark colour transition via
  // the Web Animations API instead of CSS classes — CSS keyframes were
  // being swallowed by the OS compositor in the first ~200 ms after the
  // hidden webview became visible, so the user saw the pill appear with
  // no motion. Imperative .animate() bypasses that whole class of timing
  // bugs; the browser runs it deterministically on the main frame.
  useEffect(() => {
    const unlisten = listen<{ bubble?: string }>('pill-animate-reveal', (e) => {
      const text = e.payload?.bubble ?? null
      if (text) {
        setBubbleText(text)
        if (bubbleTimerRef.current) clearTimeout(bubbleTimerRef.current)
        bubbleTimerRef.current = setTimeout(() => setBubbleText(null), BUBBLE_MS)
      }

      // Defer one frame so the refs are attached after any pending React
      // re-render (the bubble mounts in the same tick).
      requestAnimationFrame(() => {
        playRevealAnimation()
      })

      // Reset and schedule the attention wiggle.
      if (wiggleTimerRef.current) clearTimeout(wiggleTimerRef.current)
      wiggleTimerRef.current = setTimeout(() => {
        playWiggleAnimation()
      }, WIGGLE_AFTER_MS)
    })
    return () => {
      unlisten.then((fn) => fn())
      if (bubbleTimerRef.current) clearTimeout(bubbleTimerRef.current)
      if (wiggleTimerRef.current) clearTimeout(wiggleTimerRef.current)
    }
  }, [])

  function playRevealAnimation() {
    const outer = outerRef.current
    const content = contentRef.current
    const reduceMotion = prefersReducedMotion()

    if (outer) {
      if (reduceMotion) {
        outer.animate(
          [
            { opacity: 0 },
            { opacity: 1 },
          ],
          { duration: 240, easing: 'ease-out', fill: 'forwards' }
        )
      } else {
        outer.animate(
          [
            { opacity: 0, transform: 'scale(0) rotate(-200deg) translateY(0)' },
            { opacity: 1, transform: 'scale(1.55) rotate(-12deg) translateY(-10px)', offset: 0.28 },
            {             transform: 'scale(1.18) rotate(10deg) translateY(-6px)',  offset: 0.52 },
            {             transform: 'scale(0.92) rotate(-4deg) translateY(-2px)',  offset: 0.72 },
            {             transform: 'scale(1.06) rotate(2deg) translateY(0)',       offset: 0.88 },
            { opacity: 1, transform: 'scale(1) rotate(0) translateY(0)' },
          ],
          { duration: REVEAL_POP_MS, easing: 'cubic-bezier(0.22, 1.25, 0.36, 1)', fill: 'forwards' }
        )
      }
    }

    if (content && !reduceMotion) {
      // Ember flood → cool-down to the normal collapsed dark. Keeps the
      // same duration as the outer swoop so both end in lockstep.
      content.animate(
        [
          { background: EMBER, boxShadow: EMBER_SHADOW, width: '44px' },
          { background: EMBER, boxShadow: EMBER_SHADOW, width: '56px', offset: 0.28 },
          { background: EMBER, boxShadow: EMBER_SHADOW, width: '44px', offset: 0.52 },
          { background: 'rgba(120,65,45,0.75)', boxShadow: '0 2px 10px rgba(198,84,65,0.35), 0 0 0 0.5px rgba(255,255,255,0.12)', offset: 0.78 },
          { background: PILL_COLLAPSED_BG, boxShadow: PILL_COLLAPSED_SHADOW, width: '44px' },
        ],
        { duration: REVEAL_POP_MS, easing: 'cubic-bezier(0.22, 1.25, 0.36, 1)', fill: 'forwards' }
      )
    }
  }

  function playWiggleAnimation() {
    const outer = outerRef.current
    if (!outer || prefersReducedMotion()) return

    outer.animate(
      [
        { transform: 'rotate(0) translateY(0)' },
        { transform: 'rotate(-9deg) translateY(-4px)', offset: 0.10 },
        { transform: 'rotate(8deg) translateY(-6px)',  offset: 0.25 },
        { transform: 'rotate(-6deg) translateY(-2px)', offset: 0.40 },
        { transform: 'rotate(5deg) translateY(-3px)',  offset: 0.55 },
        { transform: 'rotate(-3deg) translateY(-1px)', offset: 0.70 },
        { transform: 'rotate(1.5deg) translateY(0)',   offset: 0.85 },
        { transform: 'rotate(0) translateY(0)' },
      ],
      { duration: WIGGLE_DURATION_MS, easing: 'cubic-bezier(0.36, 0.07, 0.19, 0.97)', fill: 'none' }
    )
  }

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

  return (
    <div className="pill-outer" ref={outerRef}>
      {bubbleText && (
        <div className="pill-bubble" role="status" aria-live="polite">
          {bubbleText}
        </div>
      )}
      {appState === 'idle' ? (
        <div key="collapsed" className="pill-collapsed" ref={contentRef} />
      ) : appState === 'recording' ? (
        <div key="recording" className="pill-wave" ref={contentRef}>
          <div className="rec-label">
            <span className="ember" />
            REC
          </div>
          <Waveform levels={levels} bars={WAVE_BARS} />
          <span className="pill-timer">{formatTime(elapsed)}</span>
        </div>
      ) : (
        <div key={appState} className={`pill ${appState}`} ref={contentRef}>
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
