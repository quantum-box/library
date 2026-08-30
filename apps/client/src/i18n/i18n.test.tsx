import { fireEvent, render, screen } from '@testing-library/react'
import { DropdownMenu, DropdownMenuContent } from '@tachyon-sdk/native-ui'
import { beforeEach, describe, expect, it } from 'vitest'
import type { ReactNode } from 'react'
import { appKitConfig } from '../app/kitConfig'
import { LanguageMenuSection } from '../components/LanguageMenuSection'
import { detectLocale, resolveLocale } from './locales'
import { loadCatalog, translate, translatePlural } from './translate'
import { I18nProvider, readStoredLocale, useI18n } from './I18nContext'

const STORAGE_KEY = appKitConfig.storage.localeKey

/** The switcher only renders inside a dropdown, so tests open one around it. */
function OpenMenu({ children }: { children: ReactNode }) {
  return (
    <DropdownMenu open modal={false}>
      <DropdownMenuContent>{children}</DropdownMenuContent>
    </DropdownMenu>
  )
}

function CurrentLocale() {
  const { locale, t, tPlural } = useI18n()
  return (
    <>
      <span data-testid="locale">{locale}</span>
      <span data-testid="greeting">{t('home.title')}</span>
      <span data-testid="plural">{tPlural('home.organizationCount', 2)}</span>
    </>
  )
}

describe('resolveLocale', () => {
  it('keeps an exact match', () => {
    expect(resolveLocale('ja')).toBe('ja')
    expect(resolveLocale('pt-BR')).toBe('pt-BR')
  })

  it('routes Chinese by script or region', () => {
    expect(resolveLocale('zh')).toBe('zh-Hans')
    expect(resolveLocale('zh-CN')).toBe('zh-Hans')
    expect(resolveLocale('zh-TW')).toBe('zh-Hant')
    expect(resolveLocale('zh-Hant-HK')).toBe('zh-Hant')
  })

  it('falls back to the language when the region is not shipped', () => {
    expect(resolveLocale('en-GB')).toBe('en')
    expect(resolveLocale('pt-PT')).toBe('pt-BR')
    expect(resolveLocale('de-AT')).toBe('de')
  })

  it('returns undefined for a language we do not ship', () => {
    expect(resolveLocale('sv')).toBeUndefined()
    expect(resolveLocale('')).toBeUndefined()
    expect(resolveLocale(undefined)).toBeUndefined()
  })
})

describe('detectLocale', () => {
  it('takes the first preference it can serve', () => {
    expect(detectLocale(['sv', 'fi', 'ko-KR', 'en'])).toBe('ko')
  })

  it('falls back to English when nothing matches', () => {
    expect(detectLocale(['sv', 'fi'])).toBe('en')
    expect(detectLocale([])).toBe('en')
  })
})

describe('translate', () => {
  it('interpolates named placeholders', () => {
    expect(translate('en', 'sidebar.repositories.actionsFor', { name: 'acme/docs' })).toBe(
      'Repository actions for acme/docs',
    )
  })

  it('leaves an unknown placeholder untouched rather than blanking it', () => {
    expect(translate('en', 'sidebar.repositories.actionsFor', {})).toBe(
      'Repository actions for {name}',
    )
  })

  it('renders the key itself when it is missing, so the gap is visible', () => {
    // `as never` reaches past the key type on purpose: this is the runtime
    // guard for a key that only a stale build could ask for.
    expect(translate('en', 'nope.not.a.key' as never)).toBe('nope.not.a.key')
  })

  it('falls back to English while another locale is still loading', () => {
    expect(translate('ko', 'home.title')).toBe('Home')
  })

  it('uses the locale once its catalog has loaded', async () => {
    await loadCatalog('ja')
    expect(translate('ja', 'home.title')).toBe('ホーム')
  })
})

describe('translatePlural', () => {
  it('picks the English category', async () => {
    expect(translatePlural('en', 'home.organizationCount', 1)).toBe('1 organization')
    expect(translatePlural('en', 'home.organizationCount', 4)).toBe('4 organizations')
  })

  it('uses the extra Russian categories', async () => {
    await loadCatalog('ru')
    expect(translatePlural('ru', 'home.organizationCount', 1)).toBe('1 организация')
    expect(translatePlural('ru', 'home.organizationCount', 3)).toBe('3 организации')
    expect(translatePlural('ru', 'home.organizationCount', 9)).toBe('9 организаций')
  })
})

describe('I18nProvider', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('renders in the locale it is given and marks the document language', () => {
    render(
      <I18nProvider initial="en">
        <CurrentLocale />
      </I18nProvider>,
    )

    expect(screen.getByTestId('locale')).toHaveTextContent('en')
    expect(screen.getByTestId('greeting')).toHaveTextContent('Home')
    expect(screen.getByTestId('plural')).toHaveTextContent('2 organizations')
    expect(document.documentElement.lang).toBe('en')
  })

  it('reads a stored preference back', () => {
    window.localStorage.setItem(STORAGE_KEY, 'de')
    expect(readStoredLocale()).toBe('de')
  })

  it('ignores a stored value that is not a shipped locale', () => {
    window.localStorage.setItem(STORAGE_KEY, 'sv')
    expect(readStoredLocale()).toBeUndefined()
  })
})

describe('LanguageMenuSection', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('pins a locale and remembers it', async () => {
    await loadCatalog('ja')
    render(
      <I18nProvider initial="en">
        <OpenMenu>
          <LanguageMenuSection />
        </OpenMenu>
        <CurrentLocale />
      </I18nProvider>,
    )

    fireEvent.click(screen.getByTestId('language-option-ja'))

    expect(screen.getByTestId('locale')).toHaveTextContent('ja')
    expect(screen.getByTestId('greeting')).toHaveTextContent('ホーム')
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('ja')
  })

  it('drops the preference when the reader goes back to the device language', () => {
    window.localStorage.setItem(STORAGE_KEY, 'de')
    render(
      <I18nProvider initial="de">
        <OpenMenu>
          <LanguageMenuSection />
        </OpenMenu>
        <CurrentLocale />
      </I18nProvider>,
    )

    fireEvent.click(screen.getByTestId('language-option-system'))

    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull()
  })

  it('names every language in its own script', () => {
    render(
      <I18nProvider initial="en">
        <OpenMenu>
          <LanguageMenuSection />
        </OpenMenu>
      </I18nProvider>,
    )

    expect(screen.getByTestId('language-option-ja')).toHaveTextContent('日本語')
    expect(screen.getByTestId('language-option-ru')).toHaveTextContent('Русский')
    expect(screen.getByTestId('language-option-zh-Hant')).toHaveTextContent('繁體中文')
  })
})
