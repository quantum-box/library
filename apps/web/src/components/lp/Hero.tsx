import { Button } from '@/components/ui/button'
import type { LpLanguage } from '@/app/lp'
import { ArrowRight, Play, Sparkles } from 'lucide-react'
import Link from 'next/link'
import { fadeInAnimation, slideUpAnimation } from './animations'

const copy = {
	en: {
		badge: 'Private Preview',
		heading: 'Build a living knowledge base that keeps pace with your teams',
		description:
			'Library connects documents, people, and sources so every update is captured, auditable, and reusable across products.',
		bullets: [
			'Map relationships between entries and surface trusted sources without manual tagging.',
			'Versioned APIs keep every consumer synchronized while you ship continuously.',
		],
		primaryCta: 'Get Started',
		secondaryCta: 'Watch Product Walkthrough',
		metrics: [
			{
				title: 'Knowledge Coverage',
				value: '92%',
				subtitle: 'Median across pilot teams after 30 days',
			},
			{
				title: 'Audit Time',
				value: '-68%',
				subtitle: 'Reduction in compliance review effort',
			},
			{
				title: 'API Adoption',
				value: '4.6x',
				subtitle: 'Increase in downstream integrations',
			},
		],
		card: {
			title: 'Living Documentation',
			description: 'Track schema changes and stakeholder notes side-by-side.',
			metaLeft: '8 linked references',
			metaRight: 'Last update 18 min ago',
			relationships: {
				title: 'Relationships',
				subtitle: 'Spanning teams, products, and policy docs',
			},
			review: {
				title: 'Review Status',
				subtitle: 'All stakeholders approved latest revision',
			},
		},
	},
	ja: {
		badge: 'プライベートプレビュー',
		heading: 'チームとともに進化するナレッジ基盤を構築する',
		description:
			'Library はドキュメント・人・参考情報を結び、すべての更新を記録・監査可能にしながら、プロダクト全体で再利用できる状態に整えます。',
		bullets: [
			'エントリー間の関係性と出典を自動で可視化し、信頼できる情報だけを表面化します。',
			'バージョン管理された API で配信先を常に同期させ、継続的な公開を後押しします。',
		],
		primaryCta: '導入を始める',
		secondaryCta: 'プロダクト紹介を見る',
		metrics: [
			{
				title: 'ナレッジカバレッジ',
				value: '92%',
				subtitle: '導入30日後のパイロットチーム中央値',
			},
			{
				title: '監査対応時間',
				value: '-68%',
				subtitle: 'コンプライアンスレビュー工数の削減率',
			},
			{
				title: 'API 利用拡大',
				value: '4.6x',
				subtitle: '下流システム連携数の増加',
			},
		],
		card: {
			title: 'ライブドキュメント',
			description: 'スキーマの変更履歴と関係者メモを一元管理できます。',
			metaLeft: '参照リンク 8 件',
			metaRight: '最終更新 18 分前',
			relationships: {
				title: 'リレーション',
				subtitle: 'チーム・プロダクト・規程ドキュメントを横断',
			},
			review: {
				title: 'レビュー状況',
				subtitle: '全ステークホルダーが最新稿を承認済み',
			},
		},
	},
} satisfies Record<
	LpLanguage,
	{
		badge: string
		heading: string
		description: string
		bullets: [string, string]
		primaryCta: string
		secondaryCta: string
		metrics: { title: string; value: string; subtitle: string }[]
		card: {
			title: string
			description: string
			metaLeft: string
			metaRight: string
			relationships: { title: string; subtitle: string }
			review: { title: string; subtitle: string }
		}
	}
>

export function Hero({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section className='relative overflow-hidden rounded-[3rem] border border-white/5 bg-white/5 px-6 py-16 text-left shadow-[0_35px_120px_rgba(15,23,42,0.55)] backdrop-blur-xl sm:px-10 sm:py-20 lg:px-16'>
			<div className='pointer-events-none absolute -left-24 top-16 h-64 w-64 rounded-full bg-cyan-400/30 blur-3xl opacity-70' />
			<div className='pointer-events-none absolute right-0 top-0 h-72 w-72 translate-x-1/4 rounded-full bg-blue-500/30 blur-3xl opacity-60' />
			<div className='relative grid items-center gap-12 lg:grid-cols-[1.05fr_0.95fr]'>
				<div className='space-y-8'>
					<div
						className={`inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/10 px-3 py-1 text-xs font-semibold uppercase tracking-widest text-sky-200/90 ${fadeInAnimation}`}
					>
						<span className='inline-flex h-2 w-2 rounded-full bg-emerald-400' />
						{t.badge}
					</div>
					<div className={`${fadeInAnimation} delay-150 space-y-6`}>
						<h2 className='text-4xl font-semibold leading-[1.05] text-white sm:text-5xl md:text-6xl'>
							{t.heading}
						</h2>
						<p className='max-w-xl text-base text-slate-200 sm:text-lg md:text-xl'>
							{t.description}
						</p>
					</div>
					<div
						className={`space-y-4 text-sm text-slate-200 sm:text-base ${slideUpAnimation}`}
					>
						<div className='flex items-start gap-3 rounded-2xl border border-white/10 bg-white/5 p-4 backdrop-blur'>
							<Sparkles className='mt-1 h-5 w-5 text-sky-300' />
							<p>{t.bullets[0]}</p>
						</div>
						<div className='flex items-start gap-3 rounded-2xl border border-white/10 bg-white/5 p-4 backdrop-blur'>
							<ArrowRight className='mt-1 h-5 w-5 text-sky-300' />
							<p>{t.bullets[1]}</p>
						</div>
					</div>
					<div
						className={`flex flex-col gap-4 pt-2 sm:flex-row ${slideUpAnimation} delay-200`}
					>
						<Button
							className='inline-flex items-center justify-center gap-2 rounded-full bg-gradient-to-r from-sky-500 via-blue-600 to-blue-700 px-6 py-3 text-base font-semibold text-white shadow-[0_20px_45px_rgba(37,99,235,0.45)] transition hover:shadow-[0_24px_55px_rgba(37,99,235,0.5)] sm:px-8 sm:py-4'
							asChild
						>
							<Link href='/sign_up'>
								{t.primaryCta}
								<ArrowRight className='h-5 w-5' />
							</Link>
						</Button>
						<Button
							variant='outline'
							className='inline-flex items-center justify-center gap-2 rounded-full border-white/30 bg-white/5 px-6 py-3 text-base font-semibold text-white transition hover:border-white/60 hover:bg-white/10 sm:px-8 sm:py-4'
						>
							<Play className='h-5 w-5 text-sky-200' />
							{t.secondaryCta}
						</Button>
					</div>
					<div
						className={`grid gap-4 pt-6 text-left sm:grid-cols-3 ${slideUpAnimation} delay-300`}
					>
						{t.metrics.map(metric => (
							<div
								key={metric.title}
								className='rounded-2xl border border-white/10 bg-white/5 px-4 py-5'
							>
								<p className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
									{metric.title}
								</p>
								<p className='mt-2 text-2xl font-semibold text-white sm:text-3xl'>
									{metric.value}
								</p>
								<p className='mt-1 text-xs text-slate-300'>{metric.subtitle}</p>
							</div>
						))}
					</div>
				</div>

				<div
					className={`relative lg:justify-self-end ${fadeInAnimation} delay-200`}
				>
					<div className='pointer-events-none absolute -inset-x-16 -inset-y-12 rounded-[3rem] bg-gradient-to-r from-sky-500/20 via-indigo-500/15 to-transparent blur-3xl opacity-70' />
					<div className='relative w-full max-w-lg rounded-[2.5rem] border border-white/10 bg-slate-950/70 p-6 shadow-[0_35px_90px_rgba(37,99,235,0.35)] backdrop-blur-xl'>
						<div className='flex items-center justify-between border-b border-white/10 pb-4 text-xs font-semibold uppercase tracking-widest text-slate-300'>
							<span>Knowledge Graph</span>
							<span className='inline-flex items-center gap-2 text-emerald-300'>
								<span className='h-2 w-2 rounded-full bg-emerald-400' />
								Synced
							</span>
						</div>
						<div className='mt-6 space-y-4'>
							<div className='rounded-2xl border border-white/10 bg-white/5 p-4'>
								<p className='text-sm font-semibold text-white'>
									{t.card.title}
								</p>
								<p className='mt-1 text-sm text-slate-300'>
									{t.card.description}
								</p>
								<div className='mt-3 flex items-center gap-3 text-xs text-slate-300'>
									<span className='inline-flex items-center gap-1'>
										<ArrowRight className='h-3.5 w-3.5 text-sky-300' />
										{t.card.metaLeft}
									</span>
									<span className='inline-flex items-center gap-1'>
										<Sparkles className='h-3.5 w-3.5 text-amber-300' />
										{t.card.metaRight}
									</span>
								</div>
							</div>
							<div className='grid gap-3 sm:grid-cols-2'>
								<div className='rounded-2xl border border-white/10 bg-white/5 p-4'>
									<p className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
										{t.card.relationships.title}
									</p>
									<p className='mt-2 text-xl font-semibold text-white'>112</p>
									<p className='mt-1 text-xs text-slate-400'>
										{t.card.relationships.subtitle}
									</p>
								</div>
								<div className='rounded-2xl border border-white/10 bg-white/5 p-4'>
									<p className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
										{t.card.review.title}
									</p>
									<p className='mt-2 text-xl font-semibold text-white'>Ready</p>
									<p className='mt-1 text-xs text-slate-400'>
										{t.card.review.subtitle}
									</p>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</section>
	)
}
