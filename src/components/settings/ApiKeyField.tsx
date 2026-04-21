import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useApiKey, type KeyType } from '../../hooks/useApiKey'

interface Props {
  keyType?: KeyType
  placeholder?: string
}

export function ApiKeyField({ keyType = 'whisper_api_key', placeholder = 'sk-...' }: Props) {
  const { t } = useTranslation()
  const { apiKey, save, remove, loading } = useApiKey(keyType)
  const [input, setInput] = useState('')
  const [saving, setSaving] = useState(false)

  const hasKey = apiKey !== null

  async function handleSave() {
    if (!input.trim()) return
    setSaving(true)
    try {
      await save(input.trim())
      setInput('')
    } finally {
      setSaving(false)
    }
  }

  async function handleRemove() {
    await remove()
  }

  if (loading) {
    return <p className="text-xs text-text-muted">...</p>
  }

  return (
    <div className="space-y-2">
      {hasKey ? (
        <div className="flex items-center gap-2">
          <span className="text-xs font-mono bg-surface px-2 py-1 rounded border border-border text-text-muted tracking-widest">
            {'•'.repeat(16)}
          </span>
          <button
            onClick={handleRemove}
            className="text-xs text-text-muted hover:text-text transition-colors"
          >
            {t('common.delete')}
          </button>
        </div>
      ) : (
        <div className="flex items-center gap-2">
          <input
            type="password"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSave()}
            placeholder={placeholder}
            className="w-52 text-xs bg-surface-raised border border-border rounded px-2 py-1.5 text-text placeholder:text-text-subtle focus:outline-none focus:border-border-hover"
          />
          <button
            onClick={handleSave}
            disabled={!input.trim() || saving}
            className="text-xs px-3 py-1.5 bg-accent text-accent-fg rounded hover:bg-accent-hover disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            {t('common.save')}
          </button>
        </div>
      )}
    </div>
  )
}
