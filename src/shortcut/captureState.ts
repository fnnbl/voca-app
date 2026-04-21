// Shared flag: true while the ShortcutRecorder UI is actively capturing a new
// shortcut. The frontend shortcut fallback checks this and stays quiet, so
// pressing Ctrl+Super to record a new combo doesn't also start audio capture.

let capturing = false
export function setCapturing(v: boolean) { capturing = v }
export function isCapturing() { return capturing }
