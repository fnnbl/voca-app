import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import type { FillerEntry, Settings } from '../../types'

interface Props {
  settings: Settings
  onChange: (updated: Settings) => void
}

export function FillersSettings({ settings, onChange }: Props) {
  const { t } = useTranslation()
  const [entries, setEntries] = useState<FillerEntry[]>([])
  const [input, setInput] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  const enabled = settings.transcription?.removeFillerWords ?? false

  useEffect(() => {
    invoke<FillerEntry[]>('get_fillers').then(setEntries).catch(console.error)
  }, [])

  async function persist(updated: FillerEntry[]) {
    setEntries(updated)
    await invoke('save_fillers', { entries: updated })
  }

  async function handleAdd() {
    const word = input.trim()
    if (!word) return
    if (entries.some((e) => e.word.toLowerCase() === word.toLowerCase())) {
      setInput('')
      return
    }
    await persist([...entries, { id: crypto.randomUUID(), word }])
    setInput('')
    inputRef.current?.focus()
  }

  async function handleDelete(id: string) {
    await persist(entries.filter((e) => e.id !== id))
  }

  function toggleEnabled() {
    onChange({
      ...settings,
      transcription: { ...settings.transcription, removeFillerWords: !enabled },
    })
  }

  return (
    <div>
      <p className="page-eyebrow">einstellungen</p>
      <div className="flex items-center justify-between" style={{ marginBottom: 14 }}>
        <h1 className="page-title" style={{ marginBottom: 0 }}>{t('settings.fillers.title')}</h1>
        <button
          role="switch"
          aria-checked={enabled}
          onClick={toggleEnabled}
          className={`v-switch${enabled ? ' on' : ''}`}
        />
      </div>

      <p className="text-xs text-text-muted mb-4">
        {t('settings.fillers.description')}
      </p>

      <div className="flex gap-2 mb-4" style={{ opacity: enabled ? 1 : 0.55 }}>
        <input
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleAdd()}
          placeholder={t('settings.fillers.wordPlaceholder')}
          disabled={!enabled}
          className="flex-1 px-3 py-1.5 text-sm bg-surface border border-border rounded-lg text-text placeholder:text-text-muted focus:outline-none focus:border-accent disabled:cursor-not-allowed"
        />
        <button
          onClick={handleAdd}
          disabled={!enabled || !input.trim()}
          className="px-3 py-1.5 text-xs bg-accent text-accent-fg rounded-lg disabled:opacity-40 disabled:cursor-not-allowed hover:bg-accent-hover transition-colors"
        >
          {t('settings.fillers.add')}
        </button>
      </div>

      {entries.length > 0 ? (
        <div className="flex flex-wrap gap-1.5" style={{ opacity: enabled ? 1 : 0.55 }}>
          {entries.map((entry) => (
            <div
              key={entry.id}
              className="flex items-center gap-1 px-2.5 py-1 rounded-full border border-border bg-surface-raised group"
            >
              <span className="text-xs text-text">{entry.word}</span>
              <button
                onClick={() => handleDelete(entry.id)}
                disabled={!enabled}
                className="text-text-muted hover:text-red-400 transition-colors opacity-0 group-hover:opacity-100 ml-0.5 text-xs leading-none disabled:cursor-not-allowed"
                aria-label={t('common.delete')}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-xs text-text-muted text-center py-8">
          {t('settings.fillers.empty')}
        </p>
      )}
    </div>
  )
}
