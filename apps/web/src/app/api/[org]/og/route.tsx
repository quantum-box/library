import { ImageResponse } from 'next/og'
import { OgBase } from '../../../(open-graph)/components/og-base'
import { sanitizePathParam } from '../../_utils/sanitize'

export const runtime = 'edge'

export async function GET(
	req: Request,
	{ params: { org } }: { params: { org: string } },
) {
	// Sanitize URL parameters to prevent path traversal attacks
	const sanitizedOrg = sanitizePathParam(org)

	const { searchParams } = new URL(req.url)

	// Get data from search params (set by generateMetadata)
	const name = searchParams.get('name') || sanitizedOrg
	const description = searchParams.get('description') || ''
	const repoCount = searchParams.get('repos') || '0'
	const memberCount = searchParams.get('members') || '0'

	return new ImageResponse(
		<OgBase
			type='organization'
			title={name}
			description={description || undefined}
			stats={[
				{ label: 'repositories', value: repoCount, icon: 'repos' },
				{ label: 'members', value: memberCount, icon: 'members' },
			]}
		/>,
		{
			width: 1200,
			height: 630,
		},
	)
}
