import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-shell'
import { useShortcutCapture, sortShortcut } from '../hooks/useShortcutCapture'
import { DEFAULT_SHORTCUT } from '../types'
import type { Settings } from '../types'
import { formatShortcutKey } from '../shortcut/format'

interface Props {
  settings: Settings
  onComplete: (updated: Settings) => Promise<void>
}

type CloudProvider = Settings['transcription']['cloudProvider']
type AiProvider = Settings['aiEnhancement']['provider']

const CLOUD_PROVIDERS: { id: CloudProvider; label: string; meta: string; free: boolean; keyLink?: string; placeholder: string }[] = [
  { id: 'groq',       label: 'Groq',       meta: 'whisper turbo · free', free: true,  keyLink: 'https://console.groq.com/keys',                    placeholder: 'gsk_...' },
  { id: 'openai',     label: 'OpenAI',     meta: 'whisper-1 · paid',      free: false, keyLink: 'https://platform.openai.com/api-keys',             placeholder: 'sk-...' },
  { id: 'deepgram',   label: 'Deepgram',   meta: 'nova-3 · free',         free: true,  keyLink: 'https://console.deepgram.com',                     placeholder: 'deepgram key...' },
  { id: 'elevenlabs', label: 'ElevenLabs', meta: 'scribe-v1 · paid',      free: false, keyLink: 'https://elevenlabs.io/app/settings/api-keys',      placeholder: 'el_...' },
  { id: 'gemini',     label: 'Gemini',     meta: 'google · free',         free: true,  keyLink: 'https://aistudio.google.com/app/apikey',           placeholder: 'AIza...' },
  { id: 'custom',     label: 'Custom',     meta: 'eigener endpoint',      free: false, placeholder: 'API key...' },
]

const AI_PROVIDERS: { id: AiProvider; label: string; meta: string; free: boolean; keyLink?: string; placeholder: string }[] = [
  { id: 'anthropic', label: 'Anthropic',  meta: 'claude haiku · paid',    free: false, keyLink: 'https://console.anthropic.com/settings/keys', placeholder: 'sk-ant-...' },
  { id: 'groq',      label: 'Groq',       meta: 'llama 3.3 · free',       free: true,  keyLink: 'https://console.groq.com/keys',               placeholder: 'gsk_...' },
  { id: 'openai',    label: 'OpenAI',     meta: 'gpt-4o · paid',          free: false, keyLink: 'https://platform.openai.com/api-keys',        placeholder: 'sk-...' },
  { id: 'gemini',    label: 'Gemini',     meta: '2.0 flash · free',       free: true,  keyLink: 'https://aistudio.google.com/app/apikey',      placeholder: 'AIza...' },
  { id: 'ollama',    label: 'Ollama',     meta: 'lokal · free',           free: true,  placeholder: '' },
]

type ModelSize = 'tiny' | 'base' | 'small' | 'medium'
interface ModelStatus { downloaded: boolean; size_bytes: number }
interface DownloadProgress { size: string; downloaded_bytes: number; total_bytes: number }

const MODEL_SIZES: { value: ModelSize; label: string; approxMb: number }[] = [
  { value: 'tiny',   label: 'Tiny',   approxMb: 75 },
  { value: 'base',   label: 'Base',   approxMb: 142 },
  { value: 'small',  label: 'Small',  approxMb: 466 },
  { value: 'medium', label: 'Medium', approxMb: 1500 },
]

const STEPS = ['welcome', 'transcription', 'shortcut', 'useCase', 'test', 'ai', 'done'] as const
type Step = typeof STEPS[number]

type UseCaseId = 'dev' | 'pm' | 'content' | 'design' | 'consulting'
const USE_CASE_IDS: UseCaseId[] = ['dev', 'pm', 'content', 'design', 'consulting']

export function OnboardingPage({ settings, onComplete }: Props) {
  const [step, setStep] = useState<Step>('welcome')
  const [local, setLocal] = useState<Settings>(settings)

  // Force the current default shortcut at onboarding start. Existing installs
  // may still have the legacy "Ctrl+Shift+Space" in their settings.json, which
  // would mean the "default" shown on step 2 and the key registered in the
  // backend disagree with the user's expectation (Ctrl+Win / Ctrl+Cmd).
  useEffect(() => {
    if (settings.shortcuts.key === DEFAULT_SHORTCUT) return
    invoke('register_shortcut', { newShortcut: DEFAULT_SHORTCUT }).catch(() => {})
    setLocal((prev) => ({ ...prev, shortcuts: { key: DEFAULT_SHORTCUT } }))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const stepIndex = STEPS.indexOf(step)
  const totalVisible = STEPS.length - 1 // exclude 'done' from counter

  async function finish(updated: Settings) {
    await onComplete({ ...updated, general: { ...updated.general, onboardingCompleted: true } })
  }

  function next() { setStep(STEPS[Math.min(stepIndex + 1, STEPS.length - 1)] as Step) }
  function back() { setStep(STEPS[Math.max(stepIndex - 1, 0)] as Step) }

  const showRail = step !== 'welcome' && step !== 'done'
  const showFoot = step !== 'welcome' && step !== 'done' && stepIndex > 0

  return (
    <div className="onb-outer">
      {showRail && (
        <div className="onb-rail">
          {STEPS.slice(1, -1).map((s, i) => {
            const idx = i + 1
            return (
              <div
                key={s}
                className={`rail-dot${idx === stepIndex ? ' is-active' : ''}${idx < stepIndex ? ' is-done' : ''}`}
              />
            )
          })}
          <button className="onb-skip" onClick={() => finish(local)}>skip</button>
        </div>
      )}

      {step === 'welcome'      && <StepWelcome onNext={next} />}
      {step === 'transcription' && <StepTranscription settings={local} onChange={setLocal} onNext={next} />}
      {step === 'shortcut'     && <StepShortcut settings={local} onChange={setLocal} onNext={next} />}
      {step === 'useCase'      && <StepUseCase onNext={next} />}
      {step === 'test'         && <StepTest onNext={next} />}
      {step === 'ai'           && <StepAi settings={local} onChange={setLocal} onNext={next} />}
      {step === 'done'         && <StepFinale settings={local} onFinish={finish} />}

      {showFoot && (
        <div className="onb-foot">
          <button className="onb-back" onClick={back}><BackIcon /> zurück</button>
          <div className="onb-counter">{stepIndex} / {totalVisible - 1}</div>
        </div>
      )}
    </div>
  )
}

/* ── Welcome ─────────────────────────────────────────────────────────────── */

function StepWelcome({ onNext }: { onNext: () => void }) {
  return (
    <div className="onb-stage" style={{ justifyContent: 'center', alignItems: 'center' }}>
      <div className="ember-mark" />
      <div style={{ marginBottom: 36 }}>
        <WelcomeLogoMark size={132} />
      </div>
      <div className="onb-eyebrow">willkommen bei</div>
      <h1 className="onb-title" style={{ fontSize: 88, marginBottom: 24 }}>
        <span className="v-wordmark" style={{ fontSize: 'inherit' }}>VOCA</span>
      </h1>
      <p className="onb-lede" style={{ fontSize: 17, maxWidth: '44ch' }}>
        Sprich — und dein Gerät schreibt. Kein Fenster vor der Nase, kein Dialog. Nur eine ruhige Pille am unteren Rand.
      </p>
      <div style={{ display: 'flex', alignItems: 'center', gap: 20 }}>
        <button className="v-btn accent" onClick={onNext}>
          Einrichten <ChevronIcon />
        </button>
        <span className="v-meta">dauert keine 90 sekunden.</span>
      </div>
    </div>
  )
}

/* ── Transcription ───────────────────────────────────────────────────────── */

function StepTranscription({
  settings, onChange, onNext,
}: { settings: Settings; onChange: (s: Settings) => void; onNext: () => void }) {
  const mode = settings.transcription.mode
  const provider = settings.transcription.cloudProvider ?? 'groq'
  const provMeta = CLOUD_PROVIDERS.find((p) => p.id === provider) ?? CLOUD_PROVIDERS[0]
  const [apiKey, setApiKey] = useState('')
  const [showOffline, setShowOffline] = useState(mode === 'local')
  const [modelStatuses, setModelStatuses] = useState<Record<ModelSize, ModelStatus | null>>({ tiny: null, base: null, small: null, medium: null })
  const [downloading, setDownloading] = useState<ModelSize | null>(null)
  const [progress, setProgress] = useState<DownloadProgress | null>(null)

  useEffect(() => {
    invoke<string | null>('get_transcription_key', { provider }).then((k) => setApiKey(k ?? '')).catch(() => {})
  }, [provider])

  useEffect(() => {
    MODEL_SIZES.forEach(({ value }) => {
      invoke<ModelStatus>('get_model_status', { size: value }).then((s) => setModelStatuses((prev) => ({ ...prev, [value]: s }))).catch(() => {})
    })
    const unlisten = listen<DownloadProgress>('model-download-progress', (e) => setProgress(e.payload))
    return () => { unlisten.then((fn) => fn()) }
  }, [])

  function selectProvider(id: CloudProvider) {
    setShowOffline(false)
    onChange({ ...settings, transcription: { ...settings.transcription, mode: 'cloud', cloudProvider: id } })
  }
  function setOffline(on: boolean) {
    setShowOffline(on)
    onChange({ ...settings, transcription: { ...settings.transcription, mode: on ? 'local' : 'cloud' } })
  }
  async function saveKey() { if (apiKey) await invoke('save_transcription_key', { provider, value: apiKey }) }
  async function handleDownload(size: ModelSize) {
    setDownloading(size); setProgress(null)
    try {
      await invoke('download_model', { size })
      const s = await invoke<ModelStatus>('get_model_status', { size })
      setModelStatuses((prev) => ({ ...prev, [size]: s }))
    } finally { setDownloading(null); setProgress(null) }
  }
  async function handleDelete(size: ModelSize) {
    try { await invoke('delete_model', { size }); setModelStatuses((prev) => ({ ...prev, [size]: { downloaded: false, size_bytes: 0 } })) } catch { /* ignore */ }
  }
  async function handleCancel() { try { await invoke('cancel_model_download') } catch { /* ignore */ } }
  async function handleNext() { if (mode === 'cloud') await saveKey(); onNext() }

  const otherCloud = CLOUD_PROVIDERS.filter((p) => p.id !== 'groq')

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">schritt 01 · transkription</div>
      <h1 className="onb-title">Wo soll <em>gehört</em> werden?</h1>
      <p className="onb-lede">
        VOCA unterstützt schnelle Cloud-Modelle und lokale Offline-Modelle. Du kannst jederzeit wechseln.
      </p>

      {/* Provider grid */}
      <div className="prov-grid" style={{ marginBottom: 16 }}>
        <button
          className={`prov-card featured${!showOffline && provider === 'groq' ? ' is-active' : ''}`}
          onClick={() => selectProvider('groq')}
        >
          <span className="prov-badge">empfohlen</span>
          <div className="prov-name">Groq</div>
          <div className="prov-desc">Whisper large-v3-turbo — der schnellste Pfad. Kostenlose Stufe reicht für den Alltag.</div>
        </button>
        {otherCloud.map((p) => (
          <button
            key={p.id}
            className={`prov-card${!showOffline && provider === p.id ? ' is-active' : ''}`}
            onClick={() => selectProvider(p.id)}
          >
            <div className="prov-name">{p.label}</div>
            <div className={`prov-meta${p.free ? ' free' : ''}`}>{p.meta}</div>
          </button>
        ))}
        <button
          className={`prov-card${showOffline ? ' is-active' : ''}`}
          onClick={() => setOffline(!showOffline)}
        >
          <div className="prov-name">Offline</div>
          <div className="prov-meta free">whisper.cpp · lokal</div>
        </button>
      </div>

      {/* API key or offline models */}
      {!showOffline && (
        <div style={{ display: 'flex', gap: 10, marginBottom: 20 }}>
          <input
            className="v-input"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={provMeta.placeholder}
            style={{ flex: 1, maxWidth: 360 }}
          />
          {provMeta.keyLink && (
            <button className="v-btn ghost" onClick={() => open(provMeta.keyLink!)}>
              <ExternalIcon /> schlüssel holen
            </button>
          )}
        </div>
      )}

      {showOffline && (
        <div style={{ marginBottom: 20 }}>
          {MODEL_SIZES.map(({ value, label, approxMb }) => {
            const status = modelStatuses[value]
            const isSelected = settings.transcription.localModelSize === value
            const isDownloaded = status?.downloaded ?? false
            const isDownloading = downloading === value
            return (
              <div
                key={value}
                className={`model-row${isSelected ? ' is-active' : ''}`}
                onClick={() => onChange({ ...settings, transcription: { ...settings.transcription, localModelSize: value } })}
              >
                <span className="radio" />
                <span className="model-name">{label}</span>
                <span className="model-size">~{approxMb < 1000 ? `${approxMb} MB` : `${(approxMb / 1000).toFixed(1)} GB`}</span>
                <div className="model-action">
                  {isDownloading && progress ? (
                    <>
                      <span className="v-meta">{Math.round((progress.downloaded_bytes / progress.total_bytes) * 100)}%</span>
                      <div className="pbar-wrap"><span className="pbar-fill" style={{ width: `${(progress.downloaded_bytes / progress.total_bytes) * 100}%` }} /></div>
                      <button className="v-btn ghost sm" onClick={(e) => { e.stopPropagation(); handleCancel() }}>abbrechen</button>
                    </>
                  ) : isDownloaded ? (
                    <>
                      <span className="model-status-dot" />
                      <span className="v-meta">bereit</span>
                      <button className="v-btn ghost sm" onClick={(e) => { e.stopPropagation(); handleDelete(value) }}><TrashIcon /></button>
                    </>
                  ) : (
                    <button className="v-btn ghost sm" disabled={downloading !== null} onClick={(e) => { e.stopPropagation(); handleDownload(value) }}>
                      <DownloadIcon /> laden
                    </button>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      <button className="v-btn accent" onClick={handleNext}>Weiter <ChevronIcon /></button>
    </div>
  )
}

/* ── Shortcut ────────────────────────────────────────────────────────────── */

function StepShortcut({
  settings, onChange, onNext,
}: { settings: Settings; onChange: (s: Settings) => void; onNext: () => void }) {
  const currentKey = settings.shortcuts.key || DEFAULT_SHORTCUT

  async function handleKeyChange(newShortcut: string) {
    try { await invoke('register_shortcut', { newShortcut }) } catch { /* conflict handled by backend */ }
    onChange({ ...settings, shortcuts: { key: newShortcut } })
  }

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">schritt 02 · shortcut</div>
      <h1 className="onb-title">Drück, halt, <em>sprich.</em></h1>
      <p className="onb-lede">
        Halt die Tastenkombi gedrückt, während du sprichst. Loslassen transkribiert. Klick das Feld zum Neubelegen.
      </p>

      <div style={{ display: 'flex', alignItems: 'center', gap: 20, marginBottom: 24 }}>
        <KbdShortcutField value={currentKey} onChange={handleKeyChange} />
      </div>

      <div style={{ display: 'flex', gap: 18, alignItems: 'flex-start', marginBottom: 28 }}>
        <div style={{ width: 3, background: 'var(--v-accent)', alignSelf: 'stretch', borderRadius: 2, flexShrink: 0 }} />
        <div>
          <div className="v-label" style={{ marginBottom: 4 }}>tipp</div>
          <p style={{ fontSize: 13.5, color: 'var(--v-ink-2)', maxWidth: '50ch', lineHeight: 1.6, fontFamily: 'var(--f-ui)' }}>
            Press-&amp;-hold fühlt sich natürlicher an als Toggle — du weißt jederzeit, was aufgenommen wird.
          </p>
        </div>
      </div>

      <button className="v-btn accent" onClick={onNext}>Weiter <ChevronIcon /></button>
    </div>
  )
}

/* ── Use-case (dictionary seeding) ──────────────────────────────────────── */

function StepUseCase({ onNext }: { onNext: () => void }) {
  const { t } = useTranslation()
  const [selected, setSelected] = useState<Set<UseCaseId>>(new Set())
  const [submitting, setSubmitting] = useState(false)

  function toggle(id: UseCaseId) {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  async function handleNext() {
    if (selected.size === 0) {
      onNext()
      return
    }
    setSubmitting(true)
    try {
      await invoke('seed_dictionary_with_use_cases', { useCases: Array.from(selected) })
    } catch (e) {
      console.error('dictionary seeding failed:', e)
    } finally {
      setSubmitting(false)
      onNext()
    }
  }

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">schritt 03 · fachbereich</div>
      <h1 className="onb-title">{t('onboarding.useCase.title', 'Womit arbeitest du?')}</h1>
      <p className="onb-lede">
        {t('onboarding.useCase.description', 'VOCA merkt sich typische Begriffe aus deinem Fachbereich, damit Whisper sie gleich richtig versteht. Mehrfachauswahl ist OK. Kannst du später jederzeit anpassen.')}
      </p>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
          gap: 10,
          marginBottom: 28,
          width: '100%',
          maxWidth: 720,
        }}
      >
        {USE_CASE_IDS.map((id) => {
          const isOn = selected.has(id)
          return (
            <button
              key={id}
              type="button"
              onClick={() => toggle(id)}
              className={`prov-card${isOn ? ' is-active' : ''}`}
              style={{ textAlign: 'left' }}
            >
              <div className="prov-name">
                {t(`onboarding.useCase.category.${id}.label`)}
              </div>
              <div className="prov-desc">
                {t(`onboarding.useCase.category.${id}.description`)}
              </div>
            </button>
          )
        })}
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
        <button
          className="v-btn accent"
          onClick={handleNext}
          disabled={submitting}
        >
          {selected.size > 0
            ? t('onboarding.useCase.next', 'Weiter')
            : t('onboarding.useCase.skip', 'Überspringen')}
          <ChevronIcon />
        </button>
        {selected.size > 0 && (
          <span className="v-meta">
            {t('onboarding.useCase.selectionHint', { count: selected.size, defaultValue: '{{count}} ausgewählt' })}
          </span>
        )}
      </div>
    </div>
  )
}

/* ── Test ────────────────────────────────────────────────────────────────── */

type TestAppState = 'idle' | 'recording' | 'processing' | 'inserting' | 'error'

function StepTest({ onNext }: { onNext: () => void }) {
  const [appState, setAppState] = useState<TestAppState>('idle')
  const [text, setText] = useState('')

  useEffect(() => {
    const unlistenState = listen<{ state: TestAppState }>('recording-state-changed', (e) => {
      setAppState(e.payload.state)
    })
    // Direct result listener avoids the simulated Ctrl+V hop — paste into
    // VOCA's own webview isn't reliable; we just render the transcript here.
    const unlistenResult = listen<{ text: string }>('transcription-result', (e) => {
      const incoming = e.payload.text.trim()
      if (!incoming) return
      setText((prev) => (prev ? `${prev} ${incoming}` : incoming))
    })
    return () => {
      unlistenState.then((fn) => fn())
      unlistenResult.then((fn) => fn())
    }
  }, [])

  const trimmed = text.trim()
  const wordCount = trimmed ? trimmed.split(/\s+/).filter(Boolean).length : 0

  let statusLabel = 'warte auf shortcut…'
  let accentMic = false
  if (appState === 'recording') {
    statusLabel = 'nehme auf… sprich weiter.'
    accentMic = true
  } else if (appState === 'processing') {
    statusLabel = 'transkribiere…'
    accentMic = true
  } else if (appState === 'inserting') {
    statusLabel = 'einfügen…'
    accentMic = true
  }

  return (
    <div className="onb-stage" style={{ textAlign: 'center', alignItems: 'center' }}>
      <div className="onb-eyebrow" style={{ alignSelf: 'center' }}>schritt 04 · dein erster versuch</div>
      <h1 className="onb-title" style={{ textAlign: 'center', maxWidth: '16ch' }}>
        Halt den Shortcut und sag irgendwas.
      </h1>

      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14, marginTop: 12, width: '100%', maxWidth: 560 }}>
        <div className="mic-disc" style={accentMic ? { borderColor: 'var(--v-accent)' } : undefined}>
          <MicSvg color={accentMic ? 'var(--v-accent)' : undefined} />
        </div>
        <p className="v-meta">{statusLabel}</p>

        <div
          aria-label="transkript"
          style={{
            width: '100%',
            minHeight: 120,
            padding: 14,
            background: 'var(--v-bg)',
            border: `1px solid ${accentMic ? 'var(--v-accent)' : 'var(--v-line)'}`,
            borderRadius: 'var(--r-2)',
            color: trimmed ? 'var(--v-ink)' : 'var(--v-ink-3)',
            fontFamily: 'var(--f-ui)',
            fontSize: 14,
            lineHeight: 1.5,
            textAlign: 'left',
            whiteSpace: 'pre-wrap',
            transition: 'border-color 120ms ease',
            userSelect: 'text',
          }}
        >
          {trimmed || '—'}
        </div>

        {trimmed && (
          <div className="rc-stats" style={{ justifyContent: 'center' }}>
            <div className="rc-stat"><span className="k">wörter</span><span className="v">{wordCount}</span></div>
            <div className="rc-stat"><span className="k">zeichen</span><span className="v">{text.length}</span></div>
          </div>
        )}

        <button
          className={trimmed ? 'v-btn accent' : 'v-btn ghost'}
          onClick={onNext}
          style={{ marginTop: 4 }}
        >
          {trimmed ? 'Perfekt. Weiter' : 'Überspringen'} <ChevronIcon />
        </button>
      </div>
    </div>
  )
}

/* ── AI Enhancement ──────────────────────────────────────────────────────── */

function StepAi({
  settings, onChange, onNext,
}: { settings: Settings; onChange: (s: Settings) => void; onNext: () => void }) {
  const provider = settings.aiEnhancement.provider
  const provMeta = AI_PROVIDERS.find((p) => p.id === provider) ?? AI_PROVIDERS[0]
  const [apiKey, setApiKey] = useState('')
  const transcProv = settings.transcription.cloudProvider
  const canReuse = (provider as string) === (transcProv as string) && provider !== 'ollama'

  useEffect(() => {
    if (provider === 'ollama') return
    invoke<string | null>('get_ai_provider_key', { provider }).then((k) => setApiKey(k ?? '')).catch(() => {})
  }, [provider])

  async function handleReuse() {
    const k = await invoke<string | null>('get_transcription_key', { provider: transcProv })
    if (k) setApiKey(k)
  }

  async function handleEnable() {
    if (provider !== 'ollama' && apiKey) await invoke('save_ai_provider_key', { provider, value: apiKey })
    await onNext()
    // onNext navigates to 'done'; settings will be saved with enabled:true via FinaleStep
    onChange({ ...settings, aiEnhancement: { ...settings.aiEnhancement, enabled: true } })
  }

  const others = AI_PROVIDERS.filter((p) => p.id !== 'groq')

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">schritt 05 · optional</div>
      <h1 className="onb-title">Rohtext oder <em>poliert</em>?</h1>
      <p className="onb-lede">
        Optional: VOCA leitet dein Transkript durch ein Sprachmodell — räumt Füllwörter auf, korrigiert Grammatik. Ton bleibt deiner.
      </p>

      <div className="prov-grid" style={{ marginBottom: 16 }}>
        <button
          className={`prov-card featured${provider === 'groq' ? ' is-active' : ''}`}
          onClick={() => onChange({ ...settings, aiEnhancement: { ...settings.aiEnhancement, provider: 'groq' } })}
        >
          <span className="prov-badge">empfohlen</span>
          <div className="prov-name">Groq</div>
          <div className="prov-desc">Llama 3.3 auf Groq — kostenlos, extrem schnell. Gleicher Key wie bei der Transkription.</div>
        </button>
        {others.map((p) => (
          <button
            key={p.id}
            className={`prov-card${provider === p.id ? ' is-active' : ''}`}
            onClick={() => onChange({ ...settings, aiEnhancement: { ...settings.aiEnhancement, provider: p.id } })}
          >
            <div className="prov-name">{p.label}</div>
            <div className={`prov-meta${p.free ? ' free' : ''}`}>{p.meta}</div>
          </button>
        ))}
      </div>

      {provider !== 'ollama' && (
        <div style={{ display: 'flex', gap: 10, marginBottom: 20 }}>
          <input
            className="v-input"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={provMeta.placeholder}
            style={{ flex: 1, maxWidth: 360 }}
          />
          {canReuse && !apiKey ? (
            <button className="v-btn ghost" onClick={handleReuse}>gleichen key nutzen</button>
          ) : provMeta.keyLink ? (
            <button className="v-btn ghost" onClick={() => open(provMeta.keyLink!)}>
              <ExternalIcon /> schlüssel holen
            </button>
          ) : null}
        </div>
      )}
      {provider === 'ollama' && (
        <p style={{ fontSize: 12, color: 'var(--v-ink-3)', marginBottom: 20, fontFamily: 'var(--f-ui)' }}>
          Stelle sicher, dass Ollama läuft und das gewählte Modell heruntergeladen ist.
        </p>
      )}

      <div style={{ display: 'flex', gap: 12 }}>
        <button className="v-btn accent" onClick={handleEnable}>Aktivieren <ChevronIcon /></button>
        <button className="v-btn ghost" onClick={onNext}>Später</button>
      </div>
    </div>
  )
}

/* ── Finale ──────────────────────────────────────────────────────────────── */

function StepFinale({ settings, onFinish }: { settings: Settings; onFinish: (s: Settings) => Promise<void> }) {
  const keys = (settings.shortcuts.key || DEFAULT_SHORTCUT).split('+').map((k) => formatShortcutKey(k))
  return (
    <div className="onb-stage" style={{ alignItems: 'center', textAlign: 'center', justifyContent: 'center' }}>
      <div className="onb-eyebrow">einrichtung abgeschlossen</div>
      <h1 className="onb-title" style={{ fontSize: 72, textAlign: 'center', maxWidth: '14ch' }}>
        <em>Jetzt</em> sprich<br />einfach los.
      </h1>
      <p className="onb-lede" style={{ textAlign: 'center', maxWidth: '42ch', margin: '0 auto 36px' }}>
        VOCA lebt ab jetzt in der Menüleiste. Die Pille am unteren Rand erscheint, sobald du aufnimmst.
      </p>
      <div style={{ display: 'flex', alignItems: 'center', gap: 14, marginBottom: 28 }}>
        <div className="kbd-display">
          {keys.map((k, i) => (
            <span key={i}>
              <span className="kbd-key">{k}</span>
              {i < keys.length - 1 && <span className="kbd-plus">+</span>}
            </span>
          ))}
        </div>
        <span className="v-meta">drücken · sprechen · loslassen</span>
      </div>
      <button className="v-btn accent" onClick={() => onFinish(settings)}>Loslegen</button>
    </div>
  )
}

/* ── Shortcut field used in onboarding (big kbd display, click to record) ── */

function KbdShortcutField({ value, onChange }: { value: string; onChange: (s: string) => void }) {
  const { recording, held, start, cancel, onKeyDown, onKeyUp, onBlur } = useShortcutCapture(onChange)
  const label = (k: string) => formatShortcutKey(k)
  const keys = recording
    ? (held.length > 0 ? sortShortcut(held).split('+') : [])
    : value.split('+')

  return (
    <>
      <button
        type="button"
        onClick={start}
        onKeyDown={onKeyDown}
        onKeyUp={onKeyUp}
        onBlur={onBlur}
        className="kbd-display"
        style={{
          cursor: recording ? 'default' : 'pointer',
          borderColor: recording ? 'var(--v-accent)' : undefined,
          background: recording ? 'color-mix(in srgb, var(--v-accent) 10%, transparent)' : undefined,
        }}
      >
        {keys.length > 0 ? keys.map((k, i) => (
          <span key={i}>
            <span className="kbd-key">{label(k)}</span>
            {i < keys.length - 1 && <span className="kbd-plus">+</span>}
          </span>
        )) : (
          <span style={{ fontFamily: 'var(--f-mono)', fontSize: 12, color: 'var(--v-ink-3)', padding: '4px 8px' }}>
            tasten drücken…
          </span>
        )}
      </button>
      {recording && (
        <button
          type="button"
          onClick={cancel}
          style={{ fontSize: 12, color: 'var(--v-ink-3)', background: 'none', border: 'none', cursor: 'pointer' }}
        >
          abbrechen
        </button>
      )}
    </>
  )
}

/* ---- Brand ---- */
function WelcomeLogoMark({ size = 88 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 100 100" fill="none" aria-hidden>
      <path d="M20 24 L50 76 L80 24" stroke="var(--v-ink)" strokeWidth="11" strokeLinecap="square" strokeLinejoin="miter"/>
      <circle cx="80" cy="24" r="8" fill="#C65441"/>
    </svg>
  )
}

/* ---- Icons ---- */
function ChevronIcon() {
  return <svg width={12} height={12} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round"><path d="M6 4l4 4-4 4"/></svg>
}
function BackIcon() {
  return <svg width={12} height={12} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" style={{ display: 'inline', marginRight: 4 }}><path d="M10 4l-4 4 4 4"/></svg>
}
function ExternalIcon() {
  return <svg width={13} height={13} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"><path d="M10 3h3v3M13 3l-6 6M7 4H3v9h9V9"/></svg>
}
function TrashIcon() {
  return <svg width={13} height={13} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"><path d="M3 5h10M6 5V3h4v2M5 5l1 9h4l1-9"/></svg>
}
function DownloadIcon() {
  return <svg width={13} height={13} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"><path d="M8 2v9M4 7l4 4 4-4M3 14h10"/></svg>
}
function MicSvg({ color = 'var(--v-ink-2)' }: { color?: string }) {
  return (
    <svg width={44} height={44} viewBox="0 0 16 16" fill="none" stroke={color} strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="6" y="2" width="4" height="8" rx="2"/>
      <path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2M5.5 14h5"/>
    </svg>
  )
}
