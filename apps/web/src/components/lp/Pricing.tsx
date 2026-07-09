import type { LpLanguage } from '@/app/lp'
import { Check } from 'lucide-react'
import { SectionHeader } from './SectionHeader'

type PlanKey = 'rc' | 'ru'

type Plan = {
	title: string
	subtitle: string
	highlight: string
	bullets: string[]
}

type FooterItem = {
	title: string
	body: string
}

const planOrder: PlanKey[] = ['rc', 'ru']

const copy: Record<
	LpLanguage,
	{
		eyebrow: string
		title: string
		subtitle: string
		plans: Record<PlanKey, Plan>
		footers: FooterItem[]
	}
> = {
	en: {
		eyebrow: 'Pricing',
		title: 'Scale knowledge without guessing the bill',
		subtitle:
			'Library lets you match billing to how teams consume and create knowledge. Start simple with request counts or dial into resource-based units.',
		plans: {
			rc: {
				title: 'RC (Request Count)',
				subtitle: 'Predictable usage-based billing for event-driven workloads.',
				highlight: '2–10 JPY / RC',
				bullets: [
					'Straightforward budgeting with pay-per-request transparency.',
					'Volume tiers adapt to traffic bursts and seasonal peaks.',
					'No idle fees—pause services without paying for unused capacity.',
				],
			},
			ru: {
				title: 'RU (Request Unit)',
				subtitle: 'Granular billing aligned to compute, storage, and AI usage.',
				highlight: 'Metered by resource class',
				bullets: [
					'Metered resource classes let you optimize for latency or cost.',
					'Audit tokens, embeddings, and transformations in one dashboard.',
					'Enterprise guardrails with budget alerts and anomaly detection.',
				],
			},
		},
		footers: [
			{
				title: 'Launch with clarity',
				body: 'Usage explorer and scenario simulator included.',
			},
			{
				title: 'Enterprise ready',
				body: 'SOC2, regional residency, custom invoicing.',
			},
			{
				title: 'Open roadmap',
				body: 'Open-source plans with transparent maintainer compensation.',
			},
		],
	},
	ja: {
		eyebrow: '料金',
		title: '料金を予測しながらナレッジを拡張する',
		subtitle:
			'Library はチームの利用実態に合わせた課金モデルを選べます。まずはリクエスト数ベースで始め、必要に応じてリソース単位まで細かく制御できます。',
		plans: {
			rc: {
				title: 'RC（Request Count）',
				subtitle: 'イベントドリブンなワークロード向けのシンプルな従量課金。',
				highlight: '1RC あたり 2〜10 円',
				bullets: [
					'リクエスト単位の明瞭な課金で予算管理を簡単にします。',
					'ピークや季節要因に合わせてボリューム階層が自動調整されます。',
					'アイドル時の費用はゼロ。停止すると余分なコストは発生しません。',
				],
			},
			ru: {
				title: 'RU（Request Unit）',
				subtitle: '計算・ストレージ・AI 利用量に連動する細かな課金モデル。',
				highlight: 'リソースクラス単位の従量制',
				bullets: [
					'リソースクラスごとのメーターでレイテンシーとコストを両立。',
					'トークン・埋め込み・変換処理を一つのダッシュボードで監査。',
					'異常検知と予算アラートでエンタープライズ要件に対応。',
				],
			},
		},
		footers: [
			{
				title: 'スムーズな立ち上げ',
				body: '利用状況シミュレーターを同梱しています。',
			},
			{
				title: 'エンタープライズ対応',
				body: 'SOC2、リージョン指定、請求書払いに対応。',
			},
			{
				title: 'オープンな開発',
				body: 'OSS ロードマップとメンテナ補償を公開予定。',
			},
		],
	},
}

export function Pricing({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section
			id='pricing'
			className='scroll-mt-24 border-t border-slate-200 py-16 sm:py-20'
		>
			<SectionHeader
				chapter='04'
				eyebrow={t.eyebrow}
				title={t.title}
				lead={t.subtitle}
			/>

			<div className='mt-12 grid gap-6 md:grid-cols-2'>
				{planOrder.map(plan => {
					const planCopy = t.plans[plan]
					return (
						<div
							key={plan}
							className='flex flex-col rounded-lg border border-slate-200 bg-white p-6 sm:p-8'
						>
							<p className='font-mono text-xs uppercase tracking-[0.2em] text-blue-700'>
								{plan.toUpperCase()}
							</p>
							<h3 className='mt-3 text-lg font-semibold text-slate-900'>
								{planCopy.title}
							</h3>
							<p className='mt-1.5 text-sm leading-relaxed text-slate-600'>
								{planCopy.subtitle}
							</p>
							<p className='mt-5 font-display text-2xl text-slate-900'>
								{planCopy.highlight}
							</p>
							<ul className='mt-6 space-y-3 border-t border-slate-200 pt-5'>
								{planCopy.bullets.map(item => (
									<li
										key={item}
										className='flex items-start gap-3 text-sm leading-relaxed text-slate-700'
									>
										<Check className='mt-0.5 h-4 w-4 shrink-0 text-emerald-600' />
										{item}
									</li>
								))}
							</ul>
						</div>
					)
				})}
			</div>

			<div className='mt-8 grid gap-6 border-t border-slate-200 pt-6 sm:grid-cols-3'>
				{t.footers.map(footer => (
					<div key={footer.title}>
						<p className='text-sm font-semibold text-slate-900'>
							{footer.title}
						</p>
						<p className='mt-1 text-sm leading-relaxed text-slate-600'>
							{footer.body}
						</p>
					</div>
				))}
			</div>
		</section>
	)
}
