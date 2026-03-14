import { ImageResponse } from 'next/og'
import { OgBase } from '../../../../../(open-graph)/components/og-base'
import { sanitizePathParam } from '../../../../_utils/sanitize'

export const runtime = 'edge'

export async function GET(
	req: Request,
	{
		params: { org, repo, dataId },
	}: { params: { org: string; repo: string; dataId: string } },
) {
	// Sanitize URL parameters to prevent path traversal attacks
	const sanitizedOrg = sanitizePathParam(org)
	const sanitizedRepo = sanitizePathParam(repo)
	const sanitizedDataId = sanitizePathParam(dataId)

	const { searchParams } = new URL(req.url)

	// Get data from search params (set by generateMetadata)
	const title = searchParams.get('title') || sanitizedDataId
	const summary = searchParams.get('summary') || ''
	const updatedAt = searchParams.get('updated') || ''

	// Build description with summary and updated date
	let description = summary
	if (updatedAt) {
		description = description
			? `${description}\n\nLast updated: ${updatedAt}`
			: `Last updated: ${updatedAt}`
	}

	return new ImageResponse(
		<OgBase
			type='data'
			path={`${sanitizedOrg} / ${sanitizedRepo}`}
			title={title}
			description={description || undefined}
			stats={[]}
		/>,
		{
			width: 1200,
			height: 630,
		},
	)
}
