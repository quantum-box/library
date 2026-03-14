import type { LpLanguage } from '@/app/lp'
import { fadeInAnimation, slideUpAnimation } from './animations'
import {
	Code2,
	DollarSign,
	GitBranch,
	Globe2,
	ShieldCheck,
	Workflow,
} from 'lucide-react'

const roadmapOrder = [
	{ key: 'api', icon: Code2 },
	{ key: 'billing', icon: DollarSign },
	{ key: 'version', icon: GitBranch },
	{ key: 'global', icon: Globe2 },
	{ key: 'security', icon: ShieldCheck },
	{ key: 'extensibility', icon: Workflow },
] as const

type RoadmapKey = (typeof roadmapOrder)[number]['key']

type RoadmapItemCopy = {
	stage: string
	title: string
	description: string
}

const copy: Record<
	LpLanguage,
	{
		title: string
		subtitle: string
		items: Record<RoadmapKey, RoadmapItemCopy>
	}
> = {
	en: {
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
		<section id='roadmap' className='scroll-mt-24 space-y-12'>
			<div className='text-center'>
				<p
					className={`text-sm uppercase tracking-[0.3em] text-sky-200/80 ${fadeInAnimation}`}
				>
					{lang === 'en' ? 'Where we are heading' : 'これからの展望'}
				</p>
				<div className={`${fadeInAnimation} delay-150 space-y-4`}>
					<h2 className='text-3xl font-semibold text-white sm:text-4xl md:text-5xl'>
						{t.title}
					</h2>
					<p className='mx-auto max-w-3xl text-base text-slate-300 sm:text-lg'>
						{t.subtitle}
					</p>
				</div>
			</div>

			<div className={`grid gap-6 sm:grid-cols-2 ${slideUpAnimation}`}>
				{roadmapOrder.map(item => {
					const details = t.items[item.key]
					const Icon = item.icon
					return (
						<div
							key={item.key}
							className='relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-6 backdrop-blur transition duration-300 hover:border-sky-400/60 hover:bg-white/10'
						>
							<div className='pointer-events-none absolute -right-14 top-8 h-36 w-36 rounded-full bg-sky-500/15 blur-3xl opacity-0 transition duration-500 hover:opacity-80' />
							<div className='relative flex items-center gap-4'>
								<div className='flex h-12 w-12 items-center justify-center rounded-2xl border border-white/15 bg-white/10 text-white'>
									<Icon className='h-6 w-6' />
								</div>
								<div className='space-y-1 text-left'>
									<p className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
										{details.stage}
									</p>
									<h3 className='text-lg font-semibold text-white sm:text-xl'>
										{details.title}
									</h3>
									<p className='text-sm text-slate-300'>
										{details.description}
									</p>
								</div>
							</div>
						</div>
					)
				})}
			</div>
		</section>
	)
}
