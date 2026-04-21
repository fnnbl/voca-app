// Maps raw process names (Windows) and bundle identifiers (macOS) to
// human-readable labels. Lookup is case-insensitive, `.exe` is stripped.
// Unknown inputs pass through with `.exe` stripped so the UI still shows
// something sensible.

const KNOWN_APPS: &[(&str, &str)] = &[
    // Chat / messaging
    ("slack", "Slack"),
    ("com.tinyspeck.slackmacgap", "Slack"),
    ("discord", "Discord"),
    ("com.hnc.discord", "Discord"),
    ("telegram", "Telegram"),
    ("ru.keepcoder.telegram", "Telegram"),
    ("whatsapp", "WhatsApp"),
    ("net.whatsapp.whatsapp", "WhatsApp"),
    ("teams", "Teams"),
    ("ms-teams", "Teams"),
    ("com.microsoft.teams", "Teams"),
    ("com.microsoft.teams2", "Teams"),
    ("messages", "Messages"),
    ("com.apple.mobilesms", "Messages"),
    // Browsers
    ("chrome", "Chrome"),
    ("google chrome", "Chrome"),
    ("com.google.chrome", "Chrome"),
    ("firefox", "Firefox"),
    ("org.mozilla.firefox", "Firefox"),
    ("msedge", "Edge"),
    ("com.microsoft.edge", "Edge"),
    ("brave", "Brave"),
    ("com.brave.browser", "Brave"),
    ("arc", "Arc"),
    ("company.thebrowser.browser", "Arc"),
    ("safari", "Safari"),
    ("com.apple.safari", "Safari"),
    // Editors
    ("code", "VS Code"),
    ("visual studio code", "VS Code"),
    ("com.microsoft.vscode", "VS Code"),
    ("cursor", "Cursor"),
    ("com.todesktop.230313mzl4w4u92", "Cursor"),
    // Notes
    ("notion", "Notion"),
    ("notion.id", "Notion"),
    ("obsidian", "Obsidian"),
    ("md.obsidian", "Obsidian"),
    // Mail / calendar
    ("outlook", "Outlook"),
    ("com.microsoft.outlook", "Outlook"),
    ("mail", "Mail"),
    ("com.apple.mail", "Mail"),
    // System / productivity
    ("explorer", "Explorer"),
    ("finder", "Finder"),
    ("com.apple.finder", "Finder"),
    ("terminal", "Terminal"),
    ("com.apple.terminal", "Terminal"),
    ("iterm2", "iTerm"),
    ("com.googlecode.iterm2", "iTerm"),
    // Meetings
    ("zoom", "Zoom"),
    ("us.zoom.xos", "Zoom"),
];

pub fn pretty_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lowered = trimmed.to_lowercase();
    let lookup_key = lowered.strip_suffix(".exe").unwrap_or(&lowered);

    for (needle, label) in KNOWN_APPS {
        if lookup_key == *needle {
            return (*label).to_string();
        }
    }

    // Fallback: keep the raw casing but strip the .exe suffix so users don't
    // see "MyApp.exe" in their stats.
    let suffix_len = trimmed.len().saturating_sub(lookup_key.len());
    if suffix_len > 0 && lookup_key.len() < trimmed.len() {
        trimmed[..trimmed.len() - suffix_len].to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_label_strips_exe_and_matches_known_app() {
        assert_eq!(pretty_label("chrome.exe"), "Chrome");
        assert_eq!(pretty_label("Firefox.exe"), "Firefox");
        assert_eq!(pretty_label("Slack.exe"), "Slack");
    }

    #[test]
    fn pretty_label_case_insensitive_exe_suffix() {
        assert_eq!(pretty_label("Slack.EXE"), "Slack");
    }

    #[test]
    fn pretty_label_matches_macos_bundle_ids() {
        assert_eq!(pretty_label("com.tinyspeck.slackmacgap"), "Slack");
        assert_eq!(pretty_label("md.obsidian"), "Obsidian");
        assert_eq!(pretty_label("com.apple.safari"), "Safari");
    }

    #[test]
    fn pretty_label_bundle_id_case_insensitive() {
        assert_eq!(pretty_label("com.Apple.Safari"), "Safari");
    }

    #[test]
    fn pretty_label_falls_back_to_raw_without_exe_when_unknown() {
        assert_eq!(pretty_label("MyCustomApp.exe"), "MyCustomApp");
    }

    #[test]
    fn pretty_label_unknown_bundle_id_passes_through_verbatim() {
        assert_eq!(pretty_label("com.example.weirdapp"), "com.example.weirdapp");
    }

    #[test]
    fn pretty_label_empty_input_returns_empty() {
        assert_eq!(pretty_label(""), "");
        assert_eq!(pretty_label("   "), "");
    }

    #[test]
    fn pretty_label_trims_whitespace() {
        assert_eq!(pretty_label("  chrome.exe  "), "Chrome");
    }

    #[test]
    fn pretty_label_vscode_variants() {
        assert_eq!(pretty_label("Code.exe"), "VS Code");
        assert_eq!(pretty_label("com.microsoft.VSCode"), "VS Code");
    }

    #[test]
    fn pretty_label_teams_variants() {
        assert_eq!(pretty_label("Teams.exe"), "Teams");
        assert_eq!(pretty_label("ms-teams.exe"), "Teams");
        assert_eq!(pretty_label("com.microsoft.teams2"), "Teams");
    }
}
