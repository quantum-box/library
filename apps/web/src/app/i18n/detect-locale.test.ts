import { afterEach, describe, expect, it, vi } from 'vitest'

import { LOCALE_COOKIE_NAME } from '@/lib/i18n/constants'
import { defaultLocale } from '@/lib/i18n/translations'
import { detectLocale } from './detect-locale'

const setCookie = (value: string) => {
	vi.stubGlobal('document', { cookie: value })
}

const setNavigatorLanguage = (value: string) => {
	vi.stubGlobal('navigator', { language: value })
}

afterEach(() => {
	vi.unstubAllGlobals()
})

describe('detectLocale', () => {
	it('uses the supported locale cookie before browser language', () => {
		setCookie(`${LOCALE_COOKIE_NAME}=ja`)
		setNavigatorLanguage('en-US')

		expect(detectLocale()).toBe('ja')
	})

	it('ignores unsupported locale cookies and falls back to browser language', () => {
		setCookie(`${LOCALE_COOKIE_NAME}=fr`)
		setNavigatorLanguage('ja-JP')

		expect(detectLocale()).toBe('ja')
	})

	it('returns the default locale when no supported source is available', () => {
		setCookie('')
		setNavigatorLanguage('fr-FR')

		expect(detectLocale()).toBe(defaultLocale)
	})
})
