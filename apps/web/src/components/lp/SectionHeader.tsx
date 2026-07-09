export function SectionHeader({
	chapter,
	eyebrow,
	title,
	lead,
}: {
	chapter: string
	eyebrow: string
	title: string
	lead?: string
}) {
	return (
		<div className='max-w-3xl'>
			<p className='font-mono text-xs uppercase tracking-[0.2em] text-slate-500'>
				<span className='text-blue-700'>{chapter}</span>
				<span className='mx-3 text-slate-300'>/</span>
				{eyebrow}
			</p>
			<h2 className='mt-4 font-display text-3xl leading-tight text-slate-900 sm:text-4xl'>
				{title}
			</h2>
			{lead ? (
				<p className='mt-4 text-base leading-relaxed text-slate-600 sm:text-lg'>
					{lead}
				</p>
			) : null}
		</div>
	)
}
