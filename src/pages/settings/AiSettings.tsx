import { useEffect, useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { SettingRow } from '../../components/settings/SettingRow'
import type { Settings, AIPrompt } from '../../types'

interface Props {
  settings: Settings
  onChange: (updated: Settings) => void
}

type Provider = Settings['aiEnhancement']['provider']

interface ProviderMeta {
  id: Provider
  label: string
  badge: 'free' | 'paid' | 'local'
  description: string
  models: string[]
  needsKey: boolean
  needsEndpoint: boolean
  keyPlaceholder?: string
  keyLink?: string
  defaultEndpoint?: string
}

const PROVIDERS: ProviderMeta[] = [
  {
    id: 'openai',
    label: 'OpenAI',
    badge: 'paid',
    description: 'GPT-4o and other OpenAI models',
    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: 'sk-...',
    keyLink: 'https://platform.openai.com/api-keys',
  },
  {
    id: 'anthropic',
    label: 'Anthropic',
    badge: 'paid',
    description: 'Claude family of models',
    models: ['claude-opus-4-7', 'claude-sonnet-4-6', 'claude-haiku-4-5-20251001'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: 'sk-ant-...',
    keyLink: 'https://console.anthropic.com/settings/keys',
  },
  {
    id: 'groq',
    label: 'Groq',
    badge: 'free',
    description: 'Very fast inference speeds',
    models: ['llama-3.3-70b-versatile', 'mixtral-8x7b-32768', 'gemma2-9b-it'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: 'gsk_...',
    keyLink: 'https://console.groq.com/keys',
  },
  {
    id: 'cerebras',
    label: 'Cerebras',
    badge: 'free',
    description: 'High-speed inference with generous free limits',
    models: ['llama3.1-8b', 'llama3.1-70b'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: 'csk-...',
    keyLink: 'https://cloud.cerebras.ai',
  },
  {
    id: 'gemini',
    label: 'Gemini',
    badge: 'free',
    description: "Google's Gemini models with a generous free tier",
    models: ['gemini-2.0-flash', 'gemini-1.5-pro', 'gemini-1.5-flash'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: 'AIza...',
    keyLink: 'https://aistudio.google.com/app/apikey',
  },
  {
    id: 'mistral',
    label: 'Mistral',
    badge: 'paid',
    description: 'Mistral AI models',
    models: ['mistral-large-latest', 'mistral-small-latest', 'open-mistral-7b'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: '...',
    keyLink: 'https://console.mistral.ai/api-keys',
  },
  {
    id: 'openrouter',
    label: 'OpenRouter',
    badge: 'paid',
    description: 'Access many providers through a single API key',
    models: ['openai/gpt-4o', 'anthropic/claude-sonnet-4-6', 'meta-llama/llama-3.3-70b-instruct'],
    needsKey: true,
    needsEndpoint: false,
    keyPlaceholder: 'sk-or-...',
    keyLink: 'https://openrouter.ai/keys',
  },
  {
    id: 'ollama',
    label: 'Ollama',
    badge: 'local',
    description: 'Run open-source models locally',
    models: ['llama3.2', 'llama3.1', 'mistral', 'phi4'],
    needsKey: false,
    needsEndpoint: true,
    defaultEndpoint: 'http://localhost:11434',
  },
  {
    id: 'custom',
    label: 'Custom',
    badge: 'paid',
    description: 'Any OpenAI-compatible API endpoint',
    models: [],
    needsKey: true,
    needsEndpoint: true,
    keyPlaceholder: '...',
  },
]


const BADGE_LABELS: Record<string, string> = {
  free: 'Free',
  paid: 'Paid',
  local: 'Local',
}

export function AiSettings({ settings, onChange }: Props) {
  const { t } = useTranslation()
  const ai = settings.aiEnhancement
  const [prompts, setPrompts] = useState<AIPrompt[]>([])
  const [modalPrompt, setModalPrompt] = useState<AIPrompt | null>(null)
  const [modalMode, setModalMode] = useState<'view' | 'edit' | 'add'>('view')
  const [draftName, setDraftName] = useState('')
  const [draftText, setDraftText] = useState('')
  const [apiKeyInput, setApiKeyInput] = useState('')
  const [hasKey, setHasKey] = useState(false)
  const [keySaving, setKeySaving] = useState(false)

  const meta = PROVIDERS.find((p) => p.id === ai.provider) ?? PROVIDERS[0]

  useEffect(() => {
    invoke<AIPrompt[]>('get_prompts').then(setPrompts).catch(() => {})
  }, [])

  // Load key status for current provider
  useEffect(() => {
    if (!meta.needsKey) { setHasKey(false); return }
    invoke<string | null>('get_ai_provider_key', { provider: ai.provider })
      .then((k) => setHasKey(k !== null))
      .catch(() => setHasKey(false))
    setApiKeyInput('')
  }, [ai.provider, meta.needsKey])

  function updateAi(partial: Partial<Settings['aiEnhancement']>) {
    onChange({ ...settings, aiEnhancement: { ...ai, ...partial } })
  }

  function handleProviderChange(provider: Provider) {
    const newMeta = PROVIDERS.find((p) => p.id === provider)!
    updateAi({
      provider,
      model: newMeta.models[0] ?? '',
      customEndpoint: newMeta.defaultEndpoint ?? '',
    })
  }

  async function saveKey() {
    if (!apiKeyInput.trim()) return
    setKeySaving(true)
    try {
      await invoke('save_ai_provider_key', { provider: ai.provider, value: apiKeyInput.trim() })
      setHasKey(true)
      setApiKeyInput('')
    } finally {
      setKeySaving(false)
    }
  }

  async function deleteKey() {
    await invoke('delete_ai_provider_key', { provider: ai.provider })
    setHasKey(false)
  }

  async function persistPrompts(updated: AIPrompt[]) {
    setPrompts(updated)
    await invoke('save_prompts', { prompts: updated }).catch(console.error)
  }

  function openModal(prompt: AIPrompt, mode: 'view' | 'edit') {
    setModalPrompt(prompt)
    setModalMode(mode)
    if (mode === 'edit') {
      setDraftName(prompt.name)
      setDraftText(prompt.prompt)
    }
  }

  function openAddModal() {
    setModalPrompt(null)
    setModalMode('add')
    setDraftName('')
    setDraftText('')
  }

  function closeModal() {
    setModalPrompt(null)
    setModalMode('view')
    setDraftName('')
    setDraftText('')
  }

  async function saveEdit() {
    if (!modalPrompt) return
    await persistPrompts(
      prompts.map((p) =>
        p.id === modalPrompt.id ? { ...p, name: draftName.trim(), prompt: draftText.trim() } : p
      )
    )
    closeModal()
  }

  async function deletePrompt(id: string) {
    await persistPrompts(prompts.filter((p) => p.id !== id))
    if (ai.activePromptId === id) updateAi({ activePromptId: 'default' })
    closeModal()
  }

  async function addPrompt() {
    if (!draftName.trim() || !draftText.trim()) return
    const newPrompt: AIPrompt = {
      id: crypto.randomUUID(),
      name: draftName.trim(),
      prompt: draftText.trim(),
      isDefault: false,
      createdAt: new Date().toISOString(),
    }
    await persistPrompts([...prompts, newPrompt])
    closeModal()
  }

  return (
    <div>
      <p className="page-eyebrow">einstellungen</p>
      <h1 className="page-title" style={{ marginBottom: 28 }}>{t('settings.nav.ai')}</h1>

      {/* Enable toggle */}
      <SettingRow label={t('settings.ai.enabled')} description="">
        <button
          role="switch"
          aria-checked={ai.enabled}
          onClick={() => updateAi({ enabled: !ai.enabled })}
          className={`v-switch${ai.enabled ? ' on' : ''}`}
        />
      </SettingRow>

      {ai.enabled && (
        <>
          {/* Skip short transcriptions */}
          <SettingRow
            label={t('settings.ai.skipShort')}
            description={t('settings.ai.skipShortDescription')}
          >
            <div className="flex items-center gap-3">
              <button
                role="switch"
                aria-checked={ai.skipShortTranscriptions}
                onClick={() => updateAi({ skipShortTranscriptions: !ai.skipShortTranscriptions })}
                className={`v-switch${ai.skipShortTranscriptions ? ' on' : ''}`}
              />
              {ai.skipShortTranscriptions && (
                <div className="flex items-center gap-1.5">
                  <span className="text-xs text-text-muted">{t('settings.ai.minWords')}</span>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                    <button className="v-btn ghost sm" style={{ padding: '3px 8px', lineHeight: 1 }} onClick={() => updateAi({ minWords: Math.max(1, ai.minWords - 1) })}>–</button>
                    <span style={{ fontFamily: 'var(--f-mono)', fontSize: 12, minWidth: 20, textAlign: 'center' }}>{ai.minWords}</span>
                    <button className="v-btn ghost sm" style={{ padding: '3px 8px', lineHeight: 1 }} onClick={() => updateAi({ minWords: Math.min(15, ai.minWords + 1) })}>+</button>
                  </div>
                  <span className="text-xs text-text-muted">{t('settings.ai.words')}</span>
                </div>
              )}
            </div>
          </SettingRow>

          {/* Provider list */}
          <div style={{ paddingBottom: 12, borderBottom: '1px solid var(--v-line-soft)', marginBottom: 4 }}>
            <p className="sec-head" style={{ paddingTop: 16 }}>{t('settings.ai.provider')}</p>
            <div className="prov-grid cols-3">
              {PROVIDERS.map((p) => (
                <button
                  key={p.id}
                  onClick={() => handleProviderChange(p.id)}
                  className={`prov-card${ai.provider === p.id ? ' is-active' : ''}`}
                >
                  <div className="prov-name">{p.label}</div>
                  <div className={`prov-meta${p.badge === 'free' ? ' free' : ''}`}>{BADGE_LABELS[p.badge]}</div>
                  <div className="prov-desc">{p.description}</div>
                </button>
              ))}
            </div>
          </div>

          {/* Endpoint (Ollama / Custom) */}
          {meta.needsEndpoint && (
            <SettingRow label={t('settings.ai.endpoint')} description="">
              <input
                value={ai.customEndpoint}
                onChange={(e) => updateAi({ customEndpoint: e.target.value })}
                placeholder={meta.defaultEndpoint ?? 'https://...'}
                className="w-64 text-xs bg-surface-raised border border-border rounded px-2 py-1.5 text-text placeholder:text-text-subtle focus:outline-none focus:border-border-hover"
              />
            </SettingRow>
          )}

          {/* Model */}
          {meta.models.length > 0 && (
            <SettingRow label={t('settings.ai.model')} description="">
              <select
                value={ai.model}
                onChange={(e) => updateAi({ model: e.target.value })}
                className="w-52 text-xs bg-surface-raised border border-border rounded px-2 py-1.5 text-text focus:outline-none focus:border-border-hover"
              >
                {meta.models.map((m) => <option key={m} value={m}>{m}</option>)}
              </select>
            </SettingRow>
          )}

          {/* API key (only for providers that need one) */}
          {meta.needsKey && (
            <SettingRow label={t('settings.ai.apiKey')} description={meta.label}>
              {hasKey ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs font-mono bg-surface px-2 py-1 rounded border border-border text-text-muted tracking-widest">
                    {'•'.repeat(16)}
                  </span>
                  <button onClick={deleteKey} className="text-xs text-text-muted hover:text-text transition-colors">
                    {t('common.delete')}
                  </button>
                </div>
              ) : (
                <div className="flex flex-col gap-1.5 w-52">
                  {meta.keyLink && (
                    <button
                      onClick={() => open(meta.keyLink!)}
                      className="flex items-center justify-center gap-1 px-2 py-1.5 text-xs border border-accent/40 text-accent rounded-lg hover:bg-accent-subtle transition-colors"
                    >
                      Get API key for {meta.label} <ExternalLinkIcon />
                    </button>
                  )}
                  <div className="flex items-center gap-2">
                    <input
                      type="password"
                      value={apiKeyInput}
                      onChange={(e) => setApiKeyInput(e.target.value)}
                      onKeyDown={(e) => e.key === 'Enter' && saveKey()}
                      placeholder={meta.keyPlaceholder ?? '...'}
                      className="flex-1 text-xs bg-surface-raised border border-border rounded px-2 py-1.5 text-text placeholder:text-text-subtle focus:outline-none focus:border-border-hover"
                    />
                    <button
                      onClick={saveKey}
                      disabled={!apiKeyInput.trim() || keySaving}
                      className="text-xs px-3 py-1.5 bg-accent text-accent-fg rounded hover:bg-accent-hover disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                    >
                      {t('common.save')}
                    </button>
                  </div>
                </div>
              )}
            </SettingRow>
          )}

          {meta.id === 'ollama' && (
            <p className="text-xs text-text-muted px-1 -mt-2 mb-3">
              {t('settings.ai.ollamaNote')}
            </p>
          )}

          {/* Prompt list */}
          <SettingRow label={t('settings.ai.activePrompt')} description={t('settings.ai.promptsDescription')}>
            <div className="w-full space-y-1.5">
              {prompts.map((prompt) => {
                const isActive = ai.activePromptId === prompt.id || (ai.activePromptId === '' && prompt.id === 'default')
                return (
                  <div
                    key={prompt.id}
                    className={`flex items-center gap-2.5 px-3 py-2 rounded-lg border-2 cursor-pointer transition-all ${
                      isActive ? 'border-accent bg-accent-subtle' : 'border-border bg-surface-raised hover:border-border-hover'
                    }`}
                    onClick={() => updateAi({ activePromptId: prompt.id })}
                  >
                    <span className={`w-3 h-3 rounded-full border-2 flex-shrink-0 ${
                      isActive ? 'border-accent bg-accent' : 'border-border'
                    }`} />
                    <span className="flex-1 text-sm text-text font-medium truncate">{prompt.name}</span>
                    <button
                      onClick={(e) => { e.stopPropagation(); openModal(prompt, prompt.isDefault ? 'view' : 'edit') }}
                      className="text-xs text-text-muted hover:text-text transition-colors flex-shrink-0 px-1.5 py-0.5 rounded hover:bg-border"
                    >
                      {prompt.isDefault ? '↗' : t('common.edit')}
                    </button>
                  </div>
                )
              })}

              <button
                onClick={openAddModal}
                className="w-full text-xs text-text-muted hover:text-text border border-dashed border-border hover:border-border-hover rounded-lg py-2 transition-colors"
              >
                + {t('settings.ai.addPrompt')}
              </button>
            </div>
          </SettingRow>

          {/* Prompt modal */}
          {(modalPrompt !== null || modalMode === 'add') && (
            <PromptModal
              mode={modalMode}
              prompt={modalPrompt}
              draftName={draftName}
              draftText={draftText}
              onNameChange={setDraftName}
              onTextChange={setDraftText}
              onSave={modalMode === 'add' ? addPrompt : saveEdit}
              onDelete={modalPrompt && !modalPrompt.isDefault ? () => deletePrompt(modalPrompt.id) : undefined}
              onClose={closeModal}
              t={t}
            />
          )}
        </>
      )}
    </div>
  )
}

interface PromptModalProps {
  mode: 'view' | 'edit' | 'add'
  prompt: AIPrompt | null
  draftName: string; draftText: string
  onNameChange: (v: string) => void; onTextChange: (v: string) => void
  onSave: () => void
  onDelete?: () => void
  onClose: () => void
  t: (key: string) => string
}

function PromptModal({ mode, prompt, draftName, draftText, onNameChange, onTextChange, onSave, onDelete, onClose, t }: PromptModalProps) {
  const overlayRef = useRef<HTMLDivElement>(null)

  function handleOverlayClick(e: { target: unknown }) {
    if (e.target === overlayRef.current) onClose()
  }

  return (
    <div
      ref={overlayRef}
      onClick={handleOverlayClick}
      style={{
        position: 'fixed', inset: 0, zIndex: 50,
        background: 'rgba(0,0,0,0.35)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        backdropFilter: 'blur(2px)',
      }}
    >
      <div style={{
        background: 'var(--v-surface)', borderRadius: 12,
        border: '1px solid var(--v-line)', boxShadow: 'var(--sh-overlay)',
        width: 480, maxWidth: 'calc(100vw - 48px)',
        padding: '24px', display: 'flex', flexDirection: 'column', gap: 16,
      }}>
        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span style={{ fontFamily: 'var(--f-display)', fontWeight: 800, fontSize: 13, letterSpacing: '0.1em', textTransform: 'uppercase', color: 'var(--v-ink)' }}>
            {mode === 'add' ? t('settings.ai.addPrompt') : mode === 'edit' ? t('common.edit') : prompt?.name}
          </span>
          <button onClick={onClose} style={{ color: 'var(--v-ink-3)', fontSize: 18, lineHeight: 1, padding: '2px 6px' }}>×</button>
        </div>

        {mode === 'view' ? (
          /* Read-only view */
          <div style={{
            background: 'var(--v-bg)', border: '1px solid var(--v-line-soft)',
            borderRadius: 8, padding: '14px 16px',
            fontFamily: 'var(--f-ui)', fontSize: 13, lineHeight: 1.65,
            color: 'var(--v-ink-2)', maxHeight: 320, overflowY: 'auto',
            whiteSpace: 'pre-wrap',
          }}>
            {prompt?.prompt}
          </div>
        ) : (
          /* Edit / Add form */
          <>
            {mode === 'add' && (
              <input
                value={draftName}
                onChange={(e) => onNameChange(e.target.value)}
                placeholder={t('settings.ai.promptName')}
                autoFocus
                style={{
                  width: '100%', padding: '8px 12px',
                  background: 'var(--v-bg)', border: '1px solid var(--v-line)',
                  borderRadius: 8, fontSize: 13, color: 'var(--v-ink)',
                  outline: 'none',
                }}
              />
            )}
            <textarea
              value={draftText}
              onChange={(e) => onTextChange(e.target.value)}
              placeholder={t('settings.ai.promptText')}
              rows={8}
              autoFocus={mode === 'edit'}
              style={{
                width: '100%', padding: '10px 12px',
                background: 'var(--v-bg)', border: '1px solid var(--v-line)',
                borderRadius: 8, fontSize: 12.5, lineHeight: 1.65, color: 'var(--v-ink)',
                outline: 'none', resize: 'vertical', fontFamily: 'var(--f-ui)',
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                {onDelete && (
                  <button
                    onClick={onDelete}
                    style={{ fontSize: 12, color: 'var(--v-danger)', padding: '6px 0' }}
                  >
                    {t('common.delete')}
                  </button>
                )}
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <button onClick={onClose} style={{ fontSize: 12, color: 'var(--v-ink-3)', padding: '6px 12px' }}>
                  {t('common.cancel')}
                </button>
                <button
                  onClick={onSave}
                  disabled={mode === 'add' ? (!draftName.trim() || !draftText.trim()) : !draftText.trim()}
                  style={{
                    fontSize: 12, padding: '6px 16px',
                    background: 'var(--v-ink)', color: 'var(--v-bg)',
                    borderRadius: 8, opacity: (mode === 'add' ? (!draftName.trim() || !draftText.trim()) : !draftText.trim()) ? 0.4 : 1,
                    cursor: (mode === 'add' ? (!draftName.trim() || !draftText.trim()) : !draftText.trim()) ? 'not-allowed' : 'pointer',
                  }}
                >
                  {t('common.save')}
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  )
}

function ExternalLinkIcon() {
  return <svg width={11} height={11} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" style={{ display: 'inline', marginLeft: 2 }}><path d="M10 3h3v3M13 3l-6 6M7 4H3v9h9V9"/></svg>
}
