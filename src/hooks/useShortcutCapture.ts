import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { setCapturing } from '../shortcut/captureState'

const MODIFIERS = ['Ctrl', 'Alt', 'Shift', 'Super'] as const
type Modifier = typeof MODIFIERS[number]

function tokenFromKey(e: KeyboardEvent): string | null {
  switch (e.key) {
    case 'Control': return 'Ctrl'
    case 'Alt':     return 'Alt'
    case 'Shift':   return 'Shift'
    case 'Meta':
    case 'OS':      return 'Super'
    case ' ':       return 'Space'
    case 'Enter':   return 'Return'
    case 'Tab':     return 'Tab'
    case 'Backspace': return 'Backspace'
    case 'Escape':  return 'Escape'
    case 'Delete':  return 'Delete'
  }
  if (e.key.length === 1) return e.key.toUpperCase()
  if (/^F\d{1,2}$/.test(e.key)) return e.key
  return null
}

export function sortShortcut(keys: string[]): string {
  return [...keys].sort((a, b) => {
    const ai = MODIFIERS.indexOf(a as Modifier)
    const bi = MODIFIERS.indexOf(b as Modifier)
    if (ai === -1 && bi === -1) return 0
    if (ai === -1) return 1
    if (bi === -1) return -1
    return ai - bi
  }).join('+')
}

export interface ShortcutCaptureState {
  recording: boolean
  held: string[]
  start: () => void
  cancel: () => void
  onKeyDown: (e: KeyboardEvent<HTMLElement>) => void
  onKeyUp: (e: KeyboardEvent<HTMLElement>) => void
  onBlur: () => void
}

export function useShortcutCapture(onChange: (shortcut: string) => void): ShortcutCaptureState {
  const [recording, setRecording] = useState(false)
  const [, setVersion] = useState(0)
  const heldRef = useRef<string[]>([])
  const committedRef = useRef(false)

  useEffect(() => {
    setCapturing(recording)
    return () => { if (recording) setCapturing(false) }
  }, [recording])

  function touch() { setVersion(v => v + 1) }

  function reset() {
    heldRef.current = []
    committedRef.current = false
    touch()
  }

  function commit() {
    if (committedRef.current || heldRef.current.length === 0) return
    committedRef.current = true
    onChange(sortShortcut(heldRef.current))
    setRecording(false)
    reset()
  }

  function cancel() {
    setRecording(false)
    reset()
  }

  function start() {
    reset()
    setRecording(true)
  }

  function onKeyDown(e: KeyboardEvent<HTMLElement>) {
    if (!recording) return
    e.preventDefault()
    e.stopPropagation()
    const token = tokenFromKey(e)
    if (!token || heldRef.current.includes(token)) return
    heldRef.current = [...heldRef.current, token]
    touch()
    if (!MODIFIERS.includes(token as Modifier)) {
      commit()
    }
  }

  function onKeyUp(e: KeyboardEvent<HTMLElement>) {
    if (!recording) return
    e.preventDefault()
    e.stopPropagation()
    if (heldRef.current.length >= 2) {
      commit()
    } else {
      reset()
    }
  }

  return {
    recording,
    held: heldRef.current,
    start,
    cancel,
    onKeyDown,
    onKeyUp,
    onBlur: cancel,
  }
}
