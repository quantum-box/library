import { Card } from '@/components/ui/card'
import type { LpLanguage } from '@/app/lp'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { CheckCircle2 } from 'lucide-react'
import { fadeInAnimation, slideUpAnimation } from './animations'

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
		title: string
		subtitle: string
		planLabels: Record<PlanKey, string>
		plans: Record<PlanKey, Plan>
		footers: FooterItem[]
	}
> = {
	en: {
		title: 'Scale knowledge without guessing the bill',
		subtitle:
			'Library lets you match billing to how teams consume and create knowledge. Start simple with request counts or dial into resource-based units.',
		planLabels: { rc: 'Request Count', ru: 'Request Unit' },
		plans: {
			rc: {
				title: 'RC (Request Count)',
				subtitle: 'Predictable usage-based billing for event-driven workloads.',
				highlight: 'Expected: 2-10 JPY / RC',
				bullets: [
					'Straightforward budgeting with pay-per-request transparency.',
					'Volume tiers adapt to traffic bursts and seasonal peaks.',
					'No idle fees—pause services without paying for unused capacity.',
				],
			},
			ru: {
				title: 'RU (Request Unit)',
				subtitle: 'Granular billing aligned to compute, storage, and AI usage.',
				highlight: 'Precise control for intensive workloads',
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
		title: '料金を予測しながらナレッジを拡張する',
		subtitle:
			'Library はチームの利用実態に合わせた課金モデルを選べます。まずはリクエスト数ベースで始め、必要に応じてリソース単位まで細かく制御できます。',
		planLabels: { rc: 'RC（リクエスト数）', ru: 'RU（リクエスト単位）' },
		plans: {
			rc: {
				title: 'RC（Request Count）',
				subtitle: 'イベントドリブンなワークロード向けのシンプルな従量課金。',
				highlight: '想定単価: 1RC あたり 2〜10 円',
				bullets: [
					'リクエスト単位の明瞭な課金で予算管理を簡単にします。',
					'ピークや季節要因に合わせてボリューム階層が自動調整されます。',
					'アイドル時の費用はゼロ。停止すると余分なコストは発生しません。',
				],
			},
			ru: {
				title: 'RU（Request Unit）',
				subtitle: '計算・ストレージ・AI 利用量に連動する細かな課金モデル。',
				highlight: '高負荷ワークロードも細かく制御',
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
		<section id='pricing' className='scroll-mt-24 space-y-12'>
			<div className='text-center'>
				<p
					className={`text-sm uppercase tracking-[0.3em] text-sky-200/80 ${fadeInAnimation}`}
				>
					Pricing
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

			<Tabs
				defaultValue='rc'
				className={`mx-auto w-full max-w-4xl ${slideUpAnimation}`}
			>
				<TabsList className='grid w-full grid-cols-2 rounded-full border border-white/10 bg-white/5 p-1 text-slate-200'>
					{planOrder.map(plan => (
						<TabsTrigger
							key={plan}
							value={plan}
							className='rounded-full px-4 py-3 text-sm font-semibold transition data-[state=active]:bg-white data-[state=active]:text-slate-900'
						>
							{t.planLabels[plan]}
						</TabsTrigger>
					))}
				</TabsList>
				{planOrder.map(plan => {
					const planCopy = t.plans[plan]
					return (
						<TabsContent key={plan} value={plan} className='mt-6'>
							<Card className='relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-6 sm:p-8 backdrop-blur shadow-[0_25px_70px_rgba(15,23,42,0.55)]'>
								<div className='pointer-events-none absolute -right-20 top-0 h-48 w-48 rounded-full bg-sky-500/20 blur-3xl opacity-70' />
								<div className='relative space-y-4 text-left'>
									<p className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
										{planCopy.title}
									</p>
									<h3 className='text-2xl font-semibold text-white sm:text-3xl'>
										{planCopy.subtitle}
									</h3>
									<div className='inline-flex rounded-full border border-sky-400/40 bg-sky-500/10 px-4 py-1.5 text-sm font-semibold text-sky-100'>
										{planCopy.highlight}
									</div>
								</div>
								<ul className='relative mt-6 space-y-3 text-sm text-slate-200 sm:text-base'>
									{planCopy.bullets.map(item => (
										<li
											key={item}
											className='flex items-start gap-3 rounded-2xl border border-white/10 bg-white/5 p-4'
										>
											<CheckCircle2 className='mt-0.5 h-5 w-5 text-emerald-300' />
											<span>{item}</span>
										</li>
									))}
								</ul>
								<div className='relative mt-8 grid gap-4 rounded-2xl border border-white/10 bg-white/5 p-4 text-xs text-slate-300 sm:grid-cols-3 sm:text-sm'>
									{t.footers.map(footer => (
										<div key={footer.title}>
											<p className='font-semibold text-white'>{footer.title}</p>
											<p className='mt-1'>{footer.body}</p>
										</div>
									))}
								</div>
							</Card>
						</TabsContent>
					)
				})}
			</Tabs>
		</section>
	)
}
