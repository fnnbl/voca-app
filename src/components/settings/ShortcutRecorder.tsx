import { useTranslation } from 'react-i18next'
import { useShortcutCapture, sortShortcut } from '../../hooks/useShortcutCapture'
import { formatShortcut } from '../../shortcut/format'

interface ShortcutRecorderProps {
  value: string
  onChange: (shortcut: string) => void
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
