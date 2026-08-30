import { describe, expect, it } from 'vitest'
import { SUPPORTED_LOCALES, type Locale } from './locales'
import { en } from './messages/en'
import { ja } from './messages/ja'
import { zhHans } from './messages/zh-Hans'
import { zhHant } from './messages/zh-Hant'
import { ko } from './messages/ko'
import { es } from './messages/es'
import { fr } from './messages/fr'
import { de } from './messages/de'
import { ptBR } from './messages/pt-BR'
import { it as italian } from './messages/it'
import { ru } from './messages/ru'
import type { LocaleMessages, Messages } from './translate'

/**
 * Every shipped catalog, keyed the same way the loader keys them. A locale
 * added to `SUPPORTED_LOCALES` without a catalog fails the first test here.
 */
const catalogs: Record<Locale, LocaleMessages> = {
  en,
  ja,
  'zh-Hans': zhHans,
  'zh-Hant': zhHant,
  ko,
  es,
  fr,
  de,
  'pt-BR': ptBR,
  it: italian,
  ru,
}

const PLACEHOLDER = /\{(\w+)\}/g

function placeholders(template: string): string[] {
  return [...template.matchAll(PLACEHOLDER)].map((match) => match[1]).sort()
}

const englishKeys = Object.keys(en) as Array<keyof Messages>

describe('message catalogs', () => {
  it('ships a catalog for every supported locale', () => {
    expect(Object.keys(catalogs).sort()).toEqual([...SUPPORTED_LOCALES].sort())
  })

  it.each(Object.keys(catalogs) as Locale[])('%s covers every English key', (locale) => {
    const missing = englishKeys.filter((key) => !(key in catalogs[locale]))
    expect(missing).toEqual([])
  })

  it.each(Object.keys(catalogs) as Locale[])('%s adds no keys English lacks', (locale) => {
    // A catalog may add plural categories English has no form for (Russian's
    // `few`/`many`); anything else is a typo or a key that outlived its use.
    const extra = Object.keys(catalogs[locale]).filter((key) => {
      if (key in en) return false
      const base = key.replace(/\.(one|two|few|many|zero|other)$/, '')
      return base === key || !(`${base}.other` in en)
    })
    expect(extra).toEqual([])
  })

  it.each(Object.keys(catalogs) as Locale[])('%s keeps every placeholder', (locale) => {
    const mismatched = englishKeys
      .filter((key) => {
        const translated = catalogs[locale][key]
        return translated !== undefined &&
          placeholders(en[key]).join(',') !== placeholders(translated).join(',')
      })
      .map((key) => `${key}: expected ${placeholders(en[key]).join('|')}`)
    expect(mismatched).toEqual([])
  })

  it.each(Object.keys(catalogs) as Locale[])('%s leaves no entry blank', (locale) => {
    const blank = englishKeys.filter((key) => catalogs[locale][key]?.trim() === '')
    expect(blank).toEqual([])
  })

  it('pairs every plural key with an "other" variant', () => {
    const pluralBases = new Set(
      englishKeys
        .filter((key) => /\.(one|two|few|many|zero|other)$/.test(key))
        .map((key) => key.replace(/\.(one|two|few|many|zero|other)$/, '')),
    )
    const missingOther = [...pluralBases].filter((base) => !(`${base}.other` in en))
    expect(missingOther).toEqual([])
  })
})
