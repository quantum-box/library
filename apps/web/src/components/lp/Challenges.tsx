import type { LpLanguage } from '@/app/lp'
import { fadeInAnimation, slideUpAnimation } from './animations'
import { AlertTriangle, CheckCircle2, RefreshCcw } from 'lucide-react'

const copy: Record<
	LpLanguage,
	{
		title: string
		subtitle: string
		painPoints: string[]
		solutions: string[]
		alignment: string
		alignmentDetail: string
	}
> = {
	en: {
		title: 'Modern teams drown in content yet struggle to trust it',
		subtitle:
			'Library reconnects fractured knowledge ecosystems so product updates, compliance, and go-to-market teams operate from the same truth.',
		painPoints: [
			'Knowledge is duplicated across tools with no single source of truth.',
			'Compliance cannot trace who approved what and when.',
			'API consumers break whenever content structures evolve.',
		],
		solutions: [
			'Every fact links back to its evidence, keeping narratives trustworthy.',
			'Releases run through review states with immutable audit trails.',
			'Developers rely on versioned schemas and change notifications.',
		],
		alignment: 'Hours → Minutes',
		alignmentDetail:
			'Automatic updates broadcast to every channel as changes land.',
	},
	ja: {
		title: '情報はあふれているのに、信頼できる形で使いこなせない',
		subtitle:
			'Library は分断されたナレッジエコシステムをつなぎ直し、プロダクト更新・コンプライアンス・Go-to-market が同じ「事実」を基に動けるようにします。',
		painPoints: [
			'ツールを跨いで情報が複製され、信頼できる唯一の情報源が存在しない。',
			'誰がいつ承認したのかを追跡できず、監査に耐えられない。',
			'コンテンツ構造が変わるたびに API 利用側が破綻してしまう。',
		],
		solutions: [
			'すべての記述に根拠を紐づけ、ストーリーの信頼性を維持します。',
			'リリースはレビュー状態を経て、改ざんできない証跡とともに記録されます。',
			'開発者はバージョン付きスキーマと変更通知により安全に連携できます。',
		],
		alignment: '時間は数時間から数分へ',
		alignmentDetail: '変更が入るたびにすべてのチャネルへ自動で通知されます。',
	},
}

export function Challenges({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section id='challenges' className='scroll-mt-24 space-y-10'>
			<div className='text-center'>
				<p
					className={`text-sm uppercase tracking-[0.3em] text-sky-200/80 ${fadeInAnimation}`}
				>
					Library Insight
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

			<div className={`grid gap-6 lg:grid-cols-2 ${slideUpAnimation}`}>
				<div className='relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-6 sm:p-8 backdrop-blur'>
					<div className='pointer-events-none absolute -right-12 top-12 h-32 w-32 rounded-full bg-rose-500/20 blur-3xl opacity-70' />
					<div className='relative flex items-center gap-3 text-sm font-semibold uppercase tracking-widest text-rose-200'>
						<AlertTriangle className='h-5 w-5' />
						{lang === 'en'
							? 'Where organizations struggle'
							: '組織が抱える課題'}
					</div>
					<ul className='relative mt-6 space-y-4 text-sm text-slate-200 sm:text-base'>
						{t.painPoints.map(point => (
							<li key={point} className='flex items-start gap-3'>
								<span className='mt-1 h-2.5 w-2.5 flex-shrink-0 rounded-full bg-rose-300' />
								{point}
							</li>
						))}
					</ul>
					<div className='relative mt-8 rounded-2xl border border-white/10 bg-white/5 p-4 text-sm text-slate-200'>
						<p className='font-semibold text-white'>
							{lang === 'en' ? 'Without Library' : 'Library がない場合'}
						</p>
						<p className='mt-2 text-slate-300'>
							{lang === 'en'
								? 'Static pages and scattered docs keep teams in constant catch-up mode with no shared visibility into change.'
								: '静的ページと散在したドキュメントでは変更を共有できず、チームは常に後追いになります。'}
						</p>
					</div>
				</div>

				<div className='relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-6 sm:p-8 backdrop-blur'>
					<div className='pointer-events-none absolute -left-16 top-16 h-36 w-36 rounded-full bg-emerald-400/25 blur-3xl opacity-80' />
					<div className='relative flex items-center gap-3 text-sm font-semibold uppercase tracking-widest text-emerald-200'>
						<CheckCircle2 className='h-5 w-5' />
						{lang === 'en' ? 'How Library responds' : 'Library が解決すること'}
					</div>
					<ul className='relative mt-6 space-y-4 text-sm text-slate-200 sm:text-base'>
						{t.solutions.map(solution => (
							<li key={solution} className='flex items-start gap-3'>
								<span className='mt-1 h-2.5 w-2.5 flex-shrink-0 rounded-full bg-emerald-300' />
								{solution}
							</li>
						))}
					</ul>
					<div className='relative mt-8 flex flex-col gap-4 rounded-2xl border border-white/10 bg-white/5 p-4 text-sm text-slate-200 sm:flex-row sm:items-center sm:justify-between'>
						<div>
							<p className='text-xs font-semibold uppercase tracking-widest text-slate-300'>
								{lang === 'en'
									? 'Time to alignment'
									: 'アラインメントまでの時間'}
							</p>
							<p className='mt-1 text-2xl font-semibold text-white'>
								{t.alignment}
							</p>
						</div>
						<div className='flex items-center gap-3 text-xs text-slate-300 sm:text-sm'>
							<RefreshCcw className='h-4 w-4 text-emerald-200' />
							{t.alignmentDetail}
						</div>
					</div>
				</div>
			</div>
		</section>
	)
}
