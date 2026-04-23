// Curated seed-term lists per user role, offered in the onboarding use-case
// step. The goal is to fill Whisper's `initial_prompt` vocabulary hint so
// terms the user is likely to dictate — especially Denglisch tech loanwords
// like "For-Schleife", "Pull Request", "Figma" — transcribe correctly from
// the very first dictation.
//
// Lists are language-neutral: the terms shown here are common in both DE
// and EN professional contexts, and Whisper treats them the same either
// way. Adding a new role is a matter of one more `const` array and one line
// in `seeds_for()`.

const DEV_TERMS: &[&str] = &[
    "For-Schleife", "While-Schleife", "If-Statement", "Pull Request",
    "Commit", "Branch", "Merge", "Rebase", "Repository", "Deploy",
    "Debugger", "Framework", "Backend", "Frontend", "Endpoint", "JSON",
    "TypeScript", "JavaScript", "Python", "Rust", "React", "Vue", "Angular",
    "Docker", "Kubernetes", "PostgreSQL", "Redis", "nginx", "OAuth",
    "GitHub", "GitLab", "VS Code", "Stack Trace", "Refactor", "Code Review",
    "Feature Flag", "Staging", "Production", "Bugfix", "Hotfix", "Rollback",
    "Release", "CI/CD", "Linter", "Monorepo",
];

const PM_TERMS: &[&str] = &[
    "Sprint", "Backlog", "Roadmap", "Milestone", "Stakeholder", "Deliverable",
    "Scope", "Feature", "Requirement", "User Story", "Ticket", "Epic",
    "Retro", "Standup", "Review", "OKR", "KPI", "Deadline", "Lead Time",
    "Cycle Time", "Velocity", "Burndown", "Kanban", "Scrum", "Timeline",
    "Budget", "Forecast", "Discovery", "Kick-off", "Steering", "RACI",
];

const CONTENT_TERMS: &[&str] = &[
    "SEO", "CTA", "Keyword", "Headline", "Copy", "Engagement", "Impressions",
    "Analytics", "Audience", "Subscriber", "Thumbnail", "Caption", "Script",
    "Storyboard", "Hook", "Intro", "Outro", "Monetization", "Sponsor",
    "Affiliate", "Newsletter", "Podcast", "Reel", "Short", "Livestream",
    "Follower", "Reach", "Niche", "Evergreen", "Brand",
];

const DESIGN_TERMS: &[&str] = &[
    "Wireframe", "Mockup", "Prototype", "Figma", "Sketch", "Adobe XD",
    "Component", "Design System", "Token", "Grid", "Layout", "User Journey",
    "Persona", "A/B Test", "Accessibility", "Kontrast", "Typography",
    "Whitespace", "Handoff", "Hi-Fi", "Lo-Fi", "Moodboard", "Style Guide",
    "Brand Identity", "Icon", "Illustration", "Viewport", "Breakpoint",
];

const CONSULTING_TERMS: &[&str] = &[
    "Stakeholder", "Deliverable", "Workshop", "Discovery", "Proposal",
    "Pitch", "Engagement", "Client", "Retainer", "Billable", "Milestone",
    "Scope", "Out-of-Scope", "Assumption", "Risk", "Mitigation",
    "Dependency", "Forecast", "Revenue", "Margin", "Onsite", "Offsite",
    "Kick-off", "Steerco", "Executive Summary", "Follow-up",
];

/// Resolve a set of category IDs to a merged, deduplicated term list.
///
/// Unknown category IDs are silently skipped so a stale frontend enum
/// doesn't corrupt the dictionary with phantom entries. Deduplication is
/// case-insensitive so "Commit" and "commit" don't both land in the list.
pub fn seeds_for(categories: &[&str]) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for cat in categories {
        let terms = match *cat {
            "dev" => DEV_TERMS,
            "pm" => PM_TERMS,
            "content" => CONTENT_TERMS,
            "design" => DESIGN_TERMS,
            "consulting" => CONSULTING_TERMS,
            _ => continue,
        };
        for term in terms {
            let key = term.to_lowercase();
            if seen.insert(key) {
                out.push((*term).to_owned());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_categories_returns_empty_vec() {
        assert!(seeds_for(&[]).is_empty());
    }

    #[test]
    fn single_category_returns_its_full_list() {
        let result = seeds_for(&["dev"]);
        assert_eq!(result.len(), DEV_TERMS.len());
        assert!(result.contains(&"For-Schleife".to_owned()));
        assert!(result.contains(&"Pull Request".to_owned()));
    }

    #[test]
    fn unknown_category_is_silently_skipped() {
        // Frontend drift or a deleted category must not corrupt output.
        let result = seeds_for(&["doctor", "nonexistent"]);
        assert!(result.is_empty());
    }

    #[test]
    fn unknown_and_known_category_mix_only_yields_known_terms() {
        let result = seeds_for(&["dev", "zzz-bogus"]);
        assert_eq!(result.len(), DEV_TERMS.len());
    }

    #[test]
    fn multiple_categories_dedup_on_case_insensitive_key() {
        // "Milestone", "Stakeholder", "Deliverable", "Scope", "Kick-off"
        // appear in both PM and Consulting lists. Merged list must be
        // shorter than the naive concatenation.
        let merged = seeds_for(&["pm", "consulting"]);
        let naive = PM_TERMS.len() + CONSULTING_TERMS.len();
        assert!(
            merged.len() < naive,
            "expected dedup but got merged={} naive={}",
            merged.len(),
            naive,
        );
        // Each overlapping term should appear exactly once.
        let count_milestone = merged.iter().filter(|w| w.eq_ignore_ascii_case("milestone")).count();
        assert_eq!(count_milestone, 1);
    }

    #[test]
    fn all_five_categories_produce_nonempty_output() {
        let result = seeds_for(&["dev", "pm", "content", "design", "consulting"]);
        assert!(result.len() > 100, "expected a substantial merged list, got {}", result.len());
    }

    #[test]
    fn output_order_follows_category_order() {
        // The first category's terms should appear before the second's when
        // there is no overlap on the first term.
        let result = seeds_for(&["dev", "content"]);
        let dev_idx = result.iter().position(|w| w == "For-Schleife").unwrap();
        let content_idx = result.iter().position(|w| w == "SEO").unwrap();
        assert!(dev_idx < content_idx);
    }
}
