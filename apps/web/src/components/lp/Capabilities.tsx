import type { LpLanguage } from '@/app/lp'
import { Check } from 'lucide-react'
import { SectionHeader } from './SectionHeader'

const capabilityOrder = ['launch', 'connect', 'compliance', 'adaptive'] as const

type CapabilityKey = (typeof capabilityOrder)[number]

type CapabilityCopy = {
	title: string
	description: string
}

const copy: Record<
	LpLanguage,
	{
		eyebrow: string
		heading: string
		intro: string
		bullets: string[]
		capabilities: Record<CapabilityKey, CapabilityCopy>
	}
> = {
	en: {
		eyebrow: 'Capabilities',
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
			},
			connect: {
				title: 'Connect truth across systems',
				description:
					'Bridge CRM, analytics, and documentation with resilient references and bi-directional sync.',
			},
			compliance: {
				title: 'Prove trust and compliance',
				description:
					'Collect verification trails, sign-offs, and policy mappings automatically.',
			},
			adaptive: {
				title: 'Stay adaptive in production',
				description:
					'Monitor real-time usage analytics and trigger automation when content drifts.',
			},
		},
	},
	ja: {
		eyebrow: '提供価値',
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
			},
			connect: {
				title: 'システム横断で真実をつなぐ',
				description:
					'CRM や分析基盤、ドキュメントを堅牢なリファレンスと双方向同期で結びます。',
			},
			compliance: {
				title: '信頼性とコンプライアンスを証明',
				description: '検証ログ、承認履歴、ポリシーマッピングを自動収集します。',
			},
			adaptive: {
				title: '稼働中の変化にも適応',
				description:
					'利用状況をリアルタイムで監視し、ドリフト検知時に自動アクションを実行します。',
			},
		},
	},
}

export function Capabilities({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section
			id='capabilities'
			className='scroll-mt-24 border-t border-slate-200 py-16 sm:py-20'
		>
			<div className='grid gap-12 lg:grid-cols-[0.9fr_1.1fr] lg:gap-16'>
				<div>
					<SectionHeader
						chapter='02'
						eyebrow={t.eyebrow}
						title={t.heading}
						lead={t.intro}
					/>
					<ul className='mt-8 space-y-3'>
						{t.bullets.map(line => (
							<li
								key={line}
								className='flex items-start gap-3 text-sm leading-relaxed text-slate-700 sm:text-base'
							>
								<Check className='mt-1 h-4 w-4 shrink-0 text-emerald-600' />
								{line}
							</li>
						))}
					</ul>
				</div>

				<div className='divide-y divide-slate-200 border-y border-slate-200 lg:mt-2'>
					{capabilityOrder.map((key, index) => {
						const details = t.capabilities[key]
						return (
							<div key={key} className='flex gap-5 py-6'>
								<span className='w-9 shrink-0 pt-0.5 font-mono text-sm text-blue-700'>
									2.{index + 1}
								</span>
								<div>
									<h3 className='text-base font-semibold text-slate-900'>
										{details.title}
									</h3>
									<p className='mt-1.5 text-sm leading-relaxed text-slate-600'>
										{details.description}
									</p>
								</div>
							</div>
						)
					})}
				</div>
			</div>
		</section>
	)
}
