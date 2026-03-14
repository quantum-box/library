export const dashboardTranslations = {
	en: {
		sidebar: {
			dashboard: 'Dashboard',
			repositories: 'Repositories',
			organizations: 'Organizations',
			findRepository: 'Find a repository...',
			noRepositories: 'No repositories yet',
		},
		main: {
			home: 'Home',
			sendFeedback: 'Send feedback',
			filter: 'Filter',
			yourRepositories: 'Your Repositories',
			createRepository: {
				title: 'Create a new repository! 🎉',
				description:
					'You can now create a new repository in your organization. Get started now!',
				cta: 'Create a repository',
			},
		},
		rightSidebar: {
			quickStart: 'Quick Start',
			step1: 'Create an organization or join one',
			step2: 'Create a repository to organize your content',
			step3: 'Add properties and data entries via UI or API',
		},
	},
	ja: {
		sidebar: {
			dashboard: 'ダッシュボード',
			repositories: 'リポジトリ',
			organizations: '組織',
			findRepository: 'リポジトリを検索...',
			noRepositories: 'リポジトリはまだありません',
		},
		main: {
			home: 'ホーム',
			sendFeedback: 'フィードバックを送信',
			filter: 'フィルター',
			yourRepositories: 'リポジトリ一覧',
			createRepository: {
				title: '新しいリポジトリを作成しましょう！🎉',
				description:
					'組織に新しいリポジトリを作成できるようになりました。今すぐ始めましょう！',
				cta: 'リポジトリを作成',
			},
		},
		rightSidebar: {
			quickStart: 'クイックスタート',
			step1: '組織を作成するか、既存の組織に参加する',
			step2: 'リポジトリを作成してコンテンツを整理する',
			step3: 'UIまたはAPIでプロパティとデータを追加する',
		},
	},
}

export type DashboardTranslationDictionary =
	(typeof dashboardTranslations)['en']
