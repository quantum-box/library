import type { LpLanguage } from '@/app/lp'
import { SectionHeader } from './SectionHeader'

const roadmapOrder = [
	{ key: 'api', tone: 'now' },
	{ key: 'billing', tone: 'beta' },
	{ key: 'version', tone: 'planned' },
	{ key: 'global', tone: 'planned' },
	{ key: 'security', tone: 'planned' },
	{ key: 'extensibility', tone: 'planned' },
] as const

type RoadmapKey = (typeof roadmapOrder)[number]['key']

const toneStyles = {
	now: {
		dot: 'bg-emerald-500 ring-4 ring-emerald-100',
		text: 'text-emerald-700',
	},
	beta: { dot: 'bg-blue-500 ring-4 ring-blue-100', text: 'text-blue-700' },
	planned: {
		dot: 'bg-slate-300 ring-4 ring-slate-100',
		text: 'text-slate-500',
	},
} as const

type RoadmapItemCopy = {
	stage: string
	title: string
	description: string
}

const copy: Record<
	LpLanguage,
	{
		eyebrow: string
		title: string
		subtitle: string
		items: Record<RoadmapKey, RoadmapItemCopy>
	}
> = {
	en: {
		eyebrow: 'Roadmap',
		title: 'A roadmap built with customers',
		subtitle:
			'We prioritize resilient APIs, governance, and integrations that keep Library operating as your long-term knowledge OS.',
		items: {
			api: {
				stage: 'Available now',
				title: 'Create, edit, delete APIs',
				description:
					'Typed GraphQL and REST endpoints for every knowledge object, ready for live prototyping and production workloads.',
			},
			billing: {
				stage: 'In Beta',
				title: 'Usage-based billing',
				description:
					'RC and RU meters, plan management, and workspace cost controls with alerting and exports.',
			},
			version: {
				stage: 'Q4 2025',
				title: 'Version-aware releases',
				description:
					'Branch previews, merge policies, and programmable release hooks for continuous documentation delivery.',
			},
			global: {
				stage: 'Q1 2026',
				title: 'Global expansion',
				description:
					'Multi-language authoring workflows, locale fallbacks, and regional data residency.',
			},
			security: {
				stage: 'Planned',
				title: 'Advanced security posture',
				description:
					'Sensitive content tagging, bring-your-own KMS, and delegated admin tooling with audit APIs.',
			},
			extensibility: {
				stage: 'Exploratory',
				title: 'Extensibility ecosystem',
				description:
					'Workflow builder, Git interface, and connectors for BI tools, spreadsheets, and knowledge graphs.',
			},
		},
	},
	ja: {
		eyebrow: 'ロードマップ',
		title: 'お客さまと共に描くロードマップ',
		subtitle:
			'堅牢な API とガバナンス、そして統合性を最優先に、Library を長期的なナレッジ OS として進化させていきます。',
		items: {
			api: {
				stage: '提供中',
				title: 'API で作成・編集・削除',
				description:
					'すべてのナレッジオブジェクトを型付き GraphQL / REST エンドポイントとして提供し、試作から本番運用まで対応します。',
			},
			billing: {
				stage: 'ベータ提供中',
				title: '従量課金の実装',
				description:
					'RC / RU 計測とプラン管理、ワークスペース単位のコスト管理をアラート・エクスポート付きで提供します。',
			},
			version: {
				stage: '2025年 第4四半期',
				title: 'バージョン対応リリース',
				description:
					'ブランチプレビュー、マージポリシー、リリースフックを備えた継続的ドキュメント配信を実現します。',
			},
			global: {
				stage: '2026年 第1四半期',
				title: 'グローバル展開',
				description:
					'多言語の執筆ワークフロー、ロケールフォールバック、リージョナルデータ保持に対応します。',
			},
			security: {
				stage: '計画中',
				title: '高度なセキュリティ',
				description:
					'機密コンテンツのタグ付け、独自 KMS、委任管理と監査 API による強固な体制を整備します。',
			},
			extensibility: {
				stage: '検討中',
				title: '拡張エコシステム',
				description:
					'ワークフロービルダーや Git 連携、BI・スプレッドシート・ナレッジグラフ向けコネクタを拡充します。',
			},
		},
	},
}

export function Roadmap({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section
			id='roadmap'
			className='scroll-mt-24 border-t border-slate-200 py-16 sm:py-20'
		>
			<SectionHeader
				chapter='05'
				eyebrow={t.eyebrow}
				title={t.title}
				lead={t.subtitle}
			/>

			<ol className='mt-12 max-w-3xl'>
				{roadmapOrder.map((item, index) => {
					const details = t.items[item.key]
					const tone = toneStyles[item.tone]
					const isLast = index === roadmapOrder.length - 1
					return (
						<li key={item.key} className='relative flex gap-6'>
							<div className='flex flex-col items-center'>
								<span
									className={`mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full ${tone.dot}`}
								/>
								{!isLast && <span className='w-px flex-1 bg-slate-200' />}
							</div>
							<div className={isLast ? 'pb-0' : 'pb-10'}>
								<p
									className={`font-mono text-xs uppercase tracking-[0.15em] ${tone.text}`}
								>
									{details.stage}
								</p>
								<h3 className='mt-1.5 text-base font-semibold text-slate-900 sm:text-lg'>
									{details.title}
								</h3>
								<p className='mt-1.5 text-sm leading-relaxed text-slate-600'>
									{details.description}
								</p>
							</div>
						</li>
					)
				})}
			</ol>
		</section>
	)
}
