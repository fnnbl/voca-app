//! Filler-word suggestion engine.
//!
//! Pure-statistical analysis over transcript history. Tokenises each
//! `HistoryEntry.text` lowercase, counts occurrences of a fixed DE+EN
//! seed list, and ranks the top candidates that the user has neither
//! accepted (already on the manual list) nor rejected (dismissed
//! previously). No NLP, no language detection, no network calls.

use std::collections::HashSet;

use crate::storage::{FillersFile, HistoryEntry};

/// Fixed seed list of common DE + EN filler-word candidates. Multi-word
/// phrases are matched as case-insensitive substrings; single words are
/// matched against tokenised words via `is_alphabetic` boundaries.
pub const SEED_FILLERS: &[&str] = &[
    // German
    "also",
    "halt",
    "ja",
    "quasi",
    "sozusagen",
    "irgendwie",
    "eigentlich",
    "ne",
    "weißt du",
    "ähm",
    "öhm",
    // English
    "um",
    "uh",
    "like",
    "you know",
    "basically",
    "literally",
    "actually",
    "right",
    "so",
    "i mean",
];

/// Minimum total occurrences for a candidate to be suggested.
const MIN_OCCURRENCES: usize = 5;

/// Minimum number of distinct transcripts a candidate must appear in.
/// Guards against suggestions that come from a single chatty transcript.
const MIN_DISTINCT_TRANSCRIPTS: usize = 3;

/// Maximum number of suggestions returned in one call.
const MAX_SUGGESTIONS: usize = 10;

/// Compute filler-word suggestions from history, excluding seeds that are
/// already on the user's manual list (`fillers.words`) or that the user
/// has previously rejected (`fillers.rejected`). Sorted by total count
/// descending, with distinct-transcript count as the tiebreaker.
pub fn compute_suggestions(history: &[HistoryEntry], fillers: &FillersFile) -> Vec<String> {
    let accepted: HashSet<String> = fillers
        .words
        .iter()
        .map(|e| e.word.to_lowercase())
        .collect();
    let rejected: HashSet<String> = fillers
        .rejected
        .iter()
        .map(|w| w.to_lowercase())
        .collect();

    let mut scored: Vec<(String, usize, usize)> = SEED_FILLERS
        .iter()
        .filter_map(|seed| {
            let lower = seed.to_lowercase();
            if accepted.contains(&lower) || rejected.contains(&lower) {
                return None;
            }
            let (count, distinct) = count_in_history(history, &lower);
            if count < MIN_OCCURRENCES || distinct < MIN_DISTINCT_TRANSCRIPTS {
                return None;
            }
            Some((seed.to_string(), count, distinct))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    scored.truncate(MAX_SUGGESTIONS);
    scored.into_iter().map(|(w, _, _)| w).collect()
}

/// Returns `(total_count, distinct_transcript_count)` for `seed_lower`
/// across `history`. Phrases (containing whitespace) match as substrings;
/// single words match on alphabetic boundaries.
fn count_in_history(history: &[HistoryEntry], seed_lower: &str) -> (usize, usize) {
    let is_phrase = seed_lower.contains(' ');
    let mut total = 0usize;
    let mut distinct = 0usize;
    for entry in history {
        let lower_text = entry.text.to_lowercase();
        let occurrences = if is_phrase {
            count_substring(&lower_text, seed_lower)
        } else {
            count_word(&lower_text, seed_lower)
        };
        if occurrences > 0 {
            total += occurrences;
            distinct += 1;
        }
    }
    (total, distinct)
}

fn count_substring(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

fn count_word(text: &str, word: &str) -> usize {
    text.split(|c: char| !c.is_alphabetic())
        .filter(|tok| *tok == word)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::FillerEntry;

    fn entry(id: u64, text: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            timestamp_ms: id,
            text: text.into(),
            enhanced: false,
            duration_secs: 1.0,
            word_count: text.split_whitespace().count() as u32,
            provider: "test".into(),
            target_app: None,
        }
    }

    #[test]
    fn ranks_by_total_count_desc() {
        // "also" appears 6 times across 3 transcripts; "halt" 5 across 3.
        // "also" should rank first.
        let history = vec![
            entry(1, "ich gehe also nach hause also wirklich"),
            entry(2, "also kommt das halt darauf an halt"),
            entry(3, "also halt halt"),
        ];
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(suggestions.contains(&"also".to_string()));
        assert!(suggestions.contains(&"halt".to_string()));
        let also_idx = suggestions.iter().position(|w| w == "also").unwrap();
        let halt_idx = suggestions.iter().position(|w| w == "halt").unwrap();
        assert!(also_idx < halt_idx, "also should rank above halt: {:?}", suggestions);
    }

    #[test]
    fn excludes_already_accepted_words() {
        let history = vec![
            entry(1, "also also also"),
            entry(2, "also also halt halt"),
            entry(3, "also halt halt halt"),
        ];
        let fillers = FillersFile {
            words: vec![FillerEntry {
                id: "1".into(),
                word: "also".into(),
            }],
            rejected: vec![],
        };
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(!suggestions.contains(&"also".to_string()));
    }

    #[test]
    fn excludes_rejected_words() {
        let history = vec![
            entry(1, "also also also"),
            entry(2, "also also halt halt"),
            entry(3, "also halt halt halt"),
        ];
        let fillers = FillersFile {
            words: vec![],
            rejected: vec!["also".into()],
        };
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(!suggestions.contains(&"also".to_string()));
    }

    #[test]
    fn rejection_match_is_case_insensitive() {
        let history = vec![
            entry(1, "Also Also Also"),
            entry(2, "ALSO also Halt"),
            entry(3, "also Halt halt"),
        ];
        let fillers = FillersFile {
            words: vec![],
            rejected: vec!["ALSO".into()],
        };
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(!suggestions.contains(&"also".to_string()));
    }

    #[test]
    fn respects_min_occurrence_threshold() {
        // "also" appears 4 times across 3 transcripts (count below min);
        // "halt" appears 5 times across 3 transcripts (passes).
        let history = vec![
            entry(1, "also also halt halt"),
            entry(2, "also halt halt"),
            entry(3, "also halt"),
        ];
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(!suggestions.contains(&"also".to_string()));
        assert!(suggestions.contains(&"halt".to_string()));
    }

    #[test]
    fn respects_min_distinct_transcripts_threshold() {
        // "also" appears 8 times but only in 2 distinct transcripts.
        let history = vec![
            entry(1, "also also also also"),
            entry(2, "also also also also"),
        ];
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(!suggestions.contains(&"also".to_string()));
    }

    #[test]
    fn handles_multiword_phrases_via_substring() {
        let history = vec![
            entry(1, "i mean it's complicated, i mean it"),
            entry(2, "well, i mean i think so"),
            entry(3, "i mean really, i mean"),
        ];
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(suggestions.contains(&"i mean".to_string()));
    }

    #[test]
    fn word_match_respects_alphabetic_boundaries() {
        // "so" should not match "soccer", "sole", or "solid".
        let history = vec![
            entry(1, "soccer is fun, sole purpose, solid plan"),
            entry(2, "she sold soap to soldiers"),
            entry(3, "solar systems and solo gigs"),
        ];
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(!suggestions.contains(&"so".to_string()));
    }

    #[test]
    fn empty_history_returns_no_suggestions() {
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&[], &fillers);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn caps_at_max_suggestions() {
        // Synthetic: every seed appears 10x in 10 distinct transcripts.
        let mut history = vec![];
        let chunk = SEED_FILLERS.join(" ").repeat(10);
        for i in 0..10 {
            history.push(entry(i, &chunk));
        }
        let fillers = FillersFile::default();
        let suggestions = compute_suggestions(&history, &fillers);
        assert!(suggestions.len() <= MAX_SUGGESTIONS, "got {} suggestions", suggestions.len());
    }
}
