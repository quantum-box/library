export const runtime = 'edge'

import { auth } from '@/app/(auth)/auth'
import { createApiKeyAction } from '@/app/v1beta/[org]/_components/action'
import { ApiKeyDialog } from '@/app/v1beta/[org]/_components/api-key-dialog'
import { ApiKeyListServer } from '@/app/v1beta/[org]/_components/api-key-list-server'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { notFound } from 'next/navigation'
import { ApiPageUi } from './_components/api-page-ui'


export default async function ApiPage({
	params: { org, repo },
}: { params: { org: string; repo: string } }) {
	// Verify repository exists
	await platformAction(
		async sdk =>
			sdk.properties({
				orgUsername: org,
				repoUsername: repo,
			}),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
			},
			redirectOnError: false,
			allowAnonymous: true,
		},
	)

	const apiBaseUrl =
		process.env.NEXT_PUBLIC_BACKEND_API_URL || 'http://localhost:50053'

	const session = await auth()

	const apiKeySlot = session ? (
		<ApiKeyDialog orgUsername={org} onCreate={createApiKeyAction} />
	) : null

	const apiKeyListSlot = session ? <ApiKeyListServer orgUsername={org} /> : null

	return (
		<ApiPageUi
			org={org}
			repo={repo}
			apiBaseUrl={apiBaseUrl}
			apiKeySlot={apiKeySlot}
			apiKeyListSlot={apiKeyListSlot}
		/>
	)
}
