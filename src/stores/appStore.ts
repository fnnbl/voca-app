import { create } from 'zustand'
import type { AppState, AppError, Settings, UpdateInfo } from '../types'

interface AppStore {
  appState: AppState
  error: AppError | null
  settings: Settings | null
  lastTranscription: string | null
  updateAvailable: UpdateInfo | null
  setAppState: (state: AppState) => void
  setError: (error: AppError | null) => void
  setSettings: (settings: Settings) => void
  setLastTranscription: (text: string) => void
  setUpdateAvailable: (info: UpdateInfo | null) => void
}

export const useAppStore = create<AppStore>((set) => ({
  appState: 'idle',
  error: null,
  settings: null,
  lastTranscription: null,
  updateAvailable: null,
  setAppState: (appState) => set({ appState }),
  setError: (error) => set({ error }),
  setSettings: (settings) => set({ settings }),
  setLastTranscription: (text) => set({ lastTranscription: text }),
  setUpdateAvailable: (info) => set({ updateAvailable: info }),
}))
