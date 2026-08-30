import {
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from '@tachyon-sdk/native-ui'
import { useI18n, type Locale } from '../i18n'

const FOLLOW_DEVICE = 'system'

/**
 * Language picker rendered inside the account dropdown.
 *
 * The first entry follows the device language so a user who never picks one
 * keeps tracking their OS setting, and the rest pin an explicit locale. Each
 * language is written in its own script, which is what makes a switcher
 * usable when the current UI language is one you cannot read.
 */
export function LanguageMenuSection() {
  const { locale, isExplicit, availableLocales, setLocale, resetLocale, t } = useI18n()

  return (
    <>
      <DropdownMenuLabel>{t('language.heading')}</DropdownMenuLabel>
      <DropdownMenuRadioGroup
        value={isExplicit ? locale : FOLLOW_DEVICE}
        onValueChange={(value) => {
          if (value === FOLLOW_DEVICE) {
            resetLocale()
            return
          }
          setLocale(value as Locale)
        }}
      >
        <DropdownMenuRadioItem value={FOLLOW_DEVICE} data-testid="language-option-system">
          {t('language.followDevice')}
        </DropdownMenuRadioItem>
        {availableLocales.map((descriptor) => (
          <DropdownMenuRadioItem
            key={descriptor.code}
            value={descriptor.code}
            data-testid={`language-option-${descriptor.code}`}
            lang={descriptor.code}
          >
            {descriptor.nativeName}
          </DropdownMenuRadioItem>
        ))}
      </DropdownMenuRadioGroup>
    </>
  )
}
