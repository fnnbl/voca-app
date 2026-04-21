import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface HistoryEntry {
  id: string
  timestampMs: number
  text: string
  enhanced: boolean
  durationSecs: number
  wordCount: number
  provider: string
}

interface HistoryGroup {
  g: string
  items: HistoryEntry[]
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })
}

function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60)
  const s = Math.floor(secs % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}

function groupByDate(entries: HistoryEntry[]): HistoryGroup[] {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
  const yesterday = today - 86_400_000

  const groups = new Map<string, HistoryEntry[]>()
  for (const entry of [...entries].reverse()) {
    const d = new Date(entry.timestampMs)
    const dayStart = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
    let label: string
    if (dayStart >= today) label = 'heute'
    else if (dayStart >= yesterday) label = 'gestern'
    else label = d.toLocaleDateString('de-DE', { day: 'numeric', month: 'long' })

    if (!groups.has(label)) groups.set(label, [])
    groups.get(label)!.push(entry)
  }
  return Array.from(groups.entries()).map(([g, items]) => ({ g, items }))
}

export function HistoryPage() {
  const [entries, setEntries] = useState<HistoryEntry[]>([])
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<'all' | 'enhanced' | 'raw'>('all')

  useEffect(() => {
    invoke<HistoryEntry[]>('get_history').then(setEntries).catch(console.error)
  }, [])

  const filtered = entries.filter((h) => {
    const matchesQuery = query === '' || h.text.toLowerCase().includes(query.toLowerCase())
    const matchesFilter = filter === 'all' || (filter === 'enhanced' ? h.enhanced : !h.enhanced)
    return matchesQuery && matchesFilter
  })
  const grouped = groupByDate(filtered)

  return (
    <div>
      <p className="page-eyebrow">verlauf</p>
      <h1 className="page-title"><em>History</em></h1>
      <p className="page-sub">Alle deine Transkripte, lokal gespeichert. Nichts verlässt dein Gerät.</p>

      <div className="hist-filters">
        <div className="hist-search">
          <SearchIcon />
          <input
            placeholder="Durchsuchen…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <div className="v-seg">
          {(['all', 'enhanced', 'raw'] as const).map((v) => (
            <button key={v} className={filter === v ? 'is-active' : ''} onClick={() => setFilter(v)}>
              {v === 'all' ? 'Alle' : v === 'enhanced' ? 'Poliert' : 'Roh'}
            </button>
          ))}
        </div>
        <span style={{ marginLeft: 'auto', fontFamily: 'var(--f-mono)', fontSize: 11, color: 'var(--v-ink-3)' }}>
          {entries.length} Einträge
        </span>
      </div>

      {grouped.length === 0 ? (
        <div style={{ textAlign: 'center', padding: '80px 20px', color: 'var(--v-ink-3)', fontFamily: 'var(--f-mono)', fontSize: 12 }}>
          {entries.length === 0 ? 'Noch keine Transkripte.' : 'Keine Einträge gefunden.'}
        </div>
      ) : (
        grouped.map((grp) => (
          <div key={grp.g}>
            <div className="hist-group-label">{grp.g}</div>
            {grp.items.map((h) => {
              const wpm = h.durationSecs > 0 ? Math.round(h.wordCount / (h.durationSecs / 60)) : 0
              return (
                <div className="hist-item" key={h.id}>
                  <span className="time">{formatTime(h.timestampMs)}</span>
                  <div className="body">
                    <div className="text">{h.text}</div>
                    <div className="meta">
                      <span>{formatDuration(h.durationSecs)}</span>
                      <span>{h.wordCount} Wörter</span>
                      <span>{h.provider}</span>
                      {h.enhanced && <span className="enhanced">★ poliert</span>}
                    </div>
                  </div>
                  <div className="wpm">{wpm > 0 ? wpm : '—'}<small>wpm</small></div>
                </div>
              )
            })}
          </div>
        ))
      )}
    </div>
  )
}

function SearchIcon() {
  return (
    <svg width={13} height={13} viewBox="0 0 16 16" fill="none" stroke="var(--v-ink-3)" strokeWidth="1.25" strokeLinecap="round">
      <circle cx="7" cy="7" r="4.5"/>
      <path d="M10.5 10.5L14 14"/>
    </svg>
  )
}
