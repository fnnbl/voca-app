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
    if (!outer) return

    // Kill any in-flight CSS or WAAPI animations on the elements we're
    // about to touch — otherwise the pillFadeIn that fires on
    // `.pill-collapsed` mount (or a previous reveal pass) competes with
    // the inline styles and either wins or cancels ours out.
    outer.getAnimations?.().forEach((a) => a.cancel())
    content?.getAnimations?.().forEach((a) => a.cancel())

    if (prefersReducedMotion()) {
      return
    }

    // Stage the starting pose with transition disabled so the browser
    // paints it at scale(0) first, then re-enable transition and swap to
    // the target pose. The forced reflow between the two phases is the
    // key — without it the browser coalesces both style writes into one
    // paint and we never see the animation.
    outer.style.transition = 'none'
    outer.style.transformOrigin = 'center bottom'
    outer.style.transform = 'scale(0) rotate(-200deg)'
    outer.style.opacity = '0'

    if (content) {
      content.style.transition = 'none'
      content.style.backgroundColor = EMBER
      content.style.boxShadow = EMBER_SHADOW
    }

    // Force the browser to lay out and paint the starting state before we
    // queue the transition.
    void outer.offsetHeight

    requestAnimationFrame(() => {
      // Stage 1 — explosive pop to an overshoot peak.
      outer.style.transition =
        'transform 360ms cubic-bezier(0.22, 1.5, 0.36, 1), opacity 180ms ease-out'
      outer.style.transform = 'scale(1.55) rotate(-12deg) translateY(-10px)'
      outer.style.opacity = '1'

      // Stage 2 — settle back to rest with the colour cooling from ember
      // to the normal dark. The delay matches stage 1 so the two stages
      // cross seamlessly.
      setTimeout(() => {
        outer.style.transition = 'transform 520ms cubic-bezier(0.34, 1.56, 0.64, 1)'
        outer.style.transform = 'scale(1) rotate(0) translateY(0)'

        if (content) {
          content.style.transition =
            'background-color 520ms ease-out, box-shadow 520ms ease-out'
          content.style.backgroundColor = PILL_COLLAPSED_BG
          content.style.boxShadow = PILL_COLLAPSED_SHADOW
        }
      }, 380)

      // Stage 3 — clear the inline overrides so future state changes
      // (switching to pill-wave on record, etc.) pick up normal CSS.
      setTimeout(() => {
        outer.style.transition = ''
        outer.style.transform = ''
        outer.style.opacity = ''
        outer.style.transformOrigin = ''
        if (content) {
          content.style.transition = ''
          content.style.backgroundColor = ''
          content.style.boxShadow = ''
        }
      }, REVEAL_POP_MS + 120)
    })
  }

  function playWiggleAnimation() {
    const outer = outerRef.current
    if (!outer || prefersReducedMotion()) return

    // Wiggle fires 7 s after the reveal, long after the webview has been
    // stable — CSS keyframe animations are reliable at that point. Use a
    // class toggle with a reflow in between so the animation restarts if
    // it was somehow already marked "used".
    outer.classList.remove('is-wiggling')
    void outer.offsetHeight
    outer.classList.add('is-wiggling')

    setTimeout(() => {
      outer.classList.remove('is-wiggling')
    }, WIGGLE_DURATION_MS + 50)
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
