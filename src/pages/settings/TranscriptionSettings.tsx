import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-shell'
import { SettingRow } from '../../components/settings/SettingRow'
import { TRANSCRIPTION_LANGUAGES } from '../../types'
import type { Settings, TranscriptionLanguage } from '../../types'

interface Props {
  settings: Settings
  onChange: (updated: Settings) => void
}

type ModelSize = 'tiny' | 'base' | 'small' | 'medium'
type CloudProvider = Settings['transcription']['cloudProvider']

interface CloudProviderMeta {
  id: CloudProvider
  label: string
  badge: 'free' | 'paid'
  description: string
  models: string[]
  needsKey: boolean
  keyPlaceholder?: string
  keyLink?: string
  needsEndpoint: boolean
}

const CLOUD_PROVIDERS: CloudProviderMeta[] = [
  {
    id: 'openai',
    label: 'OpenAI',
    badge: 'paid',
    description: 'Whisper – high accuracy, many languages',
    models: ['whisper-1'],
    needsKey: true,
    keyPlaceholder: 'sk-...',
    keyLink: 'https://platform.openai.com/api-keys',
    needsEndpoint: false,
  },
  {
    id: 'groq',
    label: 'Groq',
    badge: 'free',
    description: 'Whisper large-v3-turbo – very fast inference',
    models: ['whisper-large-v3-turbo', 'whisper-large-v3', 'distil-whisper-large-v3-en'],
    needsKey: true,
    keyPlaceholder: 'gsk_...',
    keyLink: 'https://console.groq.com/keys',
    needsEndpoint: false,
  },
  {
    id: 'deepgram',
    label: 'Deepgram',
    badge: 'free',
    description: 'Nova-3 – fast with smart formatting',
    models: ['nova-3', 'nova-2', 'enhanced'],
    needsKey: true,
    keyPlaceholder: 'deepgram key...',
    keyLink: 'https://console.deepgram.com',
    needsEndpoint: false,
  },
  {
    id: 'elevenlabs',
    label: 'ElevenLabs',
    badge: 'paid',
    description: 'Scribe – high quality with timestamps',
    models: ['scribe_v1'],
    needsKey: true,
    keyPlaceholder: 'el_...',
    keyLink: 'https://elevenlabs.io/app/settings/api-keys',
    needsEndpoint: false,
  },
  {
    id: 'gemini',
    label: 'Gemini',
    badge: 'free',
    description: "Google Gemini – multimodal transcription",
    models: ['gemini-2.0-flash', 'gemini-1.5-flash', 'gemini-1.5-pro'],
    needsKey: true,
    keyPlaceholder: 'AIza...',
    keyLink: 'https://aistudio.google.com/app/apikey',
    needsEndpoint: false,
  },
  {
    id: 'custom',
    label: 'Custom',
    badge: 'paid',
    description: 'OpenAI-compatible endpoint',
    models: [],
    needsKey: true,
    keyPlaceholder: 'API key...',
    needsEndpoint: true,
  },
]


interface ModelStatus {
  downloaded: boolean
  size_bytes: number
}

interface DownloadProgress {
  size: string
  downloaded_bytes: number
  total_bytes: number
}

const MODEL_SIZES: { value: ModelSize; label: string; approxMb: number }[] = [
  { value: 'tiny', label: 'Tiny', approxMb: 75 },
  { value: 'base', label: 'Base', approxMb: 142 },
  { value: 'small', label: 'Small', approxMb: 466 },
  { value: 'medium', label: 'Medium', approxMb: 1500 },
]

export function TranscriptionSettings({ settings, onChange }: Props) {
  const { t } = useTranslation()
  const isLocal = settings.transcription.mode === 'local'
  const selectedSize = settings.transcription.localModelSize
  const selectedProvider = settings.transcription.cloudProvider ?? 'openai'
  const providerMeta = CLOUD_PROVIDERS.find((p) => p.id === selectedProvider)!

  const [modelStatuses, setModelStatuses] = useState<Record<ModelSize, ModelStatus | null>>({
    tiny: null, base: null, small: null, medium: null,
  })
  const [downloading, setDownloading] = useState<ModelSize | null>(null)
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null)
  const [providerKey, setProviderKey] = useState('')
  const [keyDirty, setKeyDirty] = useState(false)

  useEffect(() => {
    MODEL_SIZES.forEach(({ value }) => {
      invoke<ModelStatus>('get_model_status', { size: value })
        .then((status) => setModelStatuses((prev) => ({ ...prev, [value]: status })))
        .catch(() => {})
    })
  }, [])

  useEffect(() => {
    setProviderKey('')
    setKeyDirty(false)
    invoke<string | null>('get_transcription_key', { provider: selectedProvider })
      .then((k) => setProviderKey(k ?? ''))
      .catch(() => {})
  }, [selectedProvider])

  useEffect(() => {
    const unlisten = listen<DownloadProgress>('model-download-progress', (event) => {
      setDownloadProgress(event.payload)
    })
    return () => { unlisten.then((fn) => fn()) }
  }, [])

  function handleProviderChange(provider: CloudProvider) {
    onChange({
      ...settings,
      transcription: { ...settings.transcription, cloudProvider: provider, cloudModel: '' },
    })
  }

  async function handleSaveKey() {
    if (!keyDirty) return
    await invoke('save_transcription_key', { provider: selectedProvider, value: providerKey })
    setKeyDirty(false)
  }

  function handleModeChange(mode: 'cloud' | 'local') {
    onChange({ ...settings, transcription: { ...settings.transcription, mode } })
  }

  function handleLanguageChange(language: TranscriptionLanguage) {
    onChange({ ...settings, transcription: { ...settings.transcription, language } })
  }

  function handleModelSizeChange(size: ModelSize) {
    onChange({ ...settings, transcription: { ...settings.transcription, localModelSize: size } })
  }

  async function handleDownload(size: ModelSize) {
    setDownloading(size)
    setDownloadProgress(null)
    try {
      await invoke('download_model', { size })
      const status = await invoke<ModelStatus>('get_model_status', { size })
      setModelStatuses((prev) => ({ ...prev, [size]: status }))
    } catch (e) {
      console.error('Download failed:', e)
    } finally {
      setDownloading(null)
      setDownloadProgress(null)
    }
  }

  async function handleDeleteModel(size: ModelSize) {
    try {
      await invoke('delete_model', { size })
      setModelStatuses((prev) => ({ ...prev, [size]: { downloaded: false, size_bytes: 0 } }))
    } catch (e) {
      console.error('Delete failed:', e)
    }
  }

  async function handleCancelDownload() {
    try {
      await invoke('cancel_model_download')
    } catch (e) {
      console.error('Cancel failed:', e)
    }
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(0)} MB`
  }

  return (
    <div>
      <p className="page-eyebrow">einstellungen</p>
      <h1 className="page-title" style={{ marginBottom: 28 }}>{t('settings.nav.transcription')}</h1>

      <SettingRow label={t('settings.transcription.method')} description="">
        <div className="v-seg">
          {(['cloud', 'local'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => handleModeChange(mode)}
              className={settings.transcription.mode === mode ? 'is-active' : ''}
            >
              {t(`settings.transcription.mode.${mode}`)}
            </button>
          ))}
        </div>
      </SettingRow>

      <SettingRow
        label={t('settings.transcription.language')}
        description={t('settings.transcription.languageDescription')}
      >
        <select
          value={settings.transcription.language ?? 'auto'}
          onChange={(e) => handleLanguageChange(e.target.value as TranscriptionLanguage)}
          className="w-48 px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text focus:outline-none focus:border-accent"
        >
          {TRANSCRIPTION_LANGUAGES.map((lang) => (
            <option key={lang} value={lang}>
              {t(`settings.transcription.languageOption.${lang}`)}
            </option>
          ))}
        </select>
      </SettingRow>

      {!isLocal && (
        <>
          <div style={{ paddingBottom: 12, borderBottom: '1px solid var(--v-line-soft)', marginBottom: 4 }}>
            <p className="sec-head" style={{ paddingTop: 16 }}>{t('settings.transcription.cloudProvider')}</p>
            <div className="prov-grid cols-3">
              {CLOUD_PROVIDERS.map((p) => (
                <button
                  key={p.id}
                  onClick={() => handleProviderChange(p.id)}
                  className={`prov-card${selectedProvider === p.id ? ' is-active' : ''}`}
                >
                  <div className="prov-name">{p.label}</div>
                  <div className={`prov-meta${p.badge === 'free' ? ' free' : ''}`}>{p.badge}</div>
                  <div className="prov-desc">{p.description}</div>
                </button>
              ))}
            </div>
          </div>

          {providerMeta.models.length > 0 && (
            <SettingRow label={t('settings.transcription.cloudModel')} description="">
              <select
                value={settings.transcription.cloudModel || providerMeta.models[0]}
                onChange={(e) =>
                  onChange({
                    ...settings,
                    transcription: { ...settings.transcription, cloudModel: e.target.value },
                  })
                }
                className="w-48 px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text focus:outline-none focus:border-accent"
              >
                {providerMeta.models.map((m) => <option key={m} value={m}>{m}</option>)}
              </select>
            </SettingRow>
          )}

          {providerMeta.needsEndpoint && (
            <SettingRow label={t('settings.transcription.cloudCustomEndpoint')} description="">
              <input
                value={settings.transcription.cloudCustomEndpoint}
                onChange={(e) =>
                  onChange({
                    ...settings,
                    transcription: { ...settings.transcription, cloudCustomEndpoint: e.target.value },
                  })
                }
                placeholder="https://..."
                className="w-full max-w-xs px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
              />
            </SettingRow>
          )}

          {providerMeta.needsKey && (
            <SettingRow label={t('settings.transcription.apiKey')} description="">
              <div className="flex flex-col gap-1.5 w-full max-w-xs">
                {providerMeta.keyLink && (
                  <button
                    onClick={() => open(providerMeta.keyLink!)}
                    className="flex items-center justify-center gap-1 px-2.5 py-1.5 text-xs border border-accent/40 text-accent rounded-lg hover:bg-accent-subtle transition-colors"
                  >
                    Get API key for {providerMeta.label} <ExternalLinkIcon />
                  </button>
                )}
                <input
                  type="password"
                  value={providerKey}
                  onChange={(e) => { setProviderKey(e.target.value); setKeyDirty(true) }}
                  onBlur={handleSaveKey}
                  placeholder={providerMeta.keyPlaceholder ?? ''}
                  className="flex-1 px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
                />
              </div>
            </SettingRow>
          )}
        </>
      )}

      {isLocal && (
        <SettingRow
          label={t('settings.transcription.localModel')}
          description={t('settings.transcription.localModelDescription')}
        >
          <div className="space-y-3">
            <div className="space-y-1">
              {MODEL_SIZES.map(({ value, label, approxMb }) => {
                const status = modelStatuses[value]
                const isSelected = selectedSize === value
                const isDownloaded = status?.downloaded ?? false
                const isThisDownloading = downloading === value

                return (
                  <div
                    key={value}
                    className={`flex items-center justify-between px-3 py-2 rounded-lg border transition-colors cursor-pointer ${
                      isSelected
                        ? 'border-accent bg-accent-subtle'
                        : 'border-border hover:border-border-hover'
                    }`}
                    onClick={() => handleModelSizeChange(value)}
                  >
                    <div className="flex items-center gap-2">
                      <span className={`w-3 h-3 rounded-full border-2 flex-shrink-0 ${
                        isSelected ? 'border-accent bg-accent' : 'border-border'
                      }`} />
                      <span className="text-sm text-text font-medium">{label}</span>
                      <span className="text-xs text-text-muted">~{approxMb < 1000 ? `${approxMb} MB` : `${(approxMb / 1000).toFixed(1)} GB`}</span>
                    </div>

                    <div className="flex items-center gap-2">
                      {isThisDownloading && downloadProgress ? (
                        <div className="flex items-center gap-2">
                          <div className="w-24 h-1.5 bg-surface rounded-full overflow-hidden">
                            <div
                              className="h-full bg-accent transition-all duration-100 rounded-full"
                              style={{
                                width: downloadProgress.total_bytes > 0
                                  ? `${(downloadProgress.downloaded_bytes / downloadProgress.total_bytes) * 100}%`
                                  : '0%'
                              }}
                            />
                          </div>
                          <span className="text-xs text-text-muted">
                            {formatBytes(downloadProgress.downloaded_bytes)}
                          </span>
                          <button
                            onClick={(e) => { e.stopPropagation(); handleCancelDownload() }}
                            className="text-xs px-2 py-1 border border-border rounded hover:border-red-500 hover:border-red-500/50 text-text-muted hover:text-red-500 transition-colors"
                          >
                            {t('settings.transcription.cancelDownload')}
                          </button>
                        </div>
                      ) : isDownloaded ? (
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-green-500">✓ {t('settings.transcription.downloaded')}</span>
                          <button
                            onClick={(e) => { e.stopPropagation(); handleDeleteModel(value) }}
                            className="text-xs px-2 py-1 border border-border rounded hover:border-red-500/50 text-text-muted hover:text-red-500 transition-colors"
                          >
                            {t('settings.transcription.deleteModel')}
                          </button>
                        </div>
                      ) : (
                        <button
                          onClick={(e) => { e.stopPropagation(); handleDownload(value) }}
                          disabled={downloading !== null}
                          className="text-xs px-2 py-1 border border-border rounded hover:border-border-hover text-text-muted hover:text-text disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                        >
                          {t('settings.transcription.download')}
                        </button>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        </SettingRow>
      )}

    </div>
  )
}

function ExternalLinkIcon() {
  return <svg width={11} height={11} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" style={{ display: 'inline', marginLeft: 2 }}><path d="M10 3h3v3M13 3l-6 6M7 4H3v9h9V9"/></svg>
}
