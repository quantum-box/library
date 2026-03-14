import { ImageResponse } from 'next/og'
import { OgBase } from '../../../../(open-graph)/components/og-base'
import { sanitizePathParam } from '../../../_utils/sanitize'

export const runtime = 'edge'

export async function GET(
	req: Request,
	{ params: { org, repo } }: { params: { org: string; repo: string } },
) {
	// Sanitize URL parameters to prevent path traversal attacks
	const sanitizedOrg = sanitizePathParam(org)
	const sanitizedRepo = sanitizePathParam(repo)

	const { searchParams } = new URL(req.url)

	// Get data from search params (set by generateMetadata)
	const name = searchParams.get('name') || sanitizedRepo
	const description = searchParams.get('description') || ''
	const isPublic = searchParams.get('public') !== 'false'
	const dataCount = searchParams.get('data') || '0'
	const contributorCount = searchParams.get('contributors') || '0'
	const tagsParam = searchParams.get('tags')
	const tags = tagsParam ? tagsParam.split(',').filter(Boolean) : []

	return new ImageResponse(
		<OgBase
			type='repository'
			path={`${sanitizedOrg} / ${sanitizedRepo}`}
			title={name}
			description={description || undefined}
			badge={{
				text: isPublic ? 'Public' : 'Private',
				variant: isPublic ? 'public' : 'private',
			}}
			tags={tags}
			stats={[
				{ label: 'data', value: dataCount, icon: 'data' },
				{
					label: 'contributors',
					value: contributorCount,
					icon: 'contributors',
				},
			]}
		/>,
		{
			width: 1200,
			height: 630,
		},
	)
}
