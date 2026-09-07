import type { LpLanguage } from '@/app/lp'
import { LIBRARY_APP_URL } from './links'
import { ArrowRight } from 'lucide-react'
import { fadeInAnimation } from './animations'
import { KnowledgeGlobe } from './KnowledgeGlobe'

const copy = {
	en: {
		badge: 'Private Preview',
		heading: ['A living knowledge base', 'that keeps pace', 'with your teams'],
		description:
			'Library connects documents, people, and sources so every update is captured, auditable, and reusable across products.',
		primaryCta: 'Get started',
		secondaryCta: 'Explore the product',
		tocTitle: 'Contents',
		toc: [
			{ href: '#features', label: 'Features' },
			{ href: '#capabilities', label: 'Capabilities' },
			{ href: '#challenges', label: 'Challenges & answers' },
			{ href: '#pricing', label: 'Pricing' },
			{ href: '#roadmap', label: 'Roadmap' },
		],
	},
	ja: {
		badge: 'プライベートプレビュー',
		heading: ['チームとともに', '進化する、', '生きたナレッジ基盤'],
		description:
			'Library はドキュメント・人・出典を結びつけ、すべての更新を記録・監査可能にしながら、プロダクト全体で再利用できる状態に整えます。',
		primaryCta: '導入を始める',
		secondaryCta: 'プロダクトを知る',
		tocTitle: '目次',
		toc: [
			{ href: '#features', label: '特徴' },
			{ href: '#capabilities', label: '提供価値' },
			{ href: '#challenges', label: '課題と解決' },
			{ href: '#pricing', label: '料金' },
			{ href: '#roadmap', label: 'ロードマップ' },
		],
	},
} satisfies Record<
	LpLanguage,
	{
		badge: string
		heading: string[]
		description: string
		primaryCta: string
		secondaryCta: string
		tocTitle: string
		toc: { href: string; label: string }[]
	}
>

export function Hero({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section className={`py-16 sm:py-20 lg:py-24 ${fadeInAnimation}`}>
			<div className='grid items-start gap-12 lg:grid-cols-[1.1fr_0.9fr] lg:gap-16'>
				<div>
					<p className='inline-flex items-center gap-2 rounded-md border border-slate-200 px-3 py-1 font-mono text-xs uppercase tracking-[0.2em] text-slate-600'>
						<span className='h-1.5 w-1.5 rounded-full bg-emerald-500' />
						{t.badge}
					</p>
					<h1 className='mt-6 font-display text-4xl leading-[1.2] text-slate-900 sm:text-5xl'>
						{t.heading.map((segment, index) => (
							<span key={segment} className='inline-block'>
								{segment}
								{index < t.heading.length - 1 && lang === 'en'
									? ' '
									: ''}
							</span>
						))}
					</h1>
					<p className='mt-6 max-w-xl text-base leading-relaxed text-slate-600 sm:text-lg'>
						{t.description}
					</p>
					<div className='mt-8 flex flex-col gap-3 sm:flex-row'>
						<a
							href={LIBRARY_APP_URL}
							className='inline-flex items-center justify-center gap-2 rounded-md bg-blue-700 px-6 py-3 text-sm font-medium text-white transition hover:bg-blue-800'
						>
							{t.primaryCta}
							<ArrowRight className='h-4 w-4' />
						</a>
						<a
							href='#features'
							className='inline-flex items-center justify-center gap-2 rounded-md border border-slate-300 px-6 py-3 text-sm font-medium text-slate-700 transition hover:border-slate-400 hover:text-slate-900'
						>
							{t.secondaryCta}
						</a>
					</div>

					<nav
						aria-label={t.tocTitle}
						className='mt-12 max-w-md border-t border-slate-200 pt-6'
					>
						<p className='font-mono text-xs uppercase tracking-[0.2em] text-slate-500'>
							{t.tocTitle}
						</p>
						<ol className='mt-3'>
							{t.toc.map((item, index) => (
								<li key={item.href}>
									<a
										href={item.href}
										className='group flex items-baseline gap-3 py-1.5'
									>
										<span className='font-mono text-xs text-blue-700'>
											{String(index + 1).padStart(2, '0')}
										</span>
										<span className='text-sm text-slate-700 transition group-hover:text-blue-700'>
											{item.label}
										</span>
										<span className='mb-1 flex-1 border-b border-dotted border-slate-300' />
									</a>
								</li>
							))}
						</ol>
					</nav>
				</div>

				<KnowledgeGlobe lang={lang} />
			</div>
		</section>
	)
}
