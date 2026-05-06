import { describe, expect, it } from 'vitest'

import { getLlmsBaseUrl } from './getLlmsBaseUrl'

describe('getLlmsBaseUrl', () => {
	it('uses the configured API URL without trailing slashes', () => {
		import.meta.env.VITE_LLMS_API_URL = 'https://llms.example.com///'

		expect(getLlmsBaseUrl()).toBe('https://llms.example.com')
	})

	it('falls back to the local llms service when the override is blank', () => {
		import.meta.env.VITE_LLMS_API_URL = '   '

		expect(getLlmsBaseUrl()).toBe('http://localhost:50054')
	})
})
