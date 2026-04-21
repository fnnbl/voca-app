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
    language: 'de' | 'en'
    autostart: boolean
    onboardingCompleted: boolean
    theme: 'light' | 'dark' | 'system'
    audioInputDevice: string | null
  }
}

export type AppState = 'idle' | 'recording' | 'processing' | 'inserting' | 'error'

export interface AppError {
  code: string
  message: string
}

export const DEFAULT_SHORTCUT = 'Ctrl+Super'
