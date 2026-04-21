import { useTranslation } from 'react-i18next'
import { useShortcutCapture, sortShortcut } from '../../hooks/useShortcutCapture'

interface ShortcutRecorderProps {
  value: string
  onChange: (shortcut: string) => void
}

function displayLabel(key: string): string {
  const isMac = navigator.platform.includes('Mac')
  if (key === 'Super') return isMac ? '⌘' : '⊞'
  if (key === 'Alt')   return isMac ? '⌥' : 'Alt'
  if (key === 'Ctrl')  return 'Ctrl'
  if (key === 'Shift') return 'Shift'
  return key
}

function formatShortcut(shortcut: string): string {
  if (!shortcut) return ''
  return shortcut.split('+').map(displayLabel).join(' + ')
}

export function ShortcutRecorder({ value, onChange }: ShortcutRecorderProps) {
  const { t } = useTranslation()
  const { recording, held, start, cancel, onKeyDown, onKeyUp, onBlur } = useShortcutCapture(onChange)

  const displayValue = recording
    ? (held.length > 0
        ? formatShortcut(sortShortcut(held))
        : t('settings.shortcut.recording', 'Tasten drücken…'))
    : formatShortcut(value)

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        tabIndex={0}
        onKeyDown={onKeyDown}
        onKeyUp={onKeyUp}
        onClick={start}
        onBlur={onBlur}
        className={[
          'min-w-[140px] text-xs font-mono px-3 py-1.5 rounded border text-left transition-colors focus:outline-none',
          recording
            ? 'border-accent bg-accent-subtle text-text'
            : 'border-border bg-surface text-text hover:border-border-hover',
        ].join(' ')}
      >
        {displayValue}
      </button>
      {recording && (
        <button
          type="button"
          onClick={cancel}
          className="text-xs text-text-muted hover:text-text transition-colors"
        >
          {t('common.cancel')}
        </button>
      )}
    </div>
  )
}
