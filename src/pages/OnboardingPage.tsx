import { useEffect, useState } from 'react'
import { Trans, useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-shell'
import { useShortcutCapture, sortShortcut } from '../hooks/useShortcutCapture'
import { DEFAULT_SHORTCUT, SUPPORTED_UI_LANGUAGES } from '../types'
import type { Settings, UiLanguage } from '../types'
import { formatShortcutKey } from '../shortcut/format'

const LANGUAGE_LABELS: Record<UiLanguage, string> = {
  de: 'Deutsch',
  en: 'English',
  es: 'Español',
  fr: 'Français',
  pt: 'Português',
  it: 'Italiano',
}

interface Props {
  settings: Settings
  onComplete: (updated: Settings) => Promise<void>
}

type CloudProvider = Settings['transcription']['cloudProvider']
type AiProvider = Settings['aiEnhancement']['provider']

const CLOUD_PROVIDERS: { id: CloudProvider; label: string; free: boolean; keyLink?: string; placeholder: string }[] = [
  { id: 'groq',       label: 'Groq',       free: true,  keyLink: 'https://console.groq.com/keys',                    placeholder: 'gsk_...' },
  { id: 'openai',     label: 'OpenAI',     free: false, keyLink: 'https://platform.openai.com/api-keys',             placeholder: 'sk-...' },
  { id: 'deepgram',   label: 'Deepgram',   free: true,  keyLink: 'https://console.deepgram.com',                     placeholder: 'deepgram key...' },
  { id: 'elevenlabs', label: 'ElevenLabs', free: false, keyLink: 'https://elevenlabs.io/app/settings/api-keys',      placeholder: 'el_...' },
  { id: 'gemini',     label: 'Gemini',     free: true,  keyLink: 'https://aistudio.google.com/app/apikey',           placeholder: 'AIza...' },
  { id: 'custom',     label: 'Custom',     free: false, placeholder: 'API key...' },
]

const AI_PROVIDERS: { id: AiProvider; label: string; free: boolean; keyLink?: string; placeholder: string }[] = [
  { id: 'anthropic', label: 'Anthropic',  free: false, keyLink: 'https://console.anthropic.com/settings/keys', placeholder: 'sk-ant-...' },
  { id: 'groq',      label: 'Groq',       free: true,  keyLink: 'https://console.groq.com/keys',               placeholder: 'gsk_...' },
  { id: 'openai',    label: 'OpenAI',     free: false, keyLink: 'https://platform.openai.com/api-keys',        placeholder: 'sk-...' },
  { id: 'gemini',    label: 'Gemini',     free: true,  keyLink: 'https://aistudio.google.com/app/apikey',      placeholder: 'AIza...' },
  { id: 'ollama',    label: 'Ollama',     free: true,  placeholder: '' },
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

type UseCaseId = 'dev' | 'pm' | 'content' | 'design' | 'business'
const USE_CASE_IDS: UseCaseId[] = ['dev', 'pm', 'content', 'design', 'business']

export function OnboardingPage({ settings, onComplete }: Props) {
  const { t } = useTranslation()
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

  // Persist a language change immediately without completing onboarding.
  // Piggybacks on the same onComplete (= save) pipe but keeps
  // onboardingCompleted unchanged so the flow doesn't collapse mid-step,
  // and the appStore update causes App.tsx to call i18n.changeLanguage
  // so the rest of the onboarding re-renders in the new language.
  async function persistLanguage(lang: UiLanguage) {
    const updated: Settings = { ...local, general: { ...local.general, language: lang } }
    setLocal(updated)
    await onComplete(updated)
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
          <button className="onb-skip" onClick={() => finish(local)}>{t('onboarding.footer.skip', 'skip')}</button>
        </div>
      )}

      {step === 'welcome'      && <StepWelcome settings={local} onLanguageChange={persistLanguage} onNext={next} />}
      {step === 'transcription' && <StepTranscription settings={local} onChange={setLocal} onNext={next} />}
      {step === 'shortcut'     && <StepShortcut settings={local} onChange={setLocal} onNext={next} />}
      {step === 'useCase'      && <StepUseCase onNext={next} />}
      {step === 'test'         && <StepTest onNext={next} />}
      {step === 'ai'           && <StepAi settings={local} onChange={setLocal} onNext={next} />}
      {step === 'done'         && <StepFinale settings={local} onFinish={finish} />}

      {showFoot && (
        <div className="onb-foot">
          <button className="onb-back" onClick={back}><BackIcon /> {t('onboarding.footer.back', 'zurück')}</button>
          <div className="onb-counter">{stepIndex} / {totalVisible - 1}</div>
        </div>
      )}
    </div>
  )
}

/* ── Welcome ─────────────────────────────────────────────────────────────── */

function StepWelcome({
  settings, onLanguageChange, onNext,
}: {
  settings: Settings
  onLanguageChange: (lang: UiLanguage) => void | Promise<void>
  onNext: () => void
}) {
  const { t } = useTranslation()
  return (
    <div className="onb-stage" style={{ justifyContent: 'center', alignItems: 'center' }}>
      <div className="onb-lang-picker">
        <select
          aria-label={t('onboarding.welcome.languageLabel', 'Sprache')}
          value={settings.general.language}
          onChange={(e) => onLanguageChange(e.target.value as UiLanguage)}
        >
          {SUPPORTED_UI_LANGUAGES.map((lang) => (
            <option key={lang} value={lang}>{LANGUAGE_LABELS[lang]}</option>
          ))}
        </select>
      </div>
      <div className="ember-mark" />
      <div style={{ marginBottom: 36 }}>
        <WelcomeLogoMark size={132} />
      </div>
      <div className="onb-eyebrow">{t('onboarding.welcome.eyebrow', 'willkommen bei')}</div>
      <h1 className="onb-title" style={{ fontSize: 88, marginBottom: 24 }}>
        <span className="v-wordmark" style={{ fontSize: 'inherit' }}>VOCA</span>
      </h1>
      <p className="onb-lede" style={{ fontSize: 17, maxWidth: '44ch' }}>
        {t('onboarding.welcome.lede', 'Sprich - und dein Text erscheint dort, wo du tippst.')}
      </p>
      <div style={{ display: 'flex', alignItems: 'center', gap: 20 }}>
        <button className="v-btn accent" onClick={onNext}>
          {t('onboarding.welcome.cta', 'Einrichten')} <ChevronIcon />
        </button>
        <span className="v-meta">{t('onboarding.welcome.meta', 'dauert keine 90 sekunden')}</span>
      </div>
    </div>
  )
}

/* ── Transcription ───────────────────────────────────────────────────────── */

function StepTranscription({
  settings, onChange, onNext,
}: { settings: Settings; onChange: (s: Settings) => void; onNext: () => void }) {
  const { t } = useTranslation()
  const mode = settings.transcription.mode
  const provider = settings.transcription.cloudProvider ?? 'groq'
  const provMeta = CLOUD_PROVIDERS.find((p) => p.id === provider) ?? CLOUD_PROVIDERS[0]
  const [apiKey, setApiKey] = useState('')
  const [showOffline, setShowOffline] = useState(mode === 'local')
  const [keyError, setKeyError] = useState<string | null>(null)
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
  async function saveKey() { if (apiKey) await invoke('save_transcription_key', { provider, value: apiKey.trim() }) }
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
  async function handleNext() {
    if (mode === 'cloud') {
      const err = validateKeyFormat(provider, apiKey)
      if (err) { setKeyError(err); return }
      setKeyError(null)
      await saveKey()
    }
    onNext()
  }

  const otherCloud = CLOUD_PROVIDERS.filter((p) => p.id !== 'groq')
  const providerLabel = (id: string) => CLOUD_PROVIDERS.find((p) => p.id === id)?.label ?? id

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">{t('onboarding.transcription.eyebrow', 'schritt 01 · transkription')}</div>
      <h1 className="onb-title">
        <Trans i18nKey="onboarding.transcription.title" components={{ em: <em /> }}>
          Cloud oder <em>lokal</em>?
        </Trans>
      </h1>
      <p className="onb-lede">
        {t('onboarding.transcription.lede', 'VOCA arbeitet mit schnellen Cloud-Diensten oder lokal auf deinem Rechner - vollständig offline. Die Entscheidung lässt sich jederzeit ändern.')}
      </p>

      {/* Provider grid */}
      <div className="prov-grid" style={{ marginBottom: 16 }}>
        <button
          className={`prov-card featured${!showOffline && provider === 'groq' ? ' is-active' : ''}`}
          onClick={() => selectProvider('groq')}
        >
          <span className="prov-badge">{t('onboarding.transcription.recommended', 'empfohlen')}</span>
          <div className="prov-name">Groq</div>
          <div className="prov-desc">{t('onboarding.transcription.providers.groq.desc')}</div>
        </button>
        {otherCloud.map((p) => (
          <button
            key={p.id}
            className={`prov-card${!showOffline && provider === p.id ? ' is-active' : ''}`}
            onClick={() => selectProvider(p.id)}
          >
            <div className="prov-name">{p.label}</div>
            <div className={`prov-meta${p.free ? ' free' : ''}`}>{t(`onboarding.transcription.providers.${p.id}.meta`)}</div>
            <div className="prov-desc">{t(`onboarding.transcription.providers.${p.id}.desc`)}</div>
          </button>
        ))}
        <button
          className={`prov-card${showOffline ? ' is-active' : ''}`}
          onClick={() => setOffline(!showOffline)}
        >
          <div className="prov-name">{t('onboarding.transcription.providers.offline.name', 'Offline')}</div>
          <div className="prov-meta free">{t('onboarding.transcription.providers.offline.meta', 'whisper.cpp · lokal')}</div>
          <div className="prov-desc">{t('onboarding.transcription.providers.offline.desc')}</div>
        </button>
      </div>

      {/* API key or offline models */}
      {!showOffline && (
        <>
          <div style={{ display: 'flex', gap: 10, marginBottom: keyError ? 8 : 20 }}>
            <input
              className="v-input"
              type="password"
              value={apiKey}
              onChange={(e) => { setApiKey(e.target.value); if (keyError) setKeyError(null) }}
              placeholder={provMeta.placeholder}
              style={{ flex: 1, maxWidth: 360 }}
            />
            {provMeta.keyLink && (
              <button className="v-btn ghost" onClick={() => open(provMeta.keyLink!)}>
                <ExternalIcon /> {t('onboarding.transcription.getKey', 'Schlüssel erstellen')}
              </button>
            )}
          </div>
          {keyError && (
            <div className="onb-err" style={{ marginBottom: 20 }}>
              {t('errors.apiKeyFormat', { provider: providerLabel(provider) })}
            </div>
          )}
        </>
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
                <span className="model-desc">{t(`onboarding.transcription.models.${value}`)}</span>
                <div className="model-action">
                  {isDownloading && progress ? (
                    <>
                      <span className="v-meta">{Math.round((progress.downloaded_bytes / progress.total_bytes) * 100)}%</span>
                      <div className="pbar-wrap"><span className="pbar-fill" style={{ width: `${(progress.downloaded_bytes / progress.total_bytes) * 100}%` }} /></div>
                      <button className="v-btn ghost sm" onClick={(e) => { e.stopPropagation(); handleCancel() }}>{t('onboarding.transcription.cancel', 'abbrechen')}</button>
                    </>
                  ) : isDownloaded ? (
                    <>
                      <span className="model-status-dot" />
                      <span className="v-meta">{t('onboarding.transcription.ready', 'bereit')}</span>
                      <button className="v-btn ghost sm" onClick={(e) => { e.stopPropagation(); handleDelete(value) }}><TrashIcon /></button>
                    </>
                  ) : (
                    <button className="v-btn ghost sm" disabled={downloading !== null} onClick={(e) => { e.stopPropagation(); handleDownload(value) }}>
                      <DownloadIcon /> {t('onboarding.transcription.download', 'laden')}
                    </button>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      <button className="v-btn accent" onClick={handleNext}>{t('onboarding.transcription.next', 'Weiter')} <ChevronIcon /></button>
    </div>
  )
}

function validateKeyFormat(provider: string, key: string): 'format' | null {
  const trimmed = key.trim()
  if (!trimmed) return null
  const expected =
    provider === 'anthropic' ? 'sk-ant-' :
    provider === 'groq'      ? 'gsk_'    :
    provider === 'openai'    ? 'sk-'     :
    provider === 'gemini'    ? 'AIza'    :
    null
  if (expected && !trimmed.startsWith(expected)) return 'format'
  return null
}

/* ── Shortcut ────────────────────────────────────────────────────────────── */

function StepShortcut({
  settings, onChange, onNext,
}: { settings: Settings; onChange: (s: Settings) => void; onNext: () => void }) {
  const { t } = useTranslation()
  const currentKey = settings.shortcuts.key || DEFAULT_SHORTCUT

  async function handleKeyChange(newShortcut: string) {
    try { await invoke('register_shortcut', { newShortcut }) } catch { /* conflict handled by backend */ }
    onChange({ ...settings, shortcuts: { key: newShortcut } })
  }

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">{t('onboarding.shortcut.eyebrow', 'schritt 02 · tastenkürzel')}</div>
      <h1 className="onb-title">
        <Trans i18nKey="onboarding.shortcut.title" components={{ em: <em /> }}>
          Eine Taste. <em>Ein Gedanke.</em>
        </Trans>
      </h1>
      <p className="onb-lede">
        {t('onboarding.shortcut.lede', 'Halte die Tastenkombination gedrückt, solange du sprichst. Sobald du loslässt, wird transkribiert. Zum Neubelegen einfach auf das Feld klicken.')}
      </p>

      <div style={{ display: 'flex', alignItems: 'center', gap: 20, marginBottom: 24 }}>
        <KbdShortcutField value={currentKey} onChange={handleKeyChange} />
      </div>

      <div style={{ display: 'flex', gap: 18, alignItems: 'flex-start', marginBottom: 28 }}>
        <div style={{ width: 3, background: 'var(--v-accent)', alignSelf: 'stretch', borderRadius: 2, flexShrink: 0 }} />
        <div>
          <div className="v-label" style={{ marginBottom: 4 }}>{t('onboarding.shortcut.tipLabel', 'tipp')}</div>
          <p style={{ fontSize: 13.5, color: 'var(--v-ink-2)', maxWidth: '50ch', lineHeight: 1.6, fontFamily: 'var(--f-ui)' }}>
            {t('onboarding.shortcut.tipBody', 'Gedrückt halten fühlt sich natürlicher an als ein An-Aus-Schalter - du hast jederzeit Kontrolle darüber, was aufgenommen wird.')}
          </p>
        </div>
      </div>

      <button className="v-btn accent" onClick={onNext}>{t('onboarding.shortcut.next', 'Weiter')} <ChevronIcon /></button>
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
      <div className="onb-eyebrow">{t('onboarding.useCase.eyebrow', 'schritt 03 · fachbereich')}</div>
      <h1 className="onb-title">
        <Trans i18nKey="onboarding.useCase.title" components={{ em: <em /> }}>
          Worüber sprichst du am <em>meisten</em>?
        </Trans>
      </h1>
      <p className="onb-lede">
        {t('onboarding.useCase.lede', 'Damit Fachbegriffe, Tool-Namen und Abkürzungen sauber erkannt werden, hilft VOCA ein wenig Kontext. Wähl einfach, was auf dich zutrifft - mehrere gehen. Jederzeit änderbar.')}
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
  const { t } = useTranslation()
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

  // First-meeting ceremony: unlock the shortcut, reveal the pill, and send
  // the localised bubble text along so the pill window doesn't need its own
  // i18n state. Only runs once per onboarding — the effect's empty deps
  // mean re-entering the step (e.g. via back) would re-fire, but the pill's
  // animations are one-shot by design and StepTest is rarely revisited.
  useEffect(() => {
    (async () => {
      try {
        await invoke('unlock_recording')
        await invoke('show_pill')
        // Give the OS compositor a beat to actually paint the now-visible
        // pill window. Emitting the reveal event before this gap often
        // resulted in the CSS animation running entirely inside the hidden
        // webview, leaving the user with a silently-arrived pill. 120 ms is
        // enough on the systems we've tested without feeling like a lag.
        await new Promise((resolve) => setTimeout(resolve, 120))
        await emit('pill-animate-reveal', {
          bubble: t('onboarding.pill.bubble', { defaultValue: 'Hey, hier unten bin ich.' }),
        })
      } catch (e) {
        console.error('failed to unlock pill for test step:', e)
      }
    })()
  }, [t])

  const trimmed = text.trim()
  const wordCount = trimmed ? trimmed.split(/\s+/).filter(Boolean).length : 0

  let statusLabel = t('onboarding.test.status.idle', 'Bereit, wenn du bereit bist.')
  let accentMic = false
  if (appState === 'recording') {
    statusLabel = t('onboarding.test.status.recording', 'Läuft. Halt einfach weiter gedrückt.')
    accentMic = true
  } else if (appState === 'processing') {
    statusLabel = t('onboarding.test.status.processing', 'Hole gerade deinen Text ab …')
    accentMic = true
  } else if (appState === 'inserting') {
    statusLabel = t('onboarding.test.status.inserting', 'Füge ein …')
    accentMic = true
  }

  return (
    <div className="onb-stage" style={{ textAlign: 'center', alignItems: 'center' }}>
      <div className="onb-eyebrow" style={{ alignSelf: 'center' }}>{t('onboarding.test.eyebrow', 'schritt 04 · dein erster versuch')}</div>
      <h1 className="onb-title" style={{ textAlign: 'center', maxWidth: '20ch' }}>
        <Trans i18nKey="onboarding.test.title" components={{ em: <em /> }}>
          {'Probier\'s '}<em>einmal</em> aus.
        </Trans>
      </h1>
      <p className="onb-lede" style={{ textAlign: 'center', maxWidth: '44ch' }}>
        {t('onboarding.test.lede', 'Halte deine Tastenkombination gedrückt und sag einen beliebigen Satz. VOCA begleitet dich Schritt für Schritt.')}
      </p>

      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14, marginTop: 12, width: '100%', maxWidth: 560 }}>
        <div className="mic-disc" style={accentMic ? { borderColor: 'var(--v-accent)' } : undefined}>
          <MicSvg color={accentMic ? 'var(--v-accent)' : undefined} />
        </div>
        <p className="v-meta">{statusLabel}</p>

        <div
          aria-label={t('onboarding.test.transcriptLabel', 'transkript')}
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
            <div className="rc-stat"><span className="k">{t('onboarding.test.wordsLabel', 'wörter')}</span><span className="v">{wordCount}</span></div>
            <div className="rc-stat"><span className="k">{t('onboarding.test.charsLabel', 'zeichen')}</span><span className="v">{text.length}</span></div>
          </div>
        )}

        <button
          className={trimmed ? 'v-btn accent' : 'v-btn ghost'}
          onClick={onNext}
          style={{ marginTop: 4 }}
        >
          {trimmed ? t('onboarding.test.continue', 'Perfekt, weiter') : t('onboarding.test.skip', 'Überspringen')} <ChevronIcon />
        </button>
      </div>
    </div>
  )
}

/* ── AI Enhancement ──────────────────────────────────────────────────────── */

function StepAi({
  settings, onChange, onNext,
}: { settings: Settings; onChange: (s: Settings) => void; onNext: () => void }) {
  const { t } = useTranslation()
  const provider = settings.aiEnhancement.provider
  const provMeta = AI_PROVIDERS.find((p) => p.id === provider) ?? AI_PROVIDERS[0]
  const [apiKey, setApiKey] = useState('')
  const [keyError, setKeyError] = useState<string | null>(null)
  const transcProv = settings.transcription.cloudProvider
  const canReuse = (provider as string) === (transcProv as string) && provider !== 'ollama'

  useEffect(() => {
    if (provider === 'ollama') return
    invoke<string | null>('get_ai_provider_key', { provider }).then((k) => setApiKey(k ?? '')).catch(() => {})
  }, [provider])

  async function handleReuse() {
    const k = await invoke<string | null>('get_transcription_key', { provider: transcProv })
    if (k) { setApiKey(k); setKeyError(null) }
  }

  async function handleEnable() {
    if (provider !== 'ollama') {
      const err = validateKeyFormat(provider, apiKey)
      if (err) { setKeyError(err); return }
      setKeyError(null)
      if (apiKey) await invoke('save_ai_provider_key', { provider, value: apiKey.trim() })
    }
    await onNext()
    onChange({ ...settings, aiEnhancement: { ...settings.aiEnhancement, enabled: true } })
  }

  const others = AI_PROVIDERS.filter((p) => p.id !== 'groq')
  const providerLabel = AI_PROVIDERS.find((p) => p.id === provider)?.label ?? provider

  return (
    <div className="onb-stage">
      <div className="onb-eyebrow">{t('onboarding.aiEnhancement.eyebrow', 'schritt 05 · optional')}</div>
      <h1 className="onb-title">
        <Trans i18nKey="onboarding.aiEnhancement.title" components={{ em: <em /> }}>
          Unbearbeitet oder in <em>Form</em> gebracht?
        </Trans>
      </h1>
      <p className="onb-lede">
        {t('onboarding.aiEnhancement.lede', 'Auf Wunsch lässt VOCA dein Transkript durch ein Sprachmodell laufen. Es entfernt Füllwörter, korrigiert Grammatik und glättet den Satzbau. Deine Stimme bleibt erhalten. Jederzeit ein- oder ausschaltbar.')}
      </p>

      <div className="prov-grid" style={{ marginBottom: 16 }}>
        <button
          className={`prov-card featured${provider === 'groq' ? ' is-active' : ''}`}
          onClick={() => onChange({ ...settings, aiEnhancement: { ...settings.aiEnhancement, provider: 'groq' } })}
        >
          <span className="prov-badge">{t('onboarding.aiEnhancement.recommended', 'empfohlen')}</span>
          <div className="prov-name">Groq</div>
          <div className="prov-desc">{t('onboarding.aiEnhancement.providers.groq.desc')}</div>
        </button>
        {others.map((p) => (
          <button
            key={p.id}
            className={`prov-card${provider === p.id ? ' is-active' : ''}`}
            onClick={() => onChange({ ...settings, aiEnhancement: { ...settings.aiEnhancement, provider: p.id } })}
          >
            <div className="prov-name">{p.label}</div>
            <div className={`prov-meta${p.free ? ' free' : ''}`}>{t(`onboarding.aiEnhancement.providers.${p.id}.meta`)}</div>
            <div className="prov-desc">{t(`onboarding.aiEnhancement.providers.${p.id}.desc`)}</div>
          </button>
        ))}
      </div>

      {provider !== 'ollama' && (
        <>
          <div style={{ display: 'flex', gap: 10, marginBottom: canReuse && !apiKey ? 4 : (keyError ? 8 : 20) }}>
            <input
              className="v-input"
              type="password"
              value={apiKey}
              onChange={(e) => { setApiKey(e.target.value); if (keyError) setKeyError(null) }}
              placeholder={provMeta.placeholder}
              style={{ flex: 1, maxWidth: 360 }}
            />
            {canReuse && !apiKey ? (
              <button className="v-btn ghost" onClick={handleReuse}>{t('onboarding.aiEnhancement.reuseBtn', 'Gleichen Schlüssel nutzen')}</button>
            ) : provMeta.keyLink ? (
              <button className="v-btn ghost" onClick={() => open(provMeta.keyLink!)}>
                <ExternalIcon /> {t('onboarding.aiEnhancement.getKey', 'Schlüssel erstellen')}
              </button>
            ) : null}
          </div>
          {canReuse && !apiKey && (
            <p className="onb-reuse-hint" style={{ marginBottom: 20 }}>
              {t('onboarding.aiEnhancement.reuseHint', { provider: providerLabel, defaultValue: 'Du nutzt {{provider}} bereits für die Transkription. Der bestehende Schlüssel kann hier übernommen werden.' })}
            </p>
          )}
          {keyError && (
            <div className="onb-err" style={{ marginBottom: 20 }}>
              {t('errors.apiKeyFormat', { provider: providerLabel })}
            </div>
          )}
        </>
      )}
      {provider === 'ollama' && (
        <p style={{ fontSize: 12, color: 'var(--v-ink-3)', marginBottom: 20, fontFamily: 'var(--f-ui)' }}>
          {t('onboarding.aiEnhancement.ollamaHint', 'Ollama muss lokal laufen und das gewählte Modell heruntergeladen sein.')}
        </p>
      )}

      <div style={{ display: 'flex', gap: 12 }}>
        <button className="v-btn accent" onClick={handleEnable}>{t('onboarding.aiEnhancement.enable', 'Aktivieren')} <ChevronIcon /></button>
        <button className="v-btn ghost" onClick={onNext}>{t('onboarding.aiEnhancement.later', 'Später entscheiden')}</button>
      </div>
    </div>
  )
}

/* ── Finale ──────────────────────────────────────────────────────────────── */

function StepFinale({ settings, onFinish }: { settings: Settings; onFinish: (s: Settings) => Promise<void> }) {
  const { t } = useTranslation()
  const keys = (settings.shortcuts.key || DEFAULT_SHORTCUT).split('+').map((k) => formatShortcutKey(k))
  return (
    <div className="onb-stage" style={{ alignItems: 'center', textAlign: 'center', justifyContent: 'center' }}>
      <div className="onb-eyebrow">{t('onboarding.finale.eyebrow', 'einrichtung abgeschlossen')}</div>
      <h1 className="onb-title" style={{ fontSize: 72, textAlign: 'center', maxWidth: '14ch' }}>
        <Trans i18nKey="onboarding.finale.title" components={{ em: <em /> }}>
          Alles <em>bereit.</em>
        </Trans>
      </h1>
      <p className="onb-lede" style={{ textAlign: 'center', maxWidth: '48ch', margin: '0 auto 36px' }}>
        {t('onboarding.finale.lede', 'VOCA läuft ab jetzt leise in der Menüleiste. Die Pille am unteren Rand taucht auf, sobald du aufnimmst. Alles Weitere findest du in den Einstellungen.')}
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
        <span className="v-meta">{t('onboarding.finale.shortcutMeta', 'drücken · sprechen · loslassen')}</span>
      </div>
      <button className="v-btn accent" onClick={() => onFinish(settings)}>{t('onboarding.finale.cta', 'Loslegen')}</button>
    </div>
  )
}

/* ── Shortcut field used in onboarding (big kbd display, click to record) ── */

function KbdShortcutField({ value, onChange }: { value: string; onChange: (s: string) => void }) {
  const { t } = useTranslation()
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
            {t('onboarding.shortcut.recordBtn.idle', 'tasten drücken…')}
          </span>
        )}
      </button>
      {recording && (
        <button
          type="button"
          onClick={cancel}
          style={{ fontSize: 12, color: 'var(--v-ink-3)', background: 'none', border: 'none', cursor: 'pointer' }}
        >
          {t('onboarding.shortcut.cancel', 'abbrechen')}
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
