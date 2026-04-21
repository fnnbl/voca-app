use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const DEFAULT_PROMPT_ID: &str = "default";
const DEFAULT_PROMPT_TEXT: &str =
    "You are editing a raw voice transcription. Clean it up: add punctuation and capitalization, fix obvious speech recognition errors, remove filler words (um, uh, äh, also, halt, sozusagen), and handle self-corrections — if the speaker makes a mistake and corrects themselves, keep only the corrected version and remove the false start and any apology. Return only the cleaned text, no explanations.";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPrompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub is_default: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub trigger: String,
    pub output: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub id: String,
    pub word: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp_ms: u64,
    pub text: String,
    pub enhanced: bool,
    pub duration_secs: f32,
    pub word_count: u32,
    pub provider: String,
}

// ── Path helpers ───────────────────────────────────────────────────────────────

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|p| p.join("settings.json")).map_err(|e| format!("STORAGE_ERROR: {e}"))
}
fn prompts_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|p| p.join("prompts.json")).map_err(|e| format!("STORAGE_ERROR: {e}"))
}
fn snippets_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|p| p.join("snippets.json")).map_err(|e| format!("STORAGE_ERROR: {e}"))
}
fn dictionary_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|p| p.join("dictionary.json")).map_err(|e| format!("STORAGE_ERROR: {e}"))
}
fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|p| p.join("history.json")).map_err(|e| format!("STORAGE_ERROR: {e}"))
}

// ── AppHandle-based public API (thin wrappers) ─────────────────────────────────

pub fn load(app: &AppHandle) -> Result<serde_json::Value, String> {
    load_from_path(&settings_path(app)?)
}
pub fn save(app: &AppHandle, settings: &serde_json::Value) -> Result<(), String> {
    save_to_path(&settings_path(app)?, settings)
}
pub fn load_prompts(app: &AppHandle) -> Result<Vec<AiPrompt>, String> {
    load_prompts_from_path(&prompts_path(app)?)
}
pub fn save_prompts(app: &AppHandle, prompts: &[AiPrompt]) -> Result<(), String> {
    save_to_path_raw(&prompts_path(app)?, prompts)
}
pub fn load_snippets(app: &AppHandle) -> Result<Vec<Snippet>, String> {
    load_vec_from_path(&snippets_path(app)?)
}
pub fn save_snippets(app: &AppHandle, snippets: &[Snippet]) -> Result<(), String> {
    save_to_path_raw(&snippets_path(app)?, snippets)
}
pub fn load_dictionary(app: &AppHandle) -> Result<Vec<DictionaryEntry>, String> {
    load_vec_from_path(&dictionary_path(app)?)
}
pub fn save_dictionary(app: &AppHandle, entries: &[DictionaryEntry]) -> Result<(), String> {
    save_to_path_raw(&dictionary_path(app)?, entries)
}
pub fn load_history(app: &AppHandle) -> Result<Vec<HistoryEntry>, String> {
    load_vec_from_path(&history_path(app)?)
}
pub fn append_history_entry(app: &AppHandle, entry: HistoryEntry) -> Result<(), String> {
    let path = history_path(app)?;
    let mut entries: Vec<HistoryEntry> = load_vec_from_path(&path)?;
    entries.push(entry);
    if entries.len() > 500 {
        entries.drain(0..entries.len() - 500);
    }
    save_to_path_raw(&path, &entries)
}
pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    save_to_path_raw(&history_path(app)?, &[] as &[HistoryEntry])
}

// ── Path-based core functions (testable) ──────────────────────────────────────

pub(crate) fn load_from_path(path: &Path) -> Result<serde_json::Value, String> {
    if !path.exists() {
        return Ok(defaults());
    }
    let content = std::fs::read_to_string(path).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("STORAGE_ERROR: {e}"))
}

pub(crate) fn save_to_path(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    ensure_parent(path)?;
    let content = serde_json::to_string_pretty(value).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    std::fs::write(path, content).map_err(|e| format!("STORAGE_ERROR: {e}"))
}

pub(crate) fn load_prompts_from_path(path: &Path) -> Result<Vec<AiPrompt>, String> {
    if !path.exists() {
        return Ok(default_prompts());
    }
    let content = std::fs::read_to_string(path).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    let mut prompts: Vec<AiPrompt> =
        serde_json::from_str(&content).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    if !prompts.iter().any(|p| p.id == DEFAULT_PROMPT_ID) {
        prompts.insert(0, default_prompt());
    }
    Ok(prompts)
}

pub(crate) fn load_vec_from_path<T>(path: &Path) -> Result<Vec<T>, String>
where
    T: serde::de::DeserializeOwned,
{
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("STORAGE_ERROR: {e}"))
}

pub(crate) fn save_to_path_raw<T: serde::Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    ensure_parent(path)?;
    let content = serde_json::to_string_pretty(value).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    std::fs::write(path, content).map_err(|e| format!("STORAGE_ERROR: {e}"))
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    }
    Ok(())
}

// ── Defaults ───────────────────────────────────────────────────────────────────

pub(crate) fn defaults() -> serde_json::Value {
    serde_json::json!({
        "transcription": {
            "mode": "cloud",
            "localModelSize": "base",
            "cloudProvider": "groq",
            "cloudModel": "",
            "cloudCustomEndpoint": ""
        },
        "aiEnhancement": {
            "enabled": false,
            "provider": "groq",
            "model": "llama-3.1-8b-instant",
            "customEndpoint": "",
            "activePromptId": "",
            "skipShortTranscriptions": true,
            "minWords": 3
        },
        "shortcuts": {
            "key": "Ctrl+Super"
        },
        "general": {
            "language": "de",
            "autostart": false,
            "onboardingCompleted": false,
            "theme": "system",
            "audioInputDevice": null
        }
    })
}

fn default_prompt() -> AiPrompt {
    AiPrompt {
        id: DEFAULT_PROMPT_ID.into(),
        name: "Default".into(),
        prompt: DEFAULT_PROMPT_TEXT.into(),
        is_default: true,
        created_at: "2024-01-01T00:00:00Z".into(),
    }
}

fn default_prompts() -> Vec<AiPrompt> {
    vec![default_prompt()]
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    // ── defaults ──────────────────────────────────────────────────────────────

    #[test]
    fn defaults_has_required_sections() {
        let d = defaults();
        assert!(d["transcription"].is_object());
        assert!(d["aiEnhancement"].is_object());
        assert!(d["shortcuts"].is_object());
        assert!(d["general"].is_object());
    }

    #[test]
    fn defaults_transcription_mode_is_cloud() {
        assert_eq!(defaults()["transcription"]["mode"], "cloud");
    }

    #[test]
    fn defaults_onboarding_not_completed() {
        assert_eq!(defaults()["general"]["onboardingCompleted"], false);
    }

    #[test]
    fn defaults_ai_enhancement_disabled() {
        assert_eq!(defaults()["aiEnhancement"]["enabled"], false);
    }

    // ── load_from_path ────────────────────────────────────────────────────────

    #[test]
    fn load_returns_defaults_when_file_missing() {
        let dir = tmp();
        let path = dir.path().join("settings.json");
        let result = load_from_path(&path).unwrap();
        assert_eq!(result["transcription"]["mode"], "cloud");
    }

    #[test]
    fn load_and_save_roundtrip() {
        let dir = tmp();
        let path = dir.path().join("settings.json");
        let mut val = defaults();
        val["general"]["language"] = serde_json::json!("en");
        save_to_path(&path, &val).unwrap();
        let loaded = load_from_path(&path).unwrap();
        assert_eq!(loaded["general"]["language"], "en");
    }

    #[test]
    fn load_returns_error_on_invalid_json() {
        let dir = tmp();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not valid json {{").unwrap();
        assert!(load_from_path(&path).is_err());
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tmp();
        let path = dir.path().join("nested").join("deep").join("settings.json");
        let val = defaults();
        save_to_path(&path, &val).unwrap();
        assert!(path.exists());
    }

    // ── load_prompts_from_path ────────────────────────────────────────────────

    #[test]
    fn load_prompts_returns_default_when_missing() {
        let dir = tmp();
        let path = dir.path().join("prompts.json");
        let prompts = load_prompts_from_path(&path).unwrap();
        assert!(!prompts.is_empty());
        assert_eq!(prompts[0].id, "default");
    }

    #[test]
    fn load_prompts_injects_default_if_absent() {
        let dir = tmp();
        let path = dir.path().join("prompts.json");
        let custom = vec![AiPrompt {
            id: "custom".into(),
            name: "Custom".into(),
            prompt: "Do something".into(),
            is_default: false,
            created_at: "2025-01-01T00:00:00Z".into(),
        }];
        save_to_path_raw(&path, &custom).unwrap();
        let prompts = load_prompts_from_path(&path).unwrap();
        assert!(prompts.iter().any(|p| p.id == "default"));
        assert!(prompts.iter().any(|p| p.id == "custom"));
    }

    #[test]
    fn load_prompts_does_not_duplicate_default() {
        let dir = tmp();
        let path = dir.path().join("prompts.json");
        let with_default = vec![AiPrompt {
            id: "default".into(),
            name: "Default".into(),
            prompt: "custom text".into(),
            is_default: true,
            created_at: "".into(),
        }];
        save_to_path_raw(&path, &with_default).unwrap();
        let prompts = load_prompts_from_path(&path).unwrap();
        assert_eq!(prompts.iter().filter(|p| p.id == "default").count(), 1);
    }

    #[test]
    fn load_prompts_returns_error_on_invalid_json() {
        let dir = tmp();
        let path = dir.path().join("prompts.json");
        std::fs::write(&path, "[invalid json").unwrap();
        assert!(load_prompts_from_path(&path).is_err());
    }

    // ── load_vec_from_path (snippets / dictionary) ────────────────────────────

    #[test]
    fn load_vec_returns_empty_when_file_missing() {
        let dir = tmp();
        let path = dir.path().join("snippets.json");
        let result: Result<Vec<Snippet>, _> = load_vec_from_path(&path);
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn snippets_roundtrip() {
        let dir = tmp();
        let path = dir.path().join("snippets.json");
        let snippets = vec![Snippet {
            id: "1".into(),
            name: "Sig".into(),
            trigger: "sig".into(),
            output: "Best regards".into(),
            enabled: true,
            created_at: "2025-01-01T00:00:00Z".into(),
        }];
        save_to_path_raw(&path, &snippets).unwrap();
        let loaded: Vec<Snippet> = load_vec_from_path(&path).unwrap();
        assert_eq!(loaded[0].trigger, "sig");
        assert_eq!(loaded[0].output, "Best regards");
        assert!(loaded[0].enabled);
    }

    #[test]
    fn dictionary_roundtrip() {
        let dir = tmp();
        let path = dir.path().join("dict.json");
        let entries = vec![
            DictionaryEntry { id: "1".into(), word: "VOCA".into() },
            DictionaryEntry { id: "2".into(), word: "Tauri".into() },
        ];
        save_to_path_raw(&path, &entries).unwrap();
        let loaded: Vec<DictionaryEntry> = load_vec_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].word, "VOCA");
        assert_eq!(loaded[1].word, "Tauri");
    }

    #[test]
    fn load_vec_returns_error_on_invalid_json() {
        let dir = tmp();
        let path = dir.path().join("snippets.json");
        std::fs::write(&path, "{not an array}").unwrap();
        let result: Result<Vec<Snippet>, _> = load_vec_from_path(&path);
        assert!(result.is_err());
    }
}
