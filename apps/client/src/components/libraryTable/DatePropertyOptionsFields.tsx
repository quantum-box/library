import { useI18n } from '../../i18n'
import type { DatePropertyOptions } from '../../lib/propertyDateFormat'

const selectClassName =
  'h-8 w-full rounded-md border border-border-strong bg-background px-2 text-xs text-foreground outline-none focus-visible:border-primary disabled:opacity-60'

export function DatePropertyOptionsFields({
  options,
  disabled = false,
  modeDisabled = false,
  onChange,
}: {
  options: DatePropertyOptions
  disabled?: boolean
  modeDisabled?: boolean
  onChange: (options: DatePropertyOptions) => void
}) {
  const { t } = useI18n()

  return (
    <div className="space-y-2" data-testid="date-property-options">
      <label className="block space-y-1">
        <span className="block text-xs font-medium text-foreground">
          {t('repoSettings.dateMode')}
        </span>
        <select
          data-testid="date-property-mode"
          value={options.includeTime ? 'date_time' : 'date_only'}
          disabled={disabled || modeDisabled}
          className={selectClassName}
          onChange={(event) => onChange({
            ...options,
            includeTime: event.target.value === 'date_time',
          })}
        >
          <option value="date_only">{t('repoSettings.dateOnly')}</option>
          <option value="date_time">{t('repoSettings.dateTime')}</option>
        </select>
      </label>
      {modeDisabled ? (
        <p className="text-2xs text-muted-foreground">
          {t('repoSettings.dateModeCreateOnly')}
        </p>
      ) : null}

      <label className="block space-y-1">
        <span className="block text-xs font-medium text-foreground">
          {t('repoSettings.dateFormat')}
        </span>
        <select
          data-testid="date-property-format"
          value={options.dateFormat}
          disabled={disabled}
          className={selectClassName}
          onChange={(event) => onChange({
            ...options,
            dateFormat: event.target.value as DatePropertyOptions['dateFormat'],
          })}
        >
          <option value="locale">{t('repoSettings.dateFormatLocale')}</option>
          <option value="month_day_year">MM/DD/YYYY</option>
          <option value="day_month_year">DD/MM/YYYY</option>
          <option value="year_month_day">YYYY-MM-DD</option>
        </select>
      </label>

      {options.includeTime ? (
        <label className="block space-y-1">
          <span className="block text-xs font-medium text-foreground">
            {t('repoSettings.timeFormat')}
          </span>
          <select
            data-testid="date-property-time-format"
            value={options.timeFormat}
            disabled={disabled}
            className={selectClassName}
            onChange={(event) => onChange({
              ...options,
              timeFormat: event.target.value as DatePropertyOptions['timeFormat'],
            })}
          >
            <option value="twelve_hour">{t('repoSettings.timeFormat12Hour')}</option>
            <option value="twenty_four_hour">{t('repoSettings.timeFormat24Hour')}</option>
          </select>
        </label>
      ) : null}
    </div>
  )
}
