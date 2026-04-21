import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { isCapturing } from '../shortcut/captureState'

function tokenFromDomKey(key: string): string | null {
  switch (key) {
    case 'Control':   return 'Ctrl'
    case 'Alt':
    case 'AltGraph':  return 'Alt'
    case 'Shift':     return 'Shift'
    case 'Meta':
    case 'OS':        return 'Super'
    case ' ':         return 'Space'
    case 'Enter':     return 'Return'
    case 'Tab':       return 'Tab'
    case 'Backspace': return 'Backspace'
    case 'Escape':    return 'Escape'
    case 'Delete':    return 'Delete'
  }
  if (key.length === 1) return key.toUpperCase()
  if (/^F\d{1,2}$/.test(key)) return key
  return null
}

/**
 * Mirror the backend hotkey state machine in the WebView so the shortcut
 * also fires when VOCA itself is the focused window. The backend dedups
 * near-simultaneous fires from rdev + this hook via a 50ms window.
 */
export function useShortcutFallback(shortcutString: string) {
  const targetRef = useRef<Set<string>>(new Set())
  const heldRef = useRef<Set<string>>(new Set())
  const activeRef = useRef(false)

  useEffect(() => {
    targetRef.current = new Set(shortcutString.split('+').filter(Boolean))
    heldRef.current.clear()
    if (activeRef.current) {
      activeRef.current = false
      invoke('trigger_shortcut_release').catch(() => {})
    }
  }, [shortcutString])

  useEffect(() => {
    function evaluate() {
      const target = targetRef.current
      if (target.size === 0) return
      const held = heldRef.current
      let allHeld = true
      for (const t of target) if (!held.has(t)) { allHeld = false; break }
      if (allHeld && !activeRef.current) {
        activeRef.current = true
        invoke('trigger_shortcut_press').catch(() => {})
      } else if (!allHeld && activeRef.current) {
        activeRef.current = false
        invoke('trigger_shortcut_release').catch(() => {})
      }
    }

    function onDown(e: KeyboardEvent) {
      if (isCapturing()) return
      const tok = tokenFromDomKey(e.key)
      if (!tok) return
      heldRef.current.add(tok)
      evaluate()
    }
    function onUp(e: KeyboardEvent) {
      if (isCapturing()) {
        // Even while capturing, keep heldRef empty so we don't latch on exit.
        heldRef.current.clear()
        return
      }
      const tok = tokenFromDomKey(e.key)
      if (!tok) return
      heldRef.current.delete(tok)
      evaluate()
    }
    function onBlur() {
      heldRef.current.clear()
      if (activeRef.current) {
        activeRef.current = false
        invoke('trigger_shortcut_release').catch(() => {})
      }
    }

    window.addEventListener('keydown', onDown, true)
    window.addEventListener('keyup', onUp, true)
    window.addEventListener('blur', onBlur)
    return () => {
      window.removeEventListener('keydown', onDown, true)
      window.removeEventListener('keyup', onUp, true)
      window.removeEventListener('blur', onBlur)
    }
  }, [])
}
