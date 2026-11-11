import { useI18n } from '../../i18n'

export function CodeBlockStyleSelect({
  value,
  onChange,
  disabled,
}: {
  value: 'dark' | 'light'
  onChange: (value: 'dark' | 'light') => void
  disabled?: boolean
}) {
  const { t } = useI18n()
  return (
    <label className="library-code-style-label">
      {t('editor.codeStyle')}
      <select
        aria-label={t('editor.codeStyle')}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value === 'light' ? 'light' : 'dark')}
      >
        <option value="dark">{t('editor.codeStyleDark')}</option>
        <option value="light">{t('editor.codeStyleLight')}</option>
      </select>
    </label>
  )
}
