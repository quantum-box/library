import Link from 'next/link'

export async function BreadcrumbItem({
	segment,
	segments,
	ix,
}: {
	segment: string // id
	segments: string[]
	ix: number
}) {
	// ここでid to name
	return (
		<Link
			href={`/${segments.slice(0, ix + 1).join('/')}`}
			key={segment}
			className='hover:bg-gray-200 rounded font-semibold px-2'
		>
			{segment}
		</Link>
	)
}
