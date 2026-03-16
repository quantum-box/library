export const runtime = 'edge'

import { auth } from '@/app/(auth)/auth'
import {
	RepositoryUi,
	type RepositoryUiMetaUpdateInput,
} from '@/app/v1beta/[org]/[repo]/components/repository-ui'
import { RepositoryPageQuery, RepositoryPageWithTagsQuery } from '@/gen/graphql'
import { createSdkOperator, createSdkPlatform } from '@/lib/api-action'
import type { Metadata } from 'next'
import { revalidatePath } from 'next/cache'
import { notFound } from 'next/navigation'
import { getBaseUrl } from '../../_lib/get-base-url'
import {
	ErrorCode,
	PlatformActionError,
	platformAction,
} from '../../_lib/platform-action'
import { canEdit } from '../../_lib/repo-permissions'


type Props = {
	params: { org: string; repo: string }
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
	const { org, repo } = params

	try {
		const { repo: repoData } = await platformAction(
			async sdk => sdk.repoOgpMeta({ org, repo }),
			{
				onError: error => {
					console.warn(
						'OGP metadata generation failed for repository:',
						`${org}/${repo}`,
						error,
					)
				},
				allowAnonymous: true,
			},
		)

		const title = `${org}/${repo} | Library`
		const description =
			repoData.description || `${repoData.name} repository on Library`
		const dataCount = repoData.dataList?.paginator?.totalItems || 0
		const contributorCount = repoData.policies?.length || 0
		const tags = repoData.tags || []

		const ogImageUrl = new URL(`/api/${org}/${repo}/og`, getBaseUrl())
		ogImageUrl.searchParams.set('name', repoData.name)
		if (repoData.description) {
			ogImageUrl.searchParams.set('description', repoData.description)
		}
		ogImageUrl.searchParams.set('public', String(repoData.isPublic))
		ogImageUrl.searchParams.set('data', String(dataCount))
		ogImageUrl.searchParams.set('contributors', String(contributorCount))
		if (tags.length > 0) {
			ogImageUrl.searchParams.set('tags', tags.join(','))
		}

		return {
			title,
			description,
			openGraph: {
				title,
				description,
				type: 'website',
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
				description,
				images: [ogImageUrl.toString()],
			},
		}
	} catch {
		// Fallback: still generate OG image URL even without API data
		const ogImageUrl = new URL(`/api/${org}/${repo}/og`, getBaseUrl())
		ogImageUrl.searchParams.set('name', repo)

		return {
			title: `${org}/${repo} | Library`,
			openGraph: {
				title: `${org}/${repo} | Library`,
				type: 'website',
				images: [
					{
						url: ogImageUrl.toString(),
						width: 1200,
						height: 630,
						alt: `${org}/${repo} | Library`,
					},
				],
			},
			twitter: {
				card: 'summary_large_image',
				title: `${org}/${repo} | Library`,
				images: [ogImageUrl.toString()],
			},
		}
	}
}

async function updateRepositoryMetaAction(
	payload: RepositoryUiMetaUpdateInput,
) {
	'use server'
	const platformSdk = await createSdkPlatform()
	const { organization } = await platformSdk.GetOrgSettings({
		orgUsername: payload.org,
	})
	const operatorSdk = await createSdkOperator(organization.id)
	await operatorSdk.UpdateRepoSettings({
		input: {
			orgUsername: payload.org,
			repoUsername: payload.repo,
			name: payload.repoName,
			description: payload.about,
			isPublic: payload.isPublic,
			tags: payload.labels,
		},
	})

	revalidatePath(`/v1beta/${payload.org}/${payload.repo}`)
}

export default async function RepositoryPage({
	params: { org, repo },
	searchParams,
}: {
	params: { org: string; repo: string }
	searchParams: { page?: string; pageSize?: string }
}) {
	const session = await auth()
	const page = searchParams.page ? Number.parseInt(searchParams.page, 10) : 1
	const pageSize = searchParams.pageSize
		? Number.parseInt(searchParams.pageSize, 10)
		: 20

	let repoDataWithTags: RepositoryPageWithTagsQuery['repo'] | null = null
	let repoDataWithoutTags: RepositoryPageQuery['repo'] | null = null

	try {
		const resWithTags = await platformAction<RepositoryPageWithTagsQuery>(
			sdk =>
				sdk.repositoryPageWithTags({
					org,
					repo,
					page,
					pageSize,
				}),
			{
				onError: error => {
					if (error.code === ErrorCode.NOT_FOUND_ERROR) {
						notFound()
					}
					// Permission denied on a nested resolver (e.g. policies)
					// should not trigger notFound for public repos
					if (error.code === ErrorCode.PERMISSION_DENIED) {
						console.warn(
							'Permission denied during repo page load, continuing:',
							error.message,
						)
						return
					}
					throw error
				},
				redirectOnError: false,
				allowAnonymous: true,
			},
		)
		repoDataWithTags = resWithTags?.repo ?? null
	} catch (error: unknown) {
		if (
			error instanceof PlatformActionError &&
			error.message.includes('Unknown field "tags"')
		) {
			const resWithoutTags = await platformAction<RepositoryPageQuery>(
				sdk =>
					sdk.repositoryPage({
						org,
						repo,
						page,
						pageSize,
					}),
				{
					onError: fallbackError => {
						if (fallbackError.code === ErrorCode.NOT_FOUND_ERROR) {
							notFound()
						}
						if (fallbackError.code === ErrorCode.PERMISSION_DENIED) {
							console.warn(
								'Permission denied during repo page load, continuing:',
								fallbackError.message,
							)
							return
						}
						throw fallbackError
					},
					redirectOnError: false,
					allowAnonymous: true,
				},
			)
			repoDataWithoutTags = resWithoutTags?.repo ?? null
		} else {
			throw error
		}
	}

	const repoData = repoDataWithTags ?? repoDataWithoutTags

	if (!repoData) {
		throw new Error('Failed to load repository data')
	}

	const tags = repoDataWithTags?.tags ?? []

	const sources = repoData.sources?.map((source, index) => ({
		id: source.id,
		name: source.name,
		url: source.url ?? '',
		isPrimary: index === 0,
	}))
	const contributors = repoData.policies.map(policy => ({
		userId: policy.userId,
		role: policy.role,
		name: policy.user?.username ?? policy.user?.name ?? undefined,
		avatarUrl: policy.user?.image ?? undefined,
	}))

	// Check if user can edit (writer or owner)
	const canEditResult = await canEdit(org, repo)
	const hasEditPermission = canEditResult.isOk() && canEditResult.value

	// Check if GitHub sync is enabled (ext_github property exists)
	const hasGitHubSync = repoData.properties.some(p => p.name === 'ext_github')

	return (
		<RepositoryUi
			repoName={repoData.name}
			dataList={{
				items: repoData.dataList.items,
				paginator: repoData.dataList.paginator,
			}}
			properties={repoData.properties}
			about={repoData.description ?? ''}
			labels={tags}
			sources={sources}
			contributors={contributors}
			org={org}
			repo={repo}
			onMetaUpdate={hasEditPermission ? updateRepositoryMetaAction : undefined}
			isPublic={repoData.isPublic}
			hasGitHubSync={hasGitHubSync}
		/>
	)
}
