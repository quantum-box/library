import { authTranslations } from './auth-translations'
import { dashboardTranslations } from './dashboard-translations'
import { v1betaTranslations } from './v1beta-translations'

export type Locale = 'en' | 'ja'

export const defaultLocale: Locale = 'en'

export const locales: Locale[] = ['en', 'ja']

export const translations = {
	en: {
		common: {
			library: 'Library',
			company: 'Quantum Box, Inc.',
			tagline: 'Knowledge OS for structured data',
			termsOfService: 'Terms of Service',
			privacyPolicy: 'Privacy Policy',
			loading: 'Loading...',
			error: 'An error occurred',
			retry: 'Retry',
			cancel: 'Cancel',
			save: 'Save',
			delete: 'Delete',
			edit: 'Edit',
			create: 'Create',
			back: 'Back',
			next: 'Next',
			submit: 'Submit',
			close: 'Close',
			confirm: 'Confirm',
			yes: 'Yes',
			no: 'No',
			search: 'Search',
			filter: 'Filter',
			sort: 'Sort',
			viewMore: 'View more',
			seeMore: 'See more',
			language: 'Language',
			settings: 'Settings',
		},
		auth: authTranslations.en,
		dashboard: dashboardTranslations.en,
		v1beta: v1betaTranslations.en,
	},
	ja: {
		common: {
			library: 'Library',
			company: 'Quantum Box, Inc.',
			tagline: '構造化データのためのナレッジOS',
			termsOfService: '利用規約',
			privacyPolicy: 'プライバシーポリシー',
			loading: '読み込み中...',
			error: 'エラーが発生しました',
			retry: '再試行',
			cancel: 'キャンセル',
			save: '保存',
			delete: '削除',
			edit: '編集',
			create: '作成',
			back: '戻る',
			next: '次へ',
			submit: '送信',
			close: '閉じる',
			confirm: '確認',
			yes: 'はい',
			no: 'いいえ',
			search: '検索',
			filter: 'フィルター',
			sort: '並び替え',
			viewMore: 'もっと見る',
			seeMore: 'もっと見る',
			language: '言語',
			settings: '設定',
		},
		auth: authTranslations.ja,
		dashboard: dashboardTranslations.ja,
		v1beta: v1betaTranslations.ja,
	},
}

export type BaseTranslationDictionary = (typeof translations)[Locale]
