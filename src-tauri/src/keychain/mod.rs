use keyring::Entry;

const SERVICE: &str = "voca";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyType {
    WhisperApiKey,
    AiEnhancementApiKey,
}

impl KeyType {
    fn account(&self) -> &'static str {
        match self {
            KeyType::WhisperApiKey => "whisper_api_key",
            KeyType::AiEnhancementApiKey => "ai_enhancement_api_key",
        }
    }
}

pub fn save(key_type: KeyType, value: &str) -> Result<(), String> {
    Entry::new(SERVICE, key_type.account())
        .and_then(|e| e.set_password(value))
        .map_err(|e| format!("KEYCHAIN_ERROR: {e}"))
}

pub fn get(key_type: KeyType) -> Result<Option<String>, String> {
    match Entry::new(SERVICE, key_type.account()).and_then(|e| e.get_password()) {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("KEYCHAIN_ERROR: {e}")),
    }
}

/// Transcription provider keys. OpenAI aliases to the existing whisper_api_key entry.
pub fn save_transcription_key(provider: &str, value: &str) -> Result<(), String> {
    let account = transcription_account(provider);
    Entry::new(SERVICE, &account)
        .and_then(|e| e.set_password(value))
        .map_err(|e| format!("KEYCHAIN_ERROR: {e}"))
}

pub fn get_transcription_key(provider: &str) -> Result<Option<String>, String> {
    let account = transcription_account(provider);
    match Entry::new(SERVICE, &account).and_then(|e| e.get_password()) {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("KEYCHAIN_ERROR: {e}")),
    }
}

pub fn delete_transcription_key(provider: &str) -> Result<(), String> {
    let account = transcription_account(provider);
    match Entry::new(SERVICE, &account).and_then(|e| e.delete_password()) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("KEYCHAIN_ERROR: {e}")),
    }
}

fn transcription_account(provider: &str) -> String {
    // OpenAI aliases to the pre-existing whisper_api_key for backward compatibility
    if provider == "openai" {
        "whisper_api_key".into()
    } else {
        format!("transcription_key_{provider}")
    }
}

pub fn save_ai_provider_key(provider: &str, value: &str) -> Result<(), String> {
    Entry::new(SERVICE, &format!("ai_key_{provider}"))
        .and_then(|e| e.set_password(value))
        .map_err(|e| format!("KEYCHAIN_ERROR: {e}"))
}

pub fn get_ai_provider_key(provider: &str) -> Result<Option<String>, String> {
    match Entry::new(SERVICE, &format!("ai_key_{provider}")).and_then(|e| e.get_password()) {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("KEYCHAIN_ERROR: {e}")),
    }
}

pub fn delete_ai_provider_key(provider: &str) -> Result<(), String> {
    match Entry::new(SERVICE, &format!("ai_key_{provider}")).and_then(|e| e.delete_password()) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("KEYCHAIN_ERROR: {e}")),
    }
}

pub fn delete(key_type: KeyType) -> Result<(), String> {
    match Entry::new(SERVICE, key_type.account()).and_then(|e| e.delete_password()) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("KEYCHAIN_ERROR: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_aliases_to_whisper_api_key() {
        assert_eq!(transcription_account("openai"), "whisper_api_key");
    }

    #[test]
    fn other_providers_get_prefixed_key() {
        assert_eq!(transcription_account("groq"), "transcription_key_groq");
        assert_eq!(transcription_account("deepgram"), "transcription_key_deepgram");
        assert_eq!(transcription_account("gemini"), "transcription_key_gemini");
    }

    #[test]
    fn key_type_accounts_are_stable() {
        assert_eq!(KeyType::WhisperApiKey.account(), "whisper_api_key");
        assert_eq!(KeyType::AiEnhancementApiKey.account(), "ai_enhancement_api_key");
    }
}
