import { SUPPORTED_UI_LANGUAGES, type UiLanguage } from '../types'

const FALLBACK: UiLanguage = 'en'

export function detectOsLocale(raw?: string | null): UiLanguage {
  const source = raw ?? (typeof navigator !== 'undefined' ? navigator.language : '')
  if (!source) return FALLBACK
  const lower = source.toLowerCase()
  const short = lower.split(/[-_]/, 1)[0] as UiLanguage
  if ((SUPPORTED_UI_LANGUAGES as string[]).includes(short)) return short
  return FALLBACK
}
