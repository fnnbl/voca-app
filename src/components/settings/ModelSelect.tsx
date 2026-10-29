import { useState } from 'react'
import { useTranslation } from 'react-i18next'

const CUSTOM_OPTION = '__custom__'

interface ModelSelectProps {
  value: string
  onChange: (model: string) => void
  /** Models reported live by the provider, or null if not loaded. */
  liveModels: string[] | null
  /** Static list shown until (or instead of) a live list. */
  fallbackModels: string[]
  loading: boolean
  failed: boolean
  onRefresh?: () => void
  className?: string
}

export function ModelSelect({
  value,
  onChange,
  liveModels,
  fallbackModels,
  loading,
  failed,
  onRefresh,
  className = '',
}: ModelSelectProps) {
  const { t } = useTranslation()
  const options = liveModels ?? fallbackModels
  const isListed = options.includes(value)
  // Free-text entry for models a listing doesn't cover (custom endpoints,
  // brand-new releases). Starts open when nothing is listed at all.
  const [customMode, setCustomMode] = useState(options.length === 0)
  const showInput = customMode || options.length === 0

  // Only a live list can tell us the saved model is gone; against the
  // static fallback an unlisted value is just a custom choice.
  const unavailable = liveModels !== null && value !== '' && !isListed

  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-1.5">
        {showInput ? (
          <input
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={t('settings.models.customPlaceholder')}
            className={className}
          />
        ) : (
          <select
            value={value}
            onChange={(e) => {
              if (e.target.value === CUSTOM_OPTION) {
                setCustomMode(true)
              } else {
                onChange(e.target.value)
              }
            }}
            className={className}
          >
            {value !== '' && !isListed && (
              <option value={value}>
                {unavailable ? `${value} (${t('settings.models.unavailable')})` : value}
              </option>
            )}
            {options.map((m) => (
              <option key={m} value={m}>{m}</option>
            ))}
            <option value={CUSTOM_OPTION}>{t('settings.models.custom')}</option>
          </select>
        )}
        {showInput && options.length > 0 && (
          <button
            type="button"
            onClick={() => setCustomMode(false)}
            className="text-xs text-text-muted hover:text-text transition-colors"
          >
            {t('settings.models.backToList')}
          </button>
        )}
        {onRefresh && (
          <button
            type="button"
            onClick={onRefresh}
            disabled={loading}
            title={t('settings.models.refresh')}
            aria-label={t('settings.models.refresh')}
            className="px-1.5 py-1 text-xs text-text-muted hover:text-text disabled:opacity-40 transition-colors"
          >
            <span className={loading ? 'inline-block animate-spin' : 'inline-block'}>↻</span>
          </button>
        )}
      </div>
      {unavailable && (
        <p className="text-xs text-error">{t('settings.models.unavailableHint')}</p>
      )}
      {failed && !unavailable && (
        <p className="text-xs text-text-muted">{t('settings.models.loadFailed')}</p>
      )}
    </div>
  )
}
