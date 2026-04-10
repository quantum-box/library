import { Link, useRouterState } from '@tanstack/react-router'

export function Breadcrumbs() {
	const pathname = useRouterState({ select: (s) => s.location.pathname })
	const segments = pathname.split('/').filter(Boolean)
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
							to={`/${segments.slice(0, ix + 1).join('/')}`}
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
