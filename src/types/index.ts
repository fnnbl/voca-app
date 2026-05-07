export interface Snippet {
  id: string
  name: string
  trigger: string
  output: string
  enabled: boolean
  createdAt: string
}

export interface DictionaryEntry {
  id: string
  word: string
}

export interface FillerEntry {
  id: string
  word: string
}

export interface FillersFile {
  words: FillerEntry[]
  rejected: string[]
}

export interface AIPrompt {
  id: string
  name: string
  prompt: string
  isDefault: boolean
  createdAt: string
}

export interface Settings {
  transcription: {
    mode: 'cloud' | 'local'
    localModelSize: 'tiny' | 'base' | 'small' | 'medium'
    cloudProvider: 'openai' | 'groq' | 'deepgram' | 'elevenlabs' | 'gemini' | 'custom'
    cloudModel: string
    cloudCustomEndpoint: string
    language: TranscriptionLanguage
    removeFillerWords: boolean
    muteOtherAudio: boolean
  }
  aiEnhancement: {
    enabled: boolean
    provider: 'openai' | 'anthropic' | 'groq' | 'cerebras' | 'mistral' | 'openrouter' | 'gemini' | 'ollama' | 'custom'
    model: string
    customEndpoint: string
    activePromptId: string
    skipShortTranscriptions: boolean
    minWords: number
  }
  shortcuts: {
    key: string
  }
  general: {
    language: UiLanguage
    autostart: boolean
    onboardingCompleted: boolean
    theme: 'light' | 'dark' | 'system'
    audioInputDevice: string | null
  }
  privacy: {
    historyTracking: boolean
    targetAppTracking: boolean
  }
}

export type UiLanguage = 'de' | 'en' | 'es' | 'fr' | 'pt' | 'it'

export const SUPPORTED_UI_LANGUAGES: UiLanguage[] = ['de', 'en', 'es', 'fr', 'pt', 'it']

export type TranscriptionLanguage = 'auto' | 'de' | 'en' | 'es' | 'fr' | 'pt' | 'it'

export const TRANSCRIPTION_LANGUAGES: TranscriptionLanguage[] = [
  'auto',
  'de',
  'en',
  'es',
  'fr',
  'pt',
  'it',
]

export type AppState = 'idle' | 'recording' | 'processing' | 'inserting' | 'error'

export interface AppError {
  code: string
  message: string
}

export const DEFAULT_SHORTCUT = 'Ctrl+Super'
