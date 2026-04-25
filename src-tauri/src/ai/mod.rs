use regex::Regex;
use std::sync::OnceLock;

pub(crate) fn base_url(provider: &str, custom_endpoint: Option<&str>) -> String {
    match provider {
        "openai" => "https://api.openai.com/v1".into(),
        "groq" => "https://api.groq.com/openai/v1".into(),
        "cerebras" => "https://api.cerebras.ai/v1".into(),
        "mistral" => "https://api.mistral.ai/v1".into(),
        "openrouter" => "https://openrouter.ai/api/v1".into(),
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai".into(),
        "ollama" => custom_endpoint.unwrap_or("http://localhost:11434").trim_end_matches('/').to_string() + "/v1",
        "custom" => custom_endpoint.unwrap_or("").trim_end_matches('/').to_string() + "/v1",
        _ => "https://api.openai.com/v1".into(),
    }
}

pub async fn enhance(
    text: &str,
    system_prompt: &str,
    provider: &str,
    model: &str,
    api_key: &str,
    custom_endpoint: Option<&str>,
) -> Result<String, String> {
    if provider == "anthropic" {
        enhance_anthropic(text, system_prompt, model, api_key).await
    } else {
        let url = base_url(provider, custom_endpoint);
        enhance_openai_compat(text, system_prompt, model, api_key, &url).await
    }
}

async fn enhance_openai_compat(
    text: &str,
    system_prompt: &str,
    model: &str,
    api_key: &str,
    base: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": wrap_transcript(text)}
        ]
    });

    let client = reqwest::Client::new();
    let mut req = client
        .post(format!("{base}/chat/completions"))
        .json(&body);

    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }

    let response = req
        .send()
        .await
        .map_err(|e| format!("AI_ENHANCEMENT_FAILED: {e}"))?;

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("AI_KEY_INVALID: 401 Unauthorized".into());
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("AI_ENHANCEMENT_FAILED: HTTP {status} – {body}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("AI_ENHANCEMENT_FAILED: {e}"))?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(post_process_output)
        .ok_or_else(|| "AI_ENHANCEMENT_FAILED: unexpected response shape".into())
}

async fn enhance_anthropic(
    text: &str,
    system_prompt: &str,
    model: &str,
    api_key: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{"role": "user", "content": wrap_transcript(text)}]
    });

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI_ENHANCEMENT_FAILED: {e}"))?;

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("AI_KEY_INVALID: 401 Unauthorized".into());
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("AI_ENHANCEMENT_FAILED: HTTP {status} – {body}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("AI_ENHANCEMENT_FAILED: {e}"))?;

    json["content"][0]["text"]
        .as_str()
        .map(post_process_output)
        .ok_or_else(|| "AI_ENHANCEMENT_FAILED: unexpected response shape".into())
}

fn wrap_transcript(text: &str) -> String {
    format!("<transcript>\n{text}\n</transcript>")
}

/// Chain all LLM-output cleanup steps that don't depend on the caller.
/// Order matters and the paren-strip runs twice on purpose:
/// - meta first so any `\n\n"I made the following changes"` block is
///   removed before downstream regexes need to anchor on `$`.
/// - paren-strip first pass catches a trailing `(No changes…)` that sits
///   at the very end so the close-tag regex can then anchor on `</transcript>`.
/// - tag-strip handles wrapper leaks at start and end.
/// - paren-strip second pass catches the rarer shape where `(commentary)`
///   was *between* content and close tag, exposed only after tag-strip.
fn post_process_output(raw: &str) -> String {
    let after_meta = strip_meta_commentary(raw.trim());
    let after_paren_a = strip_trailing_paren_commentary(&after_meta);
    let after_tags = strip_transcript_tags(&after_paren_a);
    strip_trailing_paren_commentary(&after_tags)
}

/// Remove a leaked `<transcript>` opening tag at the very start of the
/// output and/or a `</transcript>` closing tag at the very end. The
/// default AI-enhancement prompt wraps the raw input in these tags and
/// instructs the model not to echo them, but Llama 3.3 70B (our free
/// default on Groq) occasionally leaks them anyway. Boundary-anchored
/// so a transcript that legitimately mentions the word `<transcript>`
/// in the middle stays intact.
fn strip_transcript_tags(text: &str) -> String {
    let open_re = open_tag_regex();
    let close_re = close_tag_regex();
    let after_open = open_re.replace(text, "");
    close_re.replace(&after_open, "").into_owned()
}

fn open_tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*<transcript\b[^>]*>\s*").expect("open-tag regex"))
}

fn close_tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\s*</transcript\s*>\s*$").expect("close-tag regex"))
}

/// Remove a trailing parenthetical commentary like
/// `(No changes made — only English technical terms)` that some models
/// like to append after the cleaned transcript. Different leak shape
/// from the `\n\n`-anchored "I made the following changes" block —
/// here the model packs its commentary into a final parenthetical
/// without a paragraph break. Boundary-anchored on `$` and only fires
/// when the paren content starts with a known commentary marker, so
/// legitimate trailing parentheticals in the user's actual transcript
/// (`…wir telefonieren später (oder morgen)`) survive.
fn strip_trailing_paren_commentary(text: &str) -> String {
    let re = trailing_paren_commentary_regex();
    re.replace(text, "").into_owned()
}

fn trailing_paren_commentary_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Anchored at end of string. Matches a trailing parenthetical
        // whose content starts with a multilingual commentary marker:
        // - EN: "no change(s)", "no modification(s)", "note", "i ", "kept"
        // - DE: "keine änderung(en)", "hinweis", "anmerkung"
        // - ES: "sin cambios", "sin modificaciones", "nota"
        // - FR: "aucun(e) changement", "aucune modification", "note"
        // - PT: "sem alterações", "sem modificações", "nota"
        // - IT: "nessuna modifica", "nessun cambiamento", "nota"
        Regex::new(r#"(?i)\s*\(\s*(?:no\s+changes?|no\s+modifications?|note\b|i\s+(?:made|kept|left|did)|kept\s+|keine\s+änderung\w*|hinweis|anmerkung|sin\s+cambios|sin\s+modificacion\w*|nota\b|aucune?\s+(?:changement|modification)\w*|sem\s+altera\w*|sem\s+modifica\w*|nessuna\s+modifica\w*|nessun\s+cambiamento\w*)[^)]*\)\s*$"#)
            .expect("trailing-paren-commentary regex")
    })
}

const META_COMMENTARY_TRIGGERS: &[&str] = &[
    // German
    "ich habe folgende änderungen",
    "ich habe die folgenden änderungen",
    "folgende änderungen wurden",
    "folgende änderungen habe ich",
    "änderungen:",
    // English
    "i made the following changes",
    "here are the changes",
    "the following changes were",
    "changes made:",
    "summary of changes:",
];

fn strip_meta_commentary(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut earliest: Option<usize> = None;
    for trigger in META_COMMENTARY_TRIGGERS {
        if let Some(pos) = lower.find(&format!("\n\n{trigger}")) {
            earliest = Some(earliest.map_or(pos, |e| e.min(pos)));
        }
    }
    match earliest {
        Some(0) | None => text.to_owned(),
        Some(pos) => text[..pos].trim_end().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_providers_return_correct_base_urls() {
        assert_eq!(base_url("openai", None), "https://api.openai.com/v1");
        assert_eq!(base_url("groq", None), "https://api.groq.com/openai/v1");
        assert_eq!(base_url("cerebras", None), "https://api.cerebras.ai/v1");
        assert_eq!(base_url("mistral", None), "https://api.mistral.ai/v1");
        assert_eq!(base_url("openrouter", None), "https://openrouter.ai/api/v1");
        assert_eq!(base_url("gemini", None), "https://generativelanguage.googleapis.com/v1beta/openai");
    }

    #[test]
    fn unknown_provider_falls_back_to_openai() {
        assert_eq!(base_url("unknown_provider", None), "https://api.openai.com/v1");
        assert_eq!(base_url("", None), "https://api.openai.com/v1");
    }

    #[test]
    fn ollama_uses_custom_endpoint_with_v1_suffix() {
        assert_eq!(base_url("ollama", Some("http://localhost:11434")), "http://localhost:11434/v1");
    }

    #[test]
    fn ollama_default_when_no_endpoint() {
        assert_eq!(base_url("ollama", None), "http://localhost:11434/v1");
    }

    #[test]
    fn ollama_strips_trailing_slash_before_appending_v1() {
        assert_eq!(base_url("ollama", Some("http://localhost:11434/")), "http://localhost:11434/v1");
    }

    #[test]
    fn custom_uses_provided_endpoint() {
        assert_eq!(base_url("custom", Some("https://my-api.example.com")), "https://my-api.example.com/v1");
    }

    #[test]
    fn custom_strips_trailing_slash() {
        assert_eq!(base_url("custom", Some("https://my-api.example.com/")), "https://my-api.example.com/v1");
    }

    #[test]
    fn custom_empty_endpoint_produces_slash_v1() {
        assert_eq!(base_url("custom", Some("")), "/v1");
    }

    // ── wrap_transcript ────────────────────────────────────────────────────────

    #[test]
    fn wrap_transcript_adds_xml_tags() {
        assert_eq!(
            wrap_transcript("hello world"),
            "<transcript>\nhello world\n</transcript>"
        );
    }

    #[test]
    fn wrap_transcript_preserves_internal_newlines() {
        assert_eq!(
            wrap_transcript("line one\nline two"),
            "<transcript>\nline one\nline two\n</transcript>"
        );
    }

    // ── strip_meta_commentary ──────────────────────────────────────────────────

    #[test]
    fn strip_meta_commentary_passes_clean_text_through() {
        let input = "Der Algorithmus initialisiert die Variable.";
        assert_eq!(strip_meta_commentary(input), input);
    }

    #[test]
    fn strip_meta_commentary_removes_german_trailing_block() {
        let input = "Der Text ist bereinigt.\n\nIch habe die folgenden Änderungen vorgenommen:\n- Punctuation\n- Groß-/Kleinschreibung";
        assert_eq!(strip_meta_commentary(input), "Der Text ist bereinigt.");
    }

    #[test]
    fn strip_meta_commentary_removes_english_trailing_block() {
        let input = "The text is cleaned.\n\nI made the following changes:\n- Added punctuation\n- Fixed capitalization";
        assert_eq!(strip_meta_commentary(input), "The text is cleaned.");
    }

    #[test]
    fn strip_meta_commentary_is_case_insensitive() {
        let input = "Text.\n\nHere are the Changes:\n- one\n- two";
        assert_eq!(strip_meta_commentary(input), "Text.");
    }

    #[test]
    fn strip_meta_commentary_keeps_whole_text_when_trigger_at_start() {
        // If the entire output is a meta block, don't nuke it — that signals
        // a complete failure and the original text should be preserved for
        // the caller to handle.
        let input = "Ich habe folgende Änderungen vorgenommen:\n- foo";
        assert_eq!(strip_meta_commentary(input), input);
    }

    #[test]
    fn strip_meta_commentary_ignores_mid_sentence_trigger_words() {
        // Trigger phrase mid-text without the \n\n paragraph break should
        // not cause stripping.
        let input = "Wir reden über Änderungen: welche gut sind und welche nicht.";
        assert_eq!(strip_meta_commentary(input), input);
    }

    #[test]
    fn strip_meta_commentary_takes_earliest_trigger_when_multiple_present() {
        let input = "Text.\n\nHere are the changes:\n- a\n\nSummary of changes:\n- b";
        assert_eq!(strip_meta_commentary(input), "Text.");
    }

    #[test]
    fn strip_meta_commentary_trims_trailing_whitespace() {
        let input = "Text.   \n\nChanges made:\n- x";
        assert_eq!(strip_meta_commentary(input), "Text.");
    }

    #[test]
    fn strip_meta_commentary_handles_empty_string() {
        assert_eq!(strip_meta_commentary(""), "");
    }

    // ── strip_transcript_tags ──────────────────────────────────────────────────

    #[test]
    fn strip_transcript_tags_removes_both_wrapper_tags() {
        let leaked = "<transcript>\nDer Algorithmus initialisiert die Variable.\n</transcript>";
        assert_eq!(
            strip_transcript_tags(leaked),
            "Der Algorithmus initialisiert die Variable."
        );
    }

    #[test]
    fn strip_transcript_tags_removes_opening_only() {
        let leaked = "<transcript>\nHello world.";
        assert_eq!(strip_transcript_tags(leaked), "Hello world.");
    }

    #[test]
    fn strip_transcript_tags_removes_closing_only() {
        let leaked = "Hello world.\n</transcript>";
        assert_eq!(strip_transcript_tags(leaked), "Hello world.");
    }

    #[test]
    fn strip_transcript_tags_passes_clean_output_through() {
        let clean = "Hello world.";
        assert_eq!(strip_transcript_tags(clean), clean);
    }

    #[test]
    fn strip_transcript_tags_is_case_insensitive() {
        let leaked = "<TRANSCRIPT>\nHello.\n</Transcript>";
        assert_eq!(strip_transcript_tags(leaked), "Hello.");
    }

    #[test]
    fn strip_transcript_tags_tolerates_attributes_on_opening_tag() {
        let leaked = "<transcript id=\"x\">\nHello.\n</transcript>";
        assert_eq!(strip_transcript_tags(leaked), "Hello.");
    }

    #[test]
    fn strip_transcript_tags_only_affects_boundaries_not_mid_text() {
        // If a user's transcript legitimately talks about the wrapper tag
        // (unlikely but possible in a dev conversation), occurrences in the
        // middle must survive.
        let text = "I was editing <transcript> tags in the repo yesterday.";
        assert_eq!(strip_transcript_tags(text), text);
    }

    #[test]
    fn strip_transcript_tags_handles_empty_string() {
        assert_eq!(strip_transcript_tags(""), "");
    }

    #[test]
    fn strip_transcript_tags_collapses_whitespace_between_content_and_tags() {
        let leaked = "<transcript>   \n\n  Hello.  \n\n   </transcript>";
        assert_eq!(strip_transcript_tags(leaked), "Hello.");
    }

    // ── post_process_output ────────────────────────────────────────────────────

    #[test]
    fn post_process_output_combines_strips_in_right_order() {
        // Worst-case leak: wrapper tags AND trailing meta commentary.
        let leaked = "<transcript>\nDer Text ist bereinigt.\n</transcript>\n\nIch habe die folgenden Änderungen vorgenommen:\n- Komma\n- Punkt";
        assert_eq!(post_process_output(leaked), "Der Text ist bereinigt.");
    }

    // ── strip_trailing_paren_commentary ────────────────────────────────────────

    #[test]
    fn strip_trailing_paren_strips_no_changes_made() {
        // The exact leak shape Fynn caught after the prompt-softening
        // landed: model packs "no changes" commentary into a trailing
        // parenthetical instead of a separate "\n\nI made..." block.
        let leaked = "Develop, auschecken, fetchen und pullen. (No changes made - only English technical terms, \"Fetchen\" to \"Fetchen\" is okay as it's Denglisch, Auschecken = checkout, Pullen = pull)";
        assert_eq!(
            strip_trailing_paren_commentary(leaked),
            "Develop, auschecken, fetchen und pullen."
        );
    }

    #[test]
    fn strip_trailing_paren_strips_note_marker() {
        let leaked = "Der Termin ist morgen. (Note: I added the period.)";
        assert_eq!(
            strip_trailing_paren_commentary(leaked),
            "Der Termin ist morgen."
        );
    }

    #[test]
    fn strip_trailing_paren_strips_german_keine_aenderungen() {
        let leaked = "Wir telefonieren morgen. (Keine Änderungen vorgenommen.)";
        assert_eq!(
            strip_trailing_paren_commentary(leaked),
            "Wir telefonieren morgen."
        );
    }

    #[test]
    fn strip_trailing_paren_strips_french_aucune_modification() {
        let leaked = "On se rappelle demain. (Aucune modification nécessaire.)";
        assert_eq!(
            strip_trailing_paren_commentary(leaked),
            "On se rappelle demain."
        );
    }

    #[test]
    fn strip_trailing_paren_strips_spanish_sin_cambios() {
        let leaked = "Te llamo mañana. (Sin cambios — el texto ya estaba limpio.)";
        assert_eq!(
            strip_trailing_paren_commentary(leaked),
            "Te llamo mañana."
        );
    }

    #[test]
    fn strip_trailing_paren_strips_i_kept_form() {
        let leaked = "Hello world. (I kept the original spelling.)";
        assert_eq!(
            strip_trailing_paren_commentary(leaked),
            "Hello world."
        );
    }

    #[test]
    fn strip_trailing_paren_preserves_legitimate_trailing_paren() {
        // A user might dictate a parenthetical at the end of their
        // actual sentence — those must survive untouched.
        let text = "Wir telefonieren später (oder morgen).";
        assert_eq!(strip_trailing_paren_commentary(text), text);
    }

    #[test]
    fn strip_trailing_paren_preserves_legitimate_aside() {
        let text = "I'll call Max (or maybe Tom).";
        assert_eq!(strip_trailing_paren_commentary(text), text);
    }

    #[test]
    fn strip_trailing_paren_preserves_mid_text_parenthetical() {
        // Even if the parenthetical contains commentary-like keywords,
        // it has to be at the end. A mid-text occurrence is part of
        // the user's transcript.
        let text = "I added the note (no changes were needed) and moved on.";
        assert_eq!(strip_trailing_paren_commentary(text), text);
    }

    #[test]
    fn strip_trailing_paren_passes_clean_text_through() {
        let clean = "Just a normal sentence with nothing in parens.";
        assert_eq!(strip_trailing_paren_commentary(clean), clean);
    }

    #[test]
    fn strip_trailing_paren_handles_empty_string() {
        assert_eq!(strip_trailing_paren_commentary(""), "");
    }

    #[test]
    fn post_process_output_handles_combined_tag_and_paren_leak() {
        // Both leak shapes at once: opening transcript tag AND trailing
        // parenthetical commentary. Both have to be stripped.
        let leaked = "<transcript>\nDevelop, auschecken, fetchen und pullen.\n</transcript> (No changes made - already clean.)";
        assert_eq!(
            post_process_output(leaked),
            "Develop, auschecken, fetchen und pullen."
        );
    }

    #[test]
    fn post_process_output_trims_leading_and_trailing_whitespace() {
        let input = "   Hello world.   ";
        assert_eq!(post_process_output(input), "Hello world.");
    }

    #[test]
    fn post_process_output_is_idempotent_on_clean_text() {
        let clean = "Already clean output.";
        assert_eq!(post_process_output(clean), clean);
    }
}
