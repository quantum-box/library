import { DataDetailUi } from '@/app/v1beta/_components/data-detail-ui'
import { getBaseUrl } from '@/app/v1beta/_lib/get-base-url'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { canEdit } from '@/app/v1beta/_lib/repo-permissions'
import type { Metadata } from 'next'
import { notFound } from 'next/navigation'
import { updateData } from './action'

export const runtime = 'edge'

type Props = {
	params: { org: string; repo: string; dataId: string }
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
	const { org, repo, dataId } = params

	try {
		const { data } = await platformAction(
			async sdk =>
				sdk.dataOgpMeta({
					orgUsername: org,
					repoUsername: repo,
					dataId,
				}),
			{
				onError: error => {
					console.warn(
						'OGP metadata generation failed for data:',
						`${org}/${repo}/${dataId}`,
						error,
					)
				},
				allowAnonymous: true,
			},
		)

		const title = `${data.name} | ${org}/${repo} | Library`
		const description = `${data.name} in ${org}/${repo} on Library`

		// Get summary from first string property value
		let summary = ''
		if (data.propertyData) {
			for (const prop of data.propertyData) {
				if (prop.value && 'string' in prop.value && prop.value.string) {
					summary = prop.value.string.slice(0, 100)
					break
				}
			}
		}

		// Format updated date
		const updatedAt = data.updatedAt
			? new Date(data.updatedAt).toLocaleDateString('en-US', {
					year: 'numeric',
					month: 'short',
					day: 'numeric',
				})
			: ''

		const ogImageUrl = new URL(`/api/${org}/${repo}/${dataId}/og`, getBaseUrl())
		ogImageUrl.searchParams.set('title', data.name)
		if (summary) {
			ogImageUrl.searchParams.set('summary', summary)
		}
		if (updatedAt) {
			ogImageUrl.searchParams.set('updated', updatedAt)
		}

		return {
			title,
			description: summary || description,
			openGraph: {
				title,
				description: summary || description,
				type: 'article',
				images: [
					{
						url: ogImageUrl.toString(),
						width: 1200,
						height: 630,
						alt: title,
					},
				],
			},
			twitter: {
				card: 'summary_large_image',
				title,
				description: summary || description,
				images: [ogImageUrl.toString()],
			},
		}
	} catch {
		// Fallback: still generate OG image URL even without API data
		const ogImageUrl = new URL(`/api/${org}/${repo}/${dataId}/og`, getBaseUrl())
		ogImageUrl.searchParams.set('title', dataId)

		return {
			title: `${dataId} | ${org}/${repo} | Library`,
			openGraph: {
				title: `${dataId} | ${org}/${repo} | Library`,
				type: 'article',
				images: [
					{
						url: ogImageUrl.toString(),
						width: 1200,
						height: 630,
						alt: `${dataId} | ${org}/${repo} | Library`,
					},
				],
			},
			twitter: {
				card: 'summary_large_image',
				title: `${dataId} | ${org}/${repo} | Library`,
				images: [ogImageUrl.toString()],
			},
		}
	}
}

export default async function DataPage({
	params: { org, repo, dataId },
}: {
	params: { org: string; repo: string; dataId: string }
}) {
	const { data, properties, dataList } = await platformAction(
		async sdk =>
			sdk.dataDetailPage({
				orgUsername: org,
				repoUsername: repo,
				dataId,
			}),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
				if (error.code === ErrorCode.PERMISSION_DENIED) {
					console.warn(
						'Permission denied during data detail page load, continuing:',
						error.message,
					)
					return
				}
			},
			redirectOnError: false,
			allowAnonymous: true,
		},
	)

	// Check if user can edit (writer or owner)
	const canEditResult = await canEdit(org, repo)
	const hasEditPermission = canEditResult.isOk() && canEditResult.value

	// Collaboration WebSocket URL (set via env var in deployment)
	const collaborationWsUrl =
		process.env.NEXT_PUBLIC_LIBRARY_COLLAB_WS_URL ?? undefined

	return (
		<>
			<DataDetailUi
				data={data}
				properties={properties}
				dataList={dataList}
				onSave={hasEditPermission ? updateData : undefined}
				viewOnly={!hasEditPermission}
				collaborationWsUrl={collaborationWsUrl}
				collaborationOperatorId={org}
				collaborationUserName='User'
			/>
		</>
	)
}
