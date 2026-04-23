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
        .map(|s| strip_meta_commentary(s.trim()))
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
        .map(|s| strip_meta_commentary(s.trim()))
        .ok_or_else(|| "AI_ENHANCEMENT_FAILED: unexpected response shape".into())
}

fn wrap_transcript(text: &str) -> String {
    format!("<transcript>\n{text}\n</transcript>")
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
}
