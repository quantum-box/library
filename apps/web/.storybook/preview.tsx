import type { Preview, ReactRenderer } from '@storybook/react'
import type { DecoratorFunction } from '@storybook/types'
import { http, HttpResponse, passthrough } from 'msw'
import { initialize, mswLoader } from 'msw-storybook-addon'
import '../src/app/globals.css'

import { getDictionary } from '../src/app/i18n/get-dictionary'
import { I18nProvider } from '../src/app/i18n/i18n-provider'
import type { Locale } from '../src/lib/i18n/translations'

// Default MSW handlers for common endpoints
const defaultHandlers = [
	// Mock NextAuth session endpoint
	http.get('/api/auth/session', () => {
		return HttpResponse.json({
			user: null,
			expires: new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString(),
		})
	}),
	// Passthrough Google Maps API requests
	http.get('https://maps.googleapis.com/*', () => passthrough()),
	http.get('https://maps.gstatic.com/*', () => passthrough()),
	http.get('https://fonts.googleapis.com/*', () => passthrough()),
	http.get('https://fonts.gstatic.com/*', () => passthrough()),
	http.post('https://maps.googleapis.com/*', () => passthrough()),
]

initialize({
	onUnhandledRequest: 'warn',
})

// I18n decorator for Storybook
const withI18n: DecoratorFunction<ReactRenderer> = (Story, context) => {
	const locale = (context.globals.locale || 'en') as Locale
	const dictionary = getDictionary(locale)

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			<Story />
		</I18nProvider>
	)
}

const preview: Preview = {
	loaders: [mswLoader],
	decorators: [withI18n],
	globalTypes: {
		locale: {
			name: 'Locale',
			description: 'Internationalization locale',
			defaultValue: 'en',
			toolbar: {
				icon: 'globe',
				items: [
					{ value: 'en', right: '🇺🇸', title: 'English' },
					{ value: 'ja', right: '🇯🇵', title: '日本語' },
				],
				showName: true,
				dynamicTitle: true,
			},
		},
	},
	parameters: {
		msw: {
			handlers: defaultHandlers,
		},
		controls: {
			matchers: {
				color: /(background|color)$/i,
				date: /Date$/i,
			},
		},
		nextjs: {
			appDirectory: true,
		},
		viewport: {
			viewports: {
				mobile1: {
					name: 'Small mobile',
					styles: {
						width: '320px',
						height: '568px',
					},
				},
				mobile2: {
					name: 'Large mobile',
					styles: {
						width: '414px',
						height: '896px',
					},
				},
				tablet: {
					name: 'Tablet',
					styles: {
						width: '768px',
						height: '1024px',
					},
				},
				desktop: {
					name: 'Desktop',
					styles: {
						width: '1280px',
						height: '800px',
					},
				},
			},
		},
	},

	tags: ['autodocs'],
}

export default preview

