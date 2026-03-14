import { auth } from '@/app/(auth)/auth'
import { baseURL, platformId } from '@/lib/apiClient'
import { NextResponse, type NextRequest } from 'next/server'

type Params = {
	params: { org: string; repo: string; dataId: string }
}

export async function GET(_req: NextRequest, { params }: Params) {
	const { org, repo, dataId } = params

	const session = await auth()
	const accessToken = session?.user.accessToken

	try {
		const res = await fetch(
			`${baseURL}/v1beta/repos/${org}/${repo}/data/${dataId}/md`,
			{
				headers: {
					'x-platform-id': platformId,
					...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
				},
			},
		)

		if (!res.ok) {
			return NextResponse.json(
				{ message: res.statusText },
				{ status: res.status },
			)
		}

		const markdown = await res.text()
		return new NextResponse(markdown, {
			headers: {
				'Content-Type': 'text/markdown; charset=utf-8',
			},
		})
	} catch (error) {
		console.error('markdown route error', error)
		return NextResponse.json(
			{ message: 'Internal Server Error' },
			{ status: 500 },
		)
	}
}
