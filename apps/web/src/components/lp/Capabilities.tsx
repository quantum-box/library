import type { LpLanguage } from '@/app/lp'
import { Activity, Link2, Rocket, ShieldCheck } from 'lucide-react'
import { fadeInAnimation, slideUpAnimation } from './animations'

const baseCapabilities = [
	{ icon: Rocket, key: 'launch' },
	{ icon: Link2, key: 'connect' },
	{ icon: ShieldCheck, key: 'compliance' },
	{ icon: Activity, key: 'adaptive' },
] as const

type CapabilityKey = (typeof baseCapabilities)[number]['key']

type CapabilityCopy = {
	title: string
	description: string
	statLabel: string
	statValue: string
	statFootnote: string
}

const copy: Record<
	LpLanguage,
	{
		heading: string
		intro: string
		bullets: string[]
		capabilities: Record<CapabilityKey, CapabilityCopy>
	}
> = {
	en: {
		heading: 'Built for organizations that treat knowledge as infrastructure',
		intro:
			'Library aligns editors, developers, and compliance reviewers around a single operating model. Everything stays versioned, traceable, and programmable.',
		bullets: [
			'Keep product, legal, and go-to-market teams aligned with a cross-functional model.',
			'Mirror the same schema across components and APIs for true reuse.',
			'Surface bottlenecks with insights before knowledge falls out of sync.',
		],
		capabilities: {
			launch: {
				title: 'Launch new knowledge spaces fast',
				description:
					'Start from templates with seeded taxonomies, localization, and contracts to ship new verticals in days.',
				statLabel: 'Days to launch',
				statValue: '5',
				statFootnote: 'Median rollout for pilot customers',
			},
			connect: {
				title: 'Connect truth across systems',
				description:
					'Bridge CRM, analytics, and documentation with resilient references and bi-directional sync.',
				statLabel: 'Systems linked',
				statValue: '14',
				statFootnote: 'Average per multi-team workspace',
			},
			compliance: {
				title: 'Prove trust and compliance',
				description:
					'Collect verification trails, sign-offs, and policy mappings automatically.',
				statLabel: 'Review time',
				statValue: '-68%',
				statFootnote: 'Reduction in security approval cycles',
			},
			adaptive: {
				title: 'Stay adaptive in production',
				description:
					'Monitor real-time usage analytics and trigger automation when content drifts.',
				statLabel: 'Signals captured',
				statValue: '24/7',
				statFootnote: 'Streaming telemetry for every endpoint',
			},
		},
	},
	ja: {
		heading: '知識をインフラと捉える組織のために設計されています',
		intro:
			'Library は編集者・開発者・コンプライアンス担当をひとつの運用モデルでつなぎます。すべてがバージョン管理され、トレースでき、プログラマブルです。',
		bullets: [
			'プロダクト・法務・営業を横断したモデルで常に認識を揃えます。',
			'コンポーネントと API を同じスキーマで揃え、真の再利用を実現します。',
			'インサイトでボトルネックを先読みし、知識のズレを未然に防ぎます。',
		],
		capabilities: {
			launch: {
				title: '新しいナレッジ空間を素早く立ち上げ',
				description:
					'テンプレートと分類・ローカライズ済みの設定から開始し、数日で新しいドメインを公開できます。',
				statLabel: 'ローンチまで',
				statValue: '5 日',
				statFootnote: 'パイロット導入の中央値',
			},
			connect: {
				title: 'システム横断で真実をつなぐ',
				description:
					'CRM や分析基盤、ドキュメントを堅牢なリファレンスと双方向同期で結びます。',
				statLabel: '連携システム',
				statValue: '14 件',
				statFootnote: 'マルチチーム環境の平均値',
			},
			compliance: {
				title: '信頼性とコンプライアンスを証明',
				description: '検証ログ、承認履歴、ポリシーマッピングを自動収集します。',
				statLabel: 'レビュー時間',
				statValue: '-68%',
				statFootnote: 'セキュリティ承認サイクルの削減率',
			},
			adaptive: {
				title: '稼働中の変化にも適応',
				description:
					'利用状況をリアルタイムで監視し、ドリフト検知時に自動アクションを実行します。',
				statLabel: '取得シグナル',
				statValue: '24/7',
				statFootnote: 'すべてのエンドポイントからのストリーミング',
			},
		},
	},
}

export function Capabilities({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section id='capabilities' className='scroll-mt-24 space-y-12'>
			<div
				className={`grid gap-10 lg:grid-cols-[0.95fr_1.05fr] ${fadeInAnimation}`}
			>
				<div className='relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-8 sm:p-10 shadow-[0_25px_70px_rgba(15,23,42,0.55)]'>
					<div className='pointer-events-none absolute -right-12 top-1/2 h-64 w-64 -translate-y-1/2 rounded-full bg-sky-500/20 blur-3xl opacity-70' />
					<h2 className='text-3xl font-semibold text-white sm:text-4xl'>
						{t.heading}
					</h2>
					<p className='mt-5 text-base text-slate-200 sm:text-lg'>{t.intro}</p>
					<ul className='mt-6 space-y-3 text-sm text-slate-200 sm:text-base'>
						{t.bullets.map(line => (
							<li key={line} className='flex items-start gap-3'>
								<span className='mt-1 h-2.5 w-2.5 flex-shrink-0 rounded-full bg-sky-300' />
								{line}
							</li>
						))}
					</ul>
				</div>

				<div className={`grid gap-5 sm:grid-cols-2 ${slideUpAnimation}`}>
					{baseCapabilities.map(capability => {
						const details = t.capabilities[capability.key]
						return (
							<div
								key={capability.key}
								className='relative overflow-hidden rounded-3xl border border-white/8 bg-white/5 p-6 backdrop-blur transition duration-300 hover:border-sky-300/60 hover:bg-white/10'
							>
								<div className='pointer-events-none absolute -right-10 top-10 h-32 w-32 rounded-full bg-sky-500/15 blur-3xl opacity-0 transition duration-500 hover:opacity-80' />
								<div className='relative flex items-center gap-4'>
									<div className='flex h-12 w-12 items-center justify-center rounded-2xl border border-white/15 bg-white/10 text-white'>
										<capability.icon className='h-6 w-6' />
									</div>
									<div>
										<h3 className='text-lg font-semibold text-white'>
											{details.title}
										</h3>
										<p className='mt-2 text-sm text-slate-300'>
											{details.description}
										</p>
									</div>
								</div>
								<dl className='relative mt-6 rounded-2xl border border-white/10 bg-white/5 p-4 text-slate-200'>
									<dt className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
										{details.statLabel}
									</dt>
									<dd className='mt-1 text-2xl font-semibold text-white'>
										{details.statValue}
									</dd>
									<dd className='mt-2 text-xs text-slate-400'>
										{details.statFootnote}
									</dd>
								</dl>
							</div>
						)
					})}
				</div>
			</div>
		</section>
	)
}
