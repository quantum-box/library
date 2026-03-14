import { Card } from '@/components/ui/card'
import type { LpLanguage } from '@/app/lp'
import {
	BadgeCheck,
	Boxes,
	Code2,
	GitBranch,
	Share2,
	Users,
} from 'lucide-react'
import { fadeInAnimation, slideUpAnimation } from './animations'

const baseFeatures = [
	{ icon: Code2, key: 'api' },
	{ icon: Share2, key: 'linking' },
	{ icon: GitBranch, key: 'version' },
	{ icon: Users, key: 'contributors' },
	{ icon: Boxes, key: 'blocks' },
	{ icon: BadgeCheck, key: 'traceability' },
] as const

type FeatureKey = (typeof baseFeatures)[number]['key']

type FeatureCopy = {
	title: string
	description: string
	badge: string
}

const copy: Record<
	LpLanguage,
	{
		title: string
		subtitle: string
		features: Record<FeatureKey, FeatureCopy>
	}
> = {
	en: {
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
		<section id='features' className='space-y-12 scroll-mt-24'>
			<div className='text-center'>
				<p
					className={`text-sm uppercase tracking-[0.3em] text-sky-200/80 ${fadeInAnimation}`}
				>
					Library
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

			<div
				className={`grid gap-5 pt-4 sm:grid-cols-2 xl:grid-cols-3 ${slideUpAnimation}`}
			>
				{baseFeatures.map(feature => {
					const details = t.features[feature.key]
					return (
						<Card
							key={feature.key}
							className='group relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-6 transition duration-300 hover:border-sky-400/60 hover:bg-white/10'
						>
							<div className='pointer-events-none absolute -left-12 top-6 h-40 w-40 rounded-full bg-sky-500/20 blur-3xl opacity-0 transition duration-500 group-hover:opacity-80' />
							<div className='relative flex items-center justify-between gap-4'>
								<div className='flex items-center gap-3'>
									<div className='flex h-12 w-12 items-center justify-center rounded-2xl border border-white/10 bg-white/10 text-white transition group-hover:border-sky-300/40 group-hover:text-sky-200'>
										<feature.icon className='h-6 w-6' />
									</div>
									<div>
										<h3 className='text-lg font-semibold text-white sm:text-xl'>
											{details.title}
										</h3>
										<p className='mt-1 text-sm text-slate-300'>
											{details.description}
										</p>
									</div>
								</div>
								<span className='rounded-full border border-sky-400/40 bg-sky-500/10 px-3 py-1 text-xs font-semibold uppercase tracking-widest text-sky-200'>
									{details.badge}
								</span>
							</div>
						</Card>
					)
				})}
			</div>
		</section>
	)
}
