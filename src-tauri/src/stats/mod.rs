use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};

use crate::storage::HistoryEntry;
use crate::target_app::labels::pretty_label;

const TYPING_WPM: f64 = 55.0;
const ACTIVITY_DAYS: usize = 30;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStat {
    pub name: String,
    pub count: u64,
    pub share: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetAppStat {
    pub name: String,
    pub count: u64,
    pub share: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummary {
    pub total_words: u64,
    pub total_duration_secs: f64,
    pub avg_wpm: f64,
    pub words_today: u64,
    pub words_this_week: u64,
    pub words_last_week: u64,
    pub minutes_saved_today: f64,
    pub minutes_saved_total: f64,
    pub longest_session_secs: f32,
    pub longest_session_timestamp_ms: Option<u64>,
    pub ai_polish_rate: f64,
    pub providers: Vec<ProviderStat>,
    pub activity_30d: Vec<u64>,
    pub top_target_app: Option<TargetAppStat>,
    pub target_app_coverage: f64,
}

pub fn aggregate(history: &[HistoryEntry], now_ms: i64) -> StatsSummary {
    let now_local: DateTime<Local> = DateTime::<Utc>::from_timestamp_millis(now_ms)
        .unwrap_or_default()
        .with_timezone(&Local);
    let today = now_local.date_naive();
    let this_monday = monday_of(today);
    let last_monday = this_monday - Duration::days(7);
    let activity_start = today - Duration::days((ACTIVITY_DAYS - 1) as i64);

    let mut total_words: u64 = 0;
    let mut total_duration_secs: f64 = 0.0;
    let mut words_today: u64 = 0;
    let mut words_this_week: u64 = 0;
    let mut words_last_week: u64 = 0;
    let mut minutes_saved_today: f64 = 0.0;
    let mut minutes_saved_total: f64 = 0.0;
    let mut longest: Option<(f32, u64)> = None;
    let mut enhanced_count: u64 = 0;
    let mut provider_counts: HashMap<String, u64> = HashMap::new();
    let mut target_app_counts: HashMap<String, u64> = HashMap::new();
    let mut entries_with_target_app: u64 = 0;
    let mut activity = vec![0u64; ACTIVITY_DAYS];

    for entry in history {
        total_words += entry.word_count as u64;
        total_duration_secs += entry.duration_secs as f64;
        if entry.enhanced {
            enhanced_count += 1;
        }
        *provider_counts.entry(entry.provider.clone()).or_insert(0) += 1;
        if let Some(target) = entry.target_app.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let label = pretty_label(target);
            if !label.is_empty() {
                *target_app_counts.entry(label).or_insert(0) += 1;
                entries_with_target_app += 1;
            }
        }

        let is_new_longest = match &longest {
            None => true,
            Some((d, _)) => entry.duration_secs > *d,
        };
        if is_new_longest {
            longest = Some((entry.duration_secs, entry.timestamp_ms));
        }

        let saved_minutes = time_saved_minutes(entry.word_count, entry.duration_secs as f64);
        minutes_saved_total += saved_minutes;

        let Some(entry_date) = local_date(entry.timestamp_ms as i64) else {
            continue;
        };

        if entry_date == today {
            words_today += entry.word_count as u64;
            minutes_saved_today += saved_minutes;
        }
        if entry_date >= this_monday && entry_date <= today {
            words_this_week += entry.word_count as u64;
        } else if entry_date >= last_monday && entry_date < this_monday {
            words_last_week += entry.word_count as u64;
        }
        if entry_date >= activity_start && entry_date <= today {
            let idx = (entry_date - activity_start).num_days() as usize;
            if idx < ACTIVITY_DAYS {
                activity[idx] += entry.word_count as u64;
            }
        }
    }

    let total = history.len() as f64;
    let mut providers: Vec<ProviderStat> = provider_counts
        .into_iter()
        .map(|(name, count)| ProviderStat {
            share: if total > 0.0 { count as f64 / total } else { 0.0 },
            count,
            name,
        })
        .collect();
    providers.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    let avg_wpm = if total_duration_secs > 0.0 {
        (total_words as f64) / (total_duration_secs / 60.0)
    } else {
        0.0
    };

    let ai_polish_rate = if history.is_empty() {
        0.0
    } else {
        enhanced_count as f64 / history.len() as f64
    };

    let (longest_session_secs, longest_session_timestamp_ms) = match longest {
        Some((d, ts)) => (d, Some(ts)),
        None => (0.0, None),
    };

    let top_target_app = target_app_counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(name, count)| TargetAppStat {
            share: count as f64 / entries_with_target_app.max(1) as f64,
            count,
            name,
        });
    let target_app_coverage = if history.is_empty() {
        0.0
    } else {
        entries_with_target_app as f64 / history.len() as f64
    };

    StatsSummary {
        total_words,
        total_duration_secs,
        avg_wpm,
        words_today,
        words_this_week,
        words_last_week,
        minutes_saved_today,
        minutes_saved_total,
        longest_session_secs,
        longest_session_timestamp_ms,
        ai_polish_rate,
        providers,
        activity_30d: activity,
        top_target_app,
        target_app_coverage,
    }
}

fn local_date(ms: i64) -> Option<NaiveDate> {
    DateTime::<Utc>::from_timestamp_millis(ms).map(|dt| dt.with_timezone(&Local).date_naive())
}

fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

fn time_saved_minutes(word_count: u32, duration_secs: f64) -> f64 {
    let typing_secs = (word_count as f64 / TYPING_WPM) * 60.0;
    let saved = typing_secs - duration_secs;
    if saved > 0.0 {
        saved / 60.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ts_ms: u64, words: u32, duration: f32, enhanced: bool, provider: &str) -> HistoryEntry {
        HistoryEntry {
            id: format!("id-{ts_ms}"),
            timestamp_ms: ts_ms,
            text: String::new(),
            enhanced,
            duration_secs: duration,
            word_count: words,
            provider: provider.into(),
            target_app: None,
        }
    }

    fn entry_with_app(ts_ms: u64, provider: &str, app: Option<&str>) -> HistoryEntry {
        HistoryEntry {
            id: format!("id-{ts_ms}"),
            timestamp_ms: ts_ms,
            text: String::new(),
            enhanced: false,
            duration_secs: 10.0,
            word_count: 10,
            provider: provider.into(),
            target_app: app.map(str::to_owned),
        }
    }

    fn now_ms() -> i64 {
        // Fixed anchor at 12:00 UTC on 2026-04-23 (a Thursday). Using
        // Local::now() made every time-bucket test flaky on CI: whenever
        // the runner fired within the first hour after midnight UTC, an
        // entry stamped "1 h ago" landed on yesterday's calendar date and
        // the `words_today` / `activity_30d[29]` assertions flipped to
        // zero. A fixed mid-day UTC anchor keeps the ±hour offsets we
        // use in these tests inside the same local date regardless of
        // when the runner fires or which timezone it inherits.
        chrono::DateTime::parse_from_rfc3339("2026-04-23T12:00:00Z")
            .expect("valid rfc3339 anchor")
            .timestamp_millis()
    }

    fn day_ms() -> u64 {
        86_400_000
    }

    #[test]
    fn aggregate_empty_history_returns_zeros() {
        let s = aggregate(&[], now_ms());
        assert_eq!(s.total_words, 0);
        assert_eq!(s.total_duration_secs, 0.0);
        assert_eq!(s.avg_wpm, 0.0);
        assert_eq!(s.words_today, 0);
        assert_eq!(s.ai_polish_rate, 0.0);
        assert_eq!(s.longest_session_timestamp_ms, None);
        assert_eq!(s.activity_30d.len(), ACTIVITY_DAYS);
        assert!(s.providers.is_empty());
        assert!(s.top_target_app.is_none());
        assert_eq!(s.target_app_coverage, 0.0);
    }

    #[test]
    fn aggregate_top_target_app_picks_most_frequent() {
        let now = now_ms();
        let hist = vec![
            entry_with_app(now as u64, "groq", Some("Slack")),
            entry_with_app(now as u64, "groq", Some("Chrome")),
            entry_with_app(now as u64, "groq", Some("Slack")),
            entry_with_app(now as u64, "groq", Some("Slack")),
        ];
        let s = aggregate(&hist, now);
        let top = s.top_target_app.unwrap();
        assert_eq!(top.name, "Slack");
        assert_eq!(top.count, 3);
        assert!((top.share - 0.75).abs() < 1e-9);
    }

    #[test]
    fn aggregate_top_target_app_ignores_entries_without_target() {
        let now = now_ms();
        let hist = vec![
            entry_with_app(now as u64, "groq", None),
            entry_with_app(now as u64, "groq", None),
            entry_with_app(now as u64, "groq", Some("Chrome")),
        ];
        let s = aggregate(&hist, now);
        let top = s.top_target_app.unwrap();
        assert_eq!(top.name, "Chrome");
        assert_eq!(top.count, 1);
        // share is relative to entries that have a target app, not total history
        assert!((top.share - 1.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_top_target_app_none_when_all_entries_lack_target() {
        let now = now_ms();
        let hist = vec![
            entry_with_app(now as u64, "groq", None),
            entry_with_app(now as u64, "groq", None),
        ];
        let s = aggregate(&hist, now);
        assert!(s.top_target_app.is_none());
        assert_eq!(s.target_app_coverage, 0.0);
    }

    #[test]
    fn aggregate_target_app_coverage_fraction_of_history() {
        let now = now_ms();
        let hist = vec![
            entry_with_app(now as u64, "groq", Some("Slack")),
            entry_with_app(now as u64, "groq", None),
            entry_with_app(now as u64, "groq", None),
            entry_with_app(now as u64, "groq", Some("Chrome")),
        ];
        let s = aggregate(&hist, now);
        assert!((s.target_app_coverage - 0.5).abs() < 1e-9);
    }

    #[test]
    fn aggregate_top_target_app_applies_pretty_label() {
        let now = now_ms();
        let hist = vec![
            entry_with_app(now as u64, "groq", Some("Code.exe")),
            entry_with_app(now as u64, "groq", Some("code")),
            entry_with_app(now as u64, "groq", Some("com.microsoft.VSCode")),
        ];
        let s = aggregate(&hist, now);
        let top = s.top_target_app.unwrap();
        assert_eq!(top.name, "VS Code");
        assert_eq!(top.count, 3);
    }

    #[test]
    fn aggregate_top_target_app_strips_exe_for_unknown_apps() {
        let now = now_ms();
        let hist = vec![entry_with_app(now as u64, "groq", Some("MyCustomTool.exe"))];
        let s = aggregate(&hist, now);
        let top = s.top_target_app.unwrap();
        assert_eq!(top.name, "MyCustomTool");
    }

    #[test]
    fn aggregate_target_app_ignores_whitespace_only_values() {
        let now = now_ms();
        let hist = vec![
            entry_with_app(now as u64, "groq", Some("   ")),
            entry_with_app(now as u64, "groq", Some("Slack")),
        ];
        let s = aggregate(&hist, now);
        let top = s.top_target_app.unwrap();
        assert_eq!(top.name, "Slack");
        assert_eq!(top.count, 1);
    }

    #[test]
    fn aggregate_sums_totals() {
        let now = now_ms();
        let hist = vec![
            entry(now as u64, 100, 30.0, false, "groq"),
            entry(now as u64 - 3_600_000, 50, 20.0, true, "groq"),
        ];
        let s = aggregate(&hist, now);
        assert_eq!(s.total_words, 150);
        assert!((s.total_duration_secs - 50.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_avg_wpm_from_totals() {
        let now = now_ms();
        // 100 words in 60s = 100 wpm
        let hist = vec![entry(now as u64, 100, 60.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert!((s.avg_wpm - 100.0).abs() < 1e-6);
    }

    #[test]
    fn aggregate_avg_wpm_zero_when_no_duration() {
        let s = aggregate(&[entry(now_ms() as u64, 50, 0.0, false, "groq")], now_ms());
        assert_eq!(s.avg_wpm, 0.0);
    }

    #[test]
    fn aggregate_words_today_counts_recent_only() {
        let now = now_ms();
        let hist = vec![
            entry(now as u64 - 3_600_000, 10, 5.0, false, "groq"), // ~1h ago
            entry(now as u64 - 10 * day_ms() as u64, 30, 10.0, false, "groq"), // 10 days ago
        ];
        let s = aggregate(&hist, now);
        assert_eq!(s.words_today, 10);
    }

    #[test]
    fn aggregate_words_this_week_includes_today() {
        let now = now_ms();
        let hist = vec![entry(now as u64 - 3_600_000, 42, 20.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert_eq!(s.words_this_week, 42);
        assert_eq!(s.words_last_week, 0);
    }

    #[test]
    fn aggregate_words_last_week_captures_seven_days_ago() {
        let now = now_ms();
        // 7 days ago is always in "last week" relative to today (same weekday, previous ISO week)
        let hist = vec![entry(now as u64 - 7 * day_ms(), 25, 10.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert_eq!(s.words_last_week, 25);
        assert_eq!(s.words_this_week, 0);
    }

    #[test]
    fn aggregate_time_saved_positive_for_fast_dictation() {
        let now = now_ms();
        // 55 words in 30s = 110 wpm → typing-at-55-wpm would take 60s → saved 30s = 0.5 min
        let hist = vec![entry(now as u64, 55, 30.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert!((s.minutes_saved_total - 0.5).abs() < 1e-9);
        assert!((s.minutes_saved_today - 0.5).abs() < 1e-9);
    }

    #[test]
    fn aggregate_time_saved_floors_at_zero_for_slow_dictation() {
        let now = now_ms();
        // 10 words in 60s = 10 wpm, way below typing baseline → saved = 0
        let hist = vec![entry(now as u64, 10, 60.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert_eq!(s.minutes_saved_total, 0.0);
    }

    #[test]
    fn aggregate_longest_session_tracked() {
        let now = now_ms();
        let hist = vec![
            entry(now as u64, 10, 5.0, false, "groq"),
            entry(now as u64 - 1000, 20, 42.0, false, "groq"),
            entry(now as u64 - 2000, 30, 18.0, false, "groq"),
        ];
        let s = aggregate(&hist, now);
        assert!((s.longest_session_secs - 42.0).abs() < 1e-6);
        assert_eq!(s.longest_session_timestamp_ms, Some(now as u64 - 1000));
    }

    #[test]
    fn aggregate_ai_polish_rate() {
        let now = now_ms();
        let hist = vec![
            entry(now as u64, 10, 5.0, true, "groq"),
            entry(now as u64, 10, 5.0, false, "groq"),
            entry(now as u64, 10, 5.0, true, "groq"),
            entry(now as u64, 10, 5.0, true, "groq"),
        ];
        let s = aggregate(&hist, now);
        assert!((s.ai_polish_rate - 0.75).abs() < 1e-9);
    }

    #[test]
    fn aggregate_providers_sorted_by_count_desc() {
        let now = now_ms();
        let hist = vec![
            entry(now as u64, 10, 5.0, false, "groq"),
            entry(now as u64, 10, 5.0, false, "local"),
            entry(now as u64, 10, 5.0, false, "groq"),
            entry(now as u64, 10, 5.0, false, "groq"),
            entry(now as u64, 10, 5.0, false, "deepgram"),
        ];
        let s = aggregate(&hist, now);
        assert_eq!(s.providers[0].name, "groq");
        assert_eq!(s.providers[0].count, 3);
        assert!((s.providers[0].share - 0.6).abs() < 1e-9);
        // Tie on count=1 → alphabetical order
        assert_eq!(s.providers[1].name, "deepgram");
        assert_eq!(s.providers[2].name, "local");
    }

    #[test]
    fn aggregate_activity_30d_has_30_buckets() {
        let s = aggregate(&[], now_ms());
        assert_eq!(s.activity_30d.len(), 30);
    }

    #[test]
    fn aggregate_activity_30d_today_is_last_bucket() {
        let now = now_ms();
        let hist = vec![entry(now as u64 - 3_600_000, 17, 10.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert_eq!(s.activity_30d[29], 17);
        assert_eq!(s.activity_30d[28], 0);
    }

    #[test]
    fn aggregate_activity_30d_excludes_older_than_30_days() {
        let now = now_ms();
        let hist = vec![entry(now as u64 - 40 * day_ms(), 99, 10.0, false, "groq")];
        let s = aggregate(&hist, now);
        assert!(s.activity_30d.iter().all(|&c| c == 0));
    }

    #[test]
    fn monday_of_monday_returns_self() {
        // 2026-04-20 is a Monday
        let d = NaiveDate::from_ymd_opt(2026, 4, 20).unwrap();
        assert_eq!(monday_of(d), d);
    }

    #[test]
    fn monday_of_sunday_returns_previous_monday() {
        // 2026-04-26 is a Sunday → Monday 2026-04-20
        let sunday = NaiveDate::from_ymd_opt(2026, 4, 26).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 4, 20).unwrap();
        assert_eq!(monday_of(sunday), monday);
    }

    #[test]
    fn time_saved_minutes_positive_when_faster_than_baseline() {
        // 110 words in 30s (220 wpm) — baseline typing would take 120s → saved 90s = 1.5 min
        let saved = time_saved_minutes(110, 30.0);
        assert!((saved - 1.5).abs() < 1e-9);
    }

    #[test]
    fn time_saved_minutes_zero_when_slower_than_baseline() {
        assert_eq!(time_saved_minutes(5, 60.0), 0.0);
    }
}
