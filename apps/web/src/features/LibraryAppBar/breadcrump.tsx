'use client'
import Link from 'next/link'
import { useSelectedLayoutSegments } from 'next/navigation'

export function Breadcrumbs() {
	const segments = useSelectedLayoutSegments()
	return (
		<div className='h-12 justify-start items-center inline-flex text-sm invisible md:visible'>
			{segments
				.filter(
					segment => !segment.startsWith('sign_in') && !segment.startsWith('('),
				)
				.slice(0, 2)
				.map((segment, ix) => (
					<span key={segment} className='flex'>
						<Link
							href={`/${segments.slice(0, ix + 1).join('/')}`}
							key={segment}
							className='hover:bg-gray-200 rounded font-semibold px-2'
						>
							{segment}
						</Link>
						{ix < 1 && <span>/</span>}
					</span>
				))}
		</div>
	)
}
