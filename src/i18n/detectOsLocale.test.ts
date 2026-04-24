import { describe, expect, it } from 'vitest'
import { detectOsLocale } from './detectOsLocale'

describe('detectOsLocale', () => {
  it('maps exact supported codes to themselves', () => {
    expect(detectOsLocale('de')).toBe('de')
    expect(detectOsLocale('en')).toBe('en')
    expect(detectOsLocale('es')).toBe('es')
    expect(detectOsLocale('fr')).toBe('fr')
    expect(detectOsLocale('pt')).toBe('pt')
    expect(detectOsLocale('it')).toBe('it')
  })

  it('strips region tags to the language prefix', () => {
    expect(detectOsLocale('en-US')).toBe('en')
    expect(detectOsLocale('de-AT')).toBe('de')
    expect(detectOsLocale('es-MX')).toBe('es')
    expect(detectOsLocale('pt-BR')).toBe('pt')
    expect(detectOsLocale('fr-CA')).toBe('fr')
  })

  it('handles underscore-separated POSIX locales', () => {
    expect(detectOsLocale('de_DE.UTF-8')).toBe('de')
    expect(detectOsLocale('it_IT')).toBe('it')
  })

  it('is case-insensitive', () => {
    expect(detectOsLocale('DE')).toBe('de')
    expect(detectOsLocale('Fr-FR')).toBe('fr')
  })

  it('falls back to English for unsupported locales', () => {
    expect(detectOsLocale('ja-JP')).toBe('en')
    expect(detectOsLocale('zh-CN')).toBe('en')
    expect(detectOsLocale('ru')).toBe('en')
    expect(detectOsLocale('nl-NL')).toBe('en')
  })

  it('falls back to English for empty or missing input', () => {
    expect(detectOsLocale('')).toBe('en')
    expect(detectOsLocale(null)).toBe('en')
    expect(detectOsLocale(undefined)).toBe('en')
  })
})
