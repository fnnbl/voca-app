import { describe, expect, it } from 'vitest'
import { formatShortcut, formatShortcutKey } from './format'

describe('formatShortcutKey', () => {
  it('renders Super as Win on non-mac', () => {
    expect(formatShortcutKey('Super', false)).toBe('Win')
  })

  it('renders Super as ⌘ on mac', () => {
    expect(formatShortcutKey('Super', true)).toBe('⌘')
  })

  it('renders Ctrl/Alt/Shift as text on non-mac', () => {
    expect(formatShortcutKey('Ctrl', false)).toBe('Ctrl')
    expect(formatShortcutKey('Alt', false)).toBe('Alt')
    expect(formatShortcutKey('Shift', false)).toBe('Shift')
  })

  it('renders Ctrl/Alt/Shift as Apple glyphs on mac', () => {
    expect(formatShortcutKey('Ctrl', true)).toBe('⌃')
    expect(formatShortcutKey('Alt', true)).toBe('⌥')
    expect(formatShortcutKey('Shift', true)).toBe('⇧')
  })

  it('passes non-modifier keys through', () => {
    expect(formatShortcutKey('Space', false)).toBe('Space')
    expect(formatShortcutKey('F5', true)).toBe('F5')
  })
})

describe('formatShortcut', () => {
  it('joins modifiers with " + " on non-mac', () => {
    expect(formatShortcut('Ctrl+Super', false)).toBe('Ctrl + Win')
  })

  it('joins modifiers on mac using Apple glyphs', () => {
    expect(formatShortcut('Ctrl+Super', true)).toBe('⌃ + ⌘')
  })

  it('returns empty string for empty input', () => {
    expect(formatShortcut('', false)).toBe('')
  })

  it('handles three-key combos', () => {
    expect(formatShortcut('Ctrl+Shift+Space', false)).toBe('Ctrl + Shift + Space')
  })
})
