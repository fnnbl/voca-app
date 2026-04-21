// Stats page — usage insights.
// Data is mock until backend exposes a stats API.

const DAYS = [12,8,18,22,30,14,45,24,38,42,28,52,35,48,30,22,56,41,38,62,44,52,38,68,54,48,42,38,55,49]

export function StatsPage() {
  const max = Math.max(...DAYS)

  return (
    <div>
      <p className="page-eyebrow">statistiken</p>
      <h1 className="page-title">3<em>.</em>284</h1>
      <p className="page-sub">Wörter diktiert seit du VOCA benutzt. Ungefähr 42 Stunden gespartes Tippen.</p>

      <div className="stats-hero">
        <div className="cell">
          <div className="k">Diese Woche</div>
          <div className="v">412<span className="unit">wörter</span></div>
          <div className="trend">↗ +23% vs. letzte Woche</div>
        </div>
        <div className="cell">
          <div className="k">Ø Geschwindigkeit</div>
          <div className="v">168<span className="unit">wpm</span></div>
          <div className="trend down">~ Tippen: 55 wpm</div>
        </div>
        <div className="cell">
          <div className="k">Heute gespart</div>
          <div className="v">78<span className="unit">min</span></div>
          <div className="trend">vs. Tippen</div>
        </div>
      </div>

      <div className="chart-wrap">
        <div className="sec-head">Aktivität · letzte 30 Tage</div>
        <div className="chart" style={{ marginTop: 16 }}>
          {DAYS.map((d, i) => (
            <div
              key={i}
              className={`bar-col${i === DAYS.length - 1 ? ' today' : ''}`}
              style={{ height: `${(d / max) * 100}%` }}
            />
          ))}
        </div>
        <div className="chart-axis">
          <span>vor 30</span><span>vor 20</span><span>vor 10</span><span>heute</span>
        </div>
      </div>

      <div className="stats-grid">
        <div className="cell">
          <div className="k">Längste Session</div>
          <div className="v">4:32</div>
          <div className="desc">14. April, beim Schreiben einer Spec-Note</div>
        </div>
        <div className="cell">
          <div className="k">Häufigstes Ziel</div>
          <div className="v">Slack</div>
          <div className="desc">38% aller Transkripte</div>
        </div>
        <div className="cell">
          <div className="k">AI-Polish Quote</div>
          <div className="v">64%</div>
          <div className="desc">Anteil mit Enhancement</div>
        </div>
        <div className="cell">
          <div className="k">Provider</div>
          <div className="v" style={{ fontSize: 22 }}>Groq</div>
          <div className="prov-mini">
            <span className="chip">Groq<span className="pct"> · 82%</span></span>
            <span className="chip">Lokal<span className="pct"> · 14%</span></span>
            <span className="chip">Deepgram<span className="pct"> · 4%</span></span>
          </div>
        </div>
      </div>
    </div>
  )
}
