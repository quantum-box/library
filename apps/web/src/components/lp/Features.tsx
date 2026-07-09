import type { LpLanguage } from '@/app/lp'
import { SectionHeader } from './SectionHeader'

const featureOrder = [
	'api',
	'linking',
	'version',
	'contributors',
	'blocks',
	'traceability',
] as const

type FeatureKey = (typeof featureOrder)[number]

type FeatureCopy = {
	title: string
	description: string
	badge: string
}

const copy: Record<
	LpLanguage,
	{
		eyebrow: string
		title: string
		subtitle: string
		features: Record<FeatureKey, FeatureCopy>
	}
> = {
	en: {
		eyebrow: 'Features',
		title: 'API-first knowledge infrastructure for product teams',
		subtitle:
			'Standardize how knowledge travels across your organization with services built for real-time updates, compliance, and reuse.',
		features: {
			api: {
				title: 'API-native publishing',
				description:
					'Expose every content type through typed APIs ready for automation.',
				badge: 'Public SDK',
			},
			linking: {
				title: 'Contextual linking',
				description:
					'Connect entries with bidirectional references and graph navigation.',
				badge: 'Graph View',
			},
			version: {
				title: 'Version-aware workflows',
				description:
					'Ship confidently with diffable histories and automated approvals.',
				badge: 'Smart Review',
			},
			contributors: {
				title: 'Contributor visibility',
				description:
					'Understand authorship, expertise, and impact with unified profiles.',
				badge: 'Identity Layer',
			},
			blocks: {
				title: 'Reusable knowledge blocks',
				description:
					'Compose experiences from modular content synced across channels.',
				badge: 'Composable UI',
			},
			traceability: {
				title: 'Source of record',
				description:
					'Prove every statement with evidence and audit-ready metadata.',
				badge: 'Traceability',
			},
		},
	},
	ja: {
		eyebrow: '特徴',
		title: 'プロダクトチームのための API ファーストなナレッジ基盤',
		subtitle:
			'リアルタイム更新・コンプライアンス・再利用性を前提としたサービスで、組織内の知識の流れを標準化します。',
		features: {
			api: {
				title: 'API ネイティブな公開',
				description:
					'すべてのコンテンツ型を型付き API として公開し、自動化に備えます。',
				badge: 'Public SDK',
			},
			linking: {
				title: '文脈をつなぐリンク',
				description:
					'双方向リファレンスとグラフビューで関連情報を一目で把握します。',
				badge: 'Graph View',
			},
			version: {
				title: 'バージョン対応ワークフロー',
				description: '差分と承認を自動化し、安心して継続的にリリースできます。',
				badge: 'Smart Review',
			},
			contributors: {
				title: '貢献者の可視化',
				description: '誰が何を編集したかを一元管理し、適切な評価につなげます。',
				badge: 'Identity Layer',
			},
			blocks: {
				title: '再利用できるナレッジブロック',
				description:
					'モジュール化したコンテンツをウェブやアプリへ同期配信します。',
				badge: 'Composable UI',
			},
			traceability: {
				title: '信頼を裏付ける証跡',
				description:
					'すべての記述に根拠を紐づけ、監査可能なメタデータを保持します。',
				badge: 'Traceability',
			},
		},
	},
}

export function Features({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section
			id='features'
			className='scroll-mt-24 border-t border-slate-200 py-16 sm:py-20'
		>
			<SectionHeader
				chapter='01'
				eyebrow={t.eyebrow}
				title={t.title}
				lead={t.subtitle}
			/>

			<div className='mt-12 grid gap-5 sm:grid-cols-2 lg:grid-cols-3'>
				{featureOrder.map((key, index) => {
					const details = t.features[key]
					return (
						<div
							key={key}
							className='rounded-lg border border-slate-200 bg-white p-6 transition-colors hover:border-blue-300'
						>
							<div className='flex items-baseline justify-between gap-4'>
								<span className='font-mono text-sm text-blue-700'>
									1.{index + 1}
								</span>
								<span className='font-mono text-[11px] uppercase tracking-wide text-slate-400'>
									{details.badge}
								</span>
							</div>
							<h3 className='mt-4 text-base font-semibold text-slate-900'>
								{details.title}
							</h3>
							<p className='mt-2 text-sm leading-relaxed text-slate-600'>
								{details.description}
							</p>
						</div>
					)
				})}
			</div>
		</section>
	)
}
