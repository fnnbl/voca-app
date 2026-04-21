import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import type { Snippet } from '../../types'

function newSnippet(): Omit<Snippet, 'id' | 'createdAt'> {
  return { name: '', trigger: '', output: '', enabled: true }
}

export function SnippetsSettings() {
  const { t } = useTranslation()
  const [snippets, setSnippets] = useState<Snippet[]>([])
  const [adding, setAdding] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [form, setForm] = useState(newSnippet())

  useEffect(() => {
    invoke<Snippet[]>('get_snippets').then(setSnippets).catch(console.error)
  }, [])

  async function persist(updated: Snippet[]) {
    setSnippets(updated)
    await invoke('save_snippets', { snippets: updated })
  }

  async function handleAdd() {
    if (!form.trigger.trim()) return
    const snippet: Snippet = {
      id: crypto.randomUUID(),
      createdAt: new Date().toISOString(),
      ...form,
    }
    await persist([...snippets, snippet])
    setAdding(false)
    setForm(newSnippet())
  }

  async function handleSaveEdit(id: string) {
    if (!form.trigger.trim()) return
    await persist(snippets.map((s) => (s.id === id ? { ...s, ...form } : s)))
    setEditingId(null)
  }

  async function handleDelete(id: string) {
    await persist(snippets.filter((s) => s.id !== id))
  }

  async function handleToggle(id: string) {
    await persist(snippets.map((s) => (s.id === id ? { ...s, enabled: !s.enabled } : s)))
  }

  function startEdit(s: Snippet) {
    setEditingId(s.id)
    setForm({ name: s.name, trigger: s.trigger, output: s.output, enabled: s.enabled })
    setAdding(false)
  }

  function cancelForm() {
    setAdding(false)
    setEditingId(null)
    setForm(newSnippet())
  }

  return (
    <div>
      <p className="page-eyebrow">einstellungen</p>
      <div className="flex items-center justify-between" style={{ marginBottom: 28 }}>
        <h1 className="page-title" style={{ marginBottom: 0 }}>{t('settings.snippets.title')}</h1>
        {!adding && !editingId && (
          <button
            onClick={() => { setAdding(true); setForm(newSnippet()) }}
            className="text-xs px-3 py-1.5 border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
          >
            + {t('settings.snippets.add')}
          </button>
        )}
      </div>

      <div className="space-y-1.5">
        {snippets.map((s) =>
          editingId === s.id ? (
            <SnippetForm
              key={s.id}
              form={form}
              onChange={setForm}
              onSave={() => handleSaveEdit(s.id)}
              onCancel={cancelForm}
              t={t}
            />
          ) : (
            <div
              key={s.id}
              className="flex items-center gap-3 px-3 py-2.5 rounded-lg border border-border bg-surface-raised group"
            >
              <button
                role="switch"
                aria-checked={s.enabled}
                onClick={() => handleToggle(s.id)}
                className={`v-switch${s.enabled ? ' on' : ''}`}
              />

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-mono px-1.5 py-0.5 rounded bg-surface border border-border text-text-muted">
                    {s.trigger}
                  </span>
                  <span className="text-text-muted text-xs">·</span>
                  <span className="text-xs text-text truncate">{s.output}</span>
                </div>
                {s.name && <p className="text-[10px] text-text-muted mt-0.5">{s.name}</p>}
              </div>

              <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={() => startEdit(s)}
                  className="text-xs px-2 py-1 text-text-muted hover:text-text transition-colors"
                >
                  {t('common.edit')}
                </button>
                <button
                  onClick={() => handleDelete(s.id)}
                  className="text-xs px-2 py-1 text-red-400 hover:text-red-300 transition-colors"
                >
                  {t('common.delete')}
                </button>
              </div>
            </div>
          )
        )}

        {adding && (
          <SnippetForm
            form={form}
            onChange={setForm}
            onSave={handleAdd}
            onCancel={cancelForm}
            t={t}
          />
        )}
      </div>

      {snippets.length === 0 && !adding && (
        <p className="text-xs text-text-muted text-center py-8">
          {t('settings.snippets.add')}
        </p>
      )}
    </div>
  )
}

interface FormProps {
  form: ReturnType<typeof newSnippet>
  onChange: (f: ReturnType<typeof newSnippet>) => void
  onSave: () => void
  onCancel: () => void
  t: ReturnType<typeof useTranslation>['t']
}

function SnippetForm({ form, onChange, onSave, onCancel, t }: FormProps) {
  return (
    <div className="p-3 rounded-lg border border-accent bg-accent-subtle space-y-2">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <label className="block text-[10px] text-text-muted mb-1">{t('settings.snippets.trigger')}</label>
          <input
            value={form.trigger}
            onChange={(e) => onChange({ ...form, trigger: e.target.value })}
            placeholder="e.g. sig"
            className="w-full px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono"
          />
        </div>
        <div>
          <label className="block text-[10px] text-text-muted mb-1">{t('settings.snippets.name')}</label>
          <input
            value={form.name}
            onChange={(e) => onChange({ ...form, name: e.target.value })}
            placeholder={t('settings.snippets.name')}
            className="w-full px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
          />
        </div>
      </div>
      <div>
        <label className="block text-[10px] text-text-muted mb-1">{t('settings.snippets.output')}</label>
        <textarea
          value={form.output}
          onChange={(e) => onChange({ ...form, output: e.target.value })}
          rows={3}
          placeholder="Replacement text…"
          className="w-full px-2.5 py-1.5 text-xs bg-surface border border-border rounded-lg text-text placeholder:text-text-muted focus:outline-none focus:border-accent resize-none"
        />
      </div>
      <div className="flex justify-end gap-2">
        <button onClick={onCancel} className="text-xs px-3 py-1.5 text-text-muted hover:text-text transition-colors">
          {t('common.cancel')}
        </button>
        <button
          onClick={onSave}
          disabled={!form.trigger.trim()}
          className="text-xs px-3 py-1.5 bg-accent text-accent-fg rounded-lg disabled:opacity-40 disabled:cursor-not-allowed hover:bg-accent-hover transition-colors"
        >
          {t('common.save')}
        </button>
      </div>
    </div>
  )
}
