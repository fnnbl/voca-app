import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type ProviderStat = { name: string; count: number; share: number }
type TargetAppStat = { name: string; count: number; share: number }

type StatsSummary = {
  totalWords: number
  totalDurationSecs: number
  avgWpm: number
  wordsToday: number
  wordsThisWeek: number
  wordsLastWeek: number
  minutesSavedToday: number
  minutesSavedTotal: number
  longestSessionSecs: number
  longestSessionTimestampMs: number | null
  aiPolishRate: number
  providers: ProviderStat[]
  activity30d: number[]
  topTargetApp: TargetAppStat | null
  targetAppCoverage: number
}

const EMPTY: StatsSummary = {
  totalWords: 0,
  totalDurationSecs: 0,
  avgWpm: 0,
  wordsToday: 0,
  wordsThisWeek: 0,
  wordsLastWeek: 0,
  minutesSavedToday: 0,
  minutesSavedTotal: 0,
  longestSessionSecs: 0,
  longestSessionTimestampMs: null,
  aiPolishRate: 0,
  providers: [],
  activity30d: Array(30).fill(0),
  topTargetApp: null,
  targetAppCoverage: 0,
}

const PROVIDER_LABELS: Record<string, string> = {
  groq: 'Groq',
  local: 'Lokal',
  deepgram: 'Deepgram',
  openai: 'OpenAI',
}

export function StatsPage() {
  const [stats, setStats] = useState<StatsSummary>(EMPTY)

  useEffect(() => {
    invoke<StatsSummary>('get_stats').then(setStats).catch(() => setStats(EMPTY))
  }, [])

  const maxBar = Math.max(1, ...stats.activity30d)
  const hoursSaved = stats.minutesSavedTotal / 60

  return (
    <div>
      <p className="page-eyebrow">statistiken</p>
      <h1 className="page-title">{formatGroupedNumber(stats.totalWords)}</h1>
      <p className="page-sub">
        {stats.totalWords === 0
          ? 'Noch keine Aufnahmen. Starte dein erstes Diktat per Shortcut.'
          : `Wörter diktiert seit du VOCA benutzt. Ungefähr ${formatHours(hoursSaved)} gespartes Tippen.`}
      </p>

      <div className="stats-hero">
        <div className="cell">
          <div className="k">Diese Woche</div>
          <div className="v">
            {formatGroupedNumber(stats.wordsThisWeek)}
            <span className="unit">wörter</span>
          </div>
          <div className={weekTrendClass(stats.wordsThisWeek, stats.wordsLastWeek)}>
            {formatWeekTrend(stats.wordsThisWeek, stats.wordsLastWeek)}
          </div>
        </div>
        <div className="cell">
          <div className="k">Ø Geschwindigkeit</div>
          <div className="v">
            {Math.round(stats.avgWpm)}
            <span className="unit">wpm</span>
          </div>
          <div className="trend down">~ Tippen: 55 wpm</div>
        </div>
        <div className="cell">
          <div className="k">Heute gespart</div>
          <div className="v">
            {Math.round(stats.minutesSavedToday)}
            <span className="unit">min</span>
          </div>
          <div className="trend">vs. Tippen</div>
        </div>
      </div>

      <div className="chart-wrap">
        <div className="sec-head">Aktivität · letzte 30 Tage</div>
        <div className="chart" style={{ marginTop: 16 }}>
          {stats.activity30d.map((d, i) => (
            <div
              key={i}
              className={`bar-col${i === stats.activity30d.length - 1 ? ' today' : ''}`}
              style={{ height: `${(d / maxBar) * 100}%` }}
            />
          ))}
        </div>
        <div className="chart-axis">
          <span>vor 30</span>
          <span>vor 20</span>
          <span>vor 10</span>
          <span>heute</span>
        </div>
      </div>

      <div className="stats-grid">
        <div className="cell">
          <div className="k">Längste Session</div>
          <div className="v">{formatDuration(stats.longestSessionSecs)}</div>
          <div className="desc">{formatLongestDate(stats.longestSessionTimestampMs)}</div>
        </div>
        <div className="cell">
          <div className="k">Häufigstes Ziel</div>
          <div className="v">{stats.topTargetApp ? stats.topTargetApp.name : '—'}</div>
          <div className="desc">{formatTargetAppDesc(stats.topTargetApp, stats.targetAppCoverage)}</div>
        </div>
        <div className="cell">
          <div className="k">AI-Polish Quote</div>
          <div className="v">{Math.round(stats.aiPolishRate * 100)}%</div>
          <div className="desc">Anteil mit Enhancement</div>
        </div>
        <div className="cell">
          <div className="k">Provider</div>
          <div className="v" style={{ fontSize: 22 }}>
            {stats.providers[0] ? providerLabel(stats.providers[0].name) : '—'}
          </div>
          <div className="prov-mini">
            {stats.providers.length === 0 ? (
              <span className="chip">Noch keine Daten</span>
            ) : (
              stats.providers.map((p) => (
                <span key={p.name} className="chip">
                  {providerLabel(p.name)}
                  <span className="pct"> · {Math.round(p.share * 100)}%</span>
                </span>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

function providerLabel(name: string): string {
  return PROVIDER_LABELS[name] ?? name
}

function formatGroupedNumber(n: number): string {
  return new Intl.NumberFormat('de-DE').format(n)
}

function formatHours(h: number): string {
  if (h < 1) return `${Math.round(h * 60)} Minuten`
  return `${h.toFixed(1).replace('.', ',')} Stunden`
}

function formatDuration(secs: number): string {
  if (secs <= 0) return '0:00'
  const total = Math.round(secs)
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

function formatLongestDate(ms: number | null): string {
  if (!ms) return 'Noch keine Sessions'
  const d = new Date(ms)
  return new Intl.DateTimeFormat('de-DE', { day: 'numeric', month: 'long' }).format(d)
}

function formatTargetAppDesc(top: TargetAppStat | null, coverage: number): string {
  if (!top) {
    return coverage === 0
      ? 'Tracking deaktiviert oder noch keine Daten'
      : 'Noch keine Daten'
  }
  return `${Math.round(top.share * 100)}% aller erfassten Ziele`
}

function formatWeekTrend(thisWeek: number, lastWeek: number): string {
  if (lastWeek === 0) {
    return thisWeek === 0 ? '—' : 'Start der Woche'
  }
  const delta = ((thisWeek - lastWeek) / lastWeek) * 100
  const sign = delta >= 0 ? '+' : ''
  const arrow = delta >= 0 ? '↗' : '↘'
  return `${arrow} ${sign}${Math.round(delta)}% vs. letzte Woche`
}

function weekTrendClass(thisWeek: number, lastWeek: number): string {
  if (lastWeek === 0 || thisWeek >= lastWeek) return 'trend'
  return 'trend down'
}
