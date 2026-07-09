import type { LpLanguage } from '@/app/lp'
import { Check, X } from 'lucide-react'
import { SectionHeader } from './SectionHeader'

const copy: Record<
	LpLanguage,
	{
		eyebrow: string
		title: string
		subtitle: string
		painTitle: string
		painPoints: string[]
		painNoteTitle: string
		painNote: string
		solutionTitle: string
		solutions: string[]
		alignmentLabel: string
		alignment: string
		alignmentDetail: string
	}
> = {
	en: {
		eyebrow: 'Challenges & answers',
		title: 'Modern teams drown in content yet struggle to trust it',
		subtitle:
			'Library reconnects fractured knowledge ecosystems so product updates, compliance, and go-to-market teams operate from the same truth.',
		painTitle: 'Where organizations struggle',
		painPoints: [
			'Knowledge is duplicated across tools with no single source of truth.',
			'Compliance cannot trace who approved what and when.',
			'API consumers break whenever content structures evolve.',
		],
		painNoteTitle: 'Without Library',
		painNote:
			'Static pages and scattered docs keep teams in constant catch-up mode with no shared visibility into change.',
		solutionTitle: 'How Library responds',
		solutions: [
			'Every fact links back to its evidence, keeping narratives trustworthy.',
			'Releases run through review states with immutable audit trails.',
			'Developers rely on versioned schemas and change notifications.',
		],
		alignmentLabel: 'Time to alignment',
		alignment: 'Hours → Minutes',
		alignmentDetail:
			'Automatic updates broadcast to every channel as changes land.',
	},
	ja: {
		eyebrow: '課題と解決',
		title: '情報はあふれているのに、信頼できる形で使いこなせない',
		subtitle:
			'Library は分断されたナレッジエコシステムをつなぎ直し、プロダクト更新・コンプライアンス・Go-to-market が同じ「事実」を基に動けるようにします。',
		painTitle: '組織が抱える課題',
		painPoints: [
			'ツールを跨いで情報が複製され、信頼できる唯一の情報源が存在しない。',
			'誰がいつ承認したのかを追跡できず、監査に耐えられない。',
			'コンテンツ構造が変わるたびに API 利用側が破綻してしまう。',
		],
		painNoteTitle: 'Library がない場合',
		painNote:
			'静的ページと散在したドキュメントでは変更を共有できず、チームは常に後追いになります。',
		solutionTitle: 'Library が解決すること',
		solutions: [
			'すべての記述に根拠を紐づけ、ストーリーの信頼性を維持します。',
			'リリースはレビュー状態を経て、改ざんできない証跡とともに記録されます。',
			'開発者はバージョン付きスキーマと変更通知により安全に連携できます。',
		],
		alignmentLabel: 'アラインメントまでの時間',
		alignment: '数時間 → 数分',
		alignmentDetail: '変更が入るたびにすべてのチャネルへ自動で通知されます。',
	},
}

export function Challenges({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<section
			id='challenges'
			className='scroll-mt-24 border-t border-slate-200 py-16 sm:py-20'
		>
			<SectionHeader
				chapter='03'
				eyebrow={t.eyebrow}
				title={t.title}
				lead={t.subtitle}
			/>

			<div className='mt-12 grid gap-6 lg:grid-cols-2'>
				<div className='rounded-lg border border-slate-200 bg-slate-50 p-6 sm:p-8'>
					<p className='font-mono text-xs uppercase tracking-[0.2em] text-slate-500'>
						{t.painTitle}
					</p>
					<ul className='mt-6 space-y-4'>
						{t.painPoints.map(point => (
							<li
								key={point}
								className='flex items-start gap-3 text-sm leading-relaxed text-slate-700 sm:text-base'
							>
								<X className='mt-1 h-4 w-4 shrink-0 text-rose-500' />
								{point}
							</li>
						))}
					</ul>
					<div className='mt-8 border-t border-slate-200 pt-4'>
						<p className='text-sm font-semibold text-slate-700'>
							{t.painNoteTitle}
						</p>
						<p className='mt-1.5 text-sm leading-relaxed text-slate-500'>
							{t.painNote}
						</p>
					</div>
				</div>

				<div className='rounded-lg border border-slate-200 bg-white p-6 sm:p-8'>
					<p className='font-mono text-xs uppercase tracking-[0.2em] text-emerald-700'>
						{t.solutionTitle}
					</p>
					<ul className='mt-6 space-y-4'>
						{t.solutions.map(solution => (
							<li
								key={solution}
								className='flex items-start gap-3 text-sm leading-relaxed text-slate-700 sm:text-base'
							>
								<Check className='mt-1 h-4 w-4 shrink-0 text-emerald-600' />
								{solution}
							</li>
						))}
					</ul>
					<div className='mt-8 flex flex-col gap-2 border-t border-slate-200 pt-4 sm:flex-row sm:items-end sm:justify-between'>
						<div>
							<p className='font-mono text-xs uppercase tracking-wide text-slate-400'>
								{t.alignmentLabel}
							</p>
							<p className='mt-1 font-display text-2xl text-slate-900'>
								{t.alignment}
							</p>
						</div>
						<p className='max-w-[16rem] text-xs leading-relaxed text-slate-500 sm:text-right'>
							{t.alignmentDetail}
						</p>
					</div>
				</div>
			</div>
		</section>
	)
}
