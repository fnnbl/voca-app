// Render internal shortcut tokens (e.g. "Ctrl+Super") with labels users
// actually recognise on their platform: "Ctrl + Win" on Windows/Linux,
// Apple modifier glyphs on macOS.

export function isMacPlatform(): boolean {
  return typeof navigator !== 'undefined' && navigator.platform.includes('Mac')
}

export function formatShortcutKey(key: string, isMac: boolean = isMacPlatform()): string {
  switch (key) {
    case 'Super': return isMac ? '⌘' : 'Win'
    case 'Alt':   return isMac ? '⌥' : 'Alt'
    case 'Ctrl':  return isMac ? '⌃' : 'Ctrl'
    case 'Shift': return isMac ? '⇧' : 'Shift'
    default:      return key
  }
}

export function formatShortcut(shortcut: string, isMac: boolean = isMacPlatform()): string {
  if (!shortcut) return ''
  return shortcut.split('+').map((k) => formatShortcutKey(k, isMac)).join(' + ')
}
