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
import { Suspense } from 'react'
import { getBaseUrl } from '../../_lib/get-base-url'
import {
	ErrorCode,
	PlatformActionError,
	platformAction,
} from '../../_lib/platform-action'
import { handleNotFoundOrPermissionDenied } from '../../_lib/platform-error-handler'
import { canEdit } from '../../_lib/repo-permissions'
import { RepoSkeleton } from './components/repo-skeleton'


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

async function fetchRepoData(
	org: string,
	repo: string,
	page: number,
	pageSize: number,
) {
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
				redirectOnError: false,
				allowAnonymous: true,
			},
		)
		repoDataWithTags = resWithTags?.repo ?? null
	} catch (error: unknown) {
		// NOT_FOUND should render the 404 page
		if (
			error instanceof PlatformActionError &&
			error.code === ErrorCode.NOT_FOUND_ERROR
		) {
			notFound()
		}

		// For any other error (schema mismatch like "Unknown field tags",
		// server errors, etc.), fall back to query without tags
		const resWithoutTags = await platformAction<RepositoryPageQuery>(
			sdk =>
				sdk.repositoryPage({
					org,
					repo,
					page,
					pageSize,
				}),
			{
				onError: handleNotFoundOrPermissionDenied,
				redirectOnError: false,
				allowAnonymous: true,
			},
		)
		repoDataWithoutTags = resWithoutTags?.repo ?? null
	}

	return { repoDataWithTags, repoDataWithoutTags }
}

async function RepoContent({
	org,
	repo,
	page,
	pageSize,
}: {
	org: string
	repo: string
	page: number
	pageSize: number
}) {
	const [sessionResult, repoDataResult, canEditResult] =
		await Promise.allSettled([
			auth(),
			fetchRepoData(org, repo, page, pageSize),
			canEdit(org, repo),
		])

	// Re-throw rejected fetchRepoData so notFound() propagates correctly
	if (repoDataResult.status === 'rejected') {
		throw repoDataResult.reason
	}

	const repoDataSettled = repoDataResult.value
	const repoData =
		repoDataSettled?.repoDataWithTags ?? repoDataSettled?.repoDataWithoutTags

	if (!repoData) {
		throw new Error('Failed to load repository data')
	}

	const tags = repoDataSettled?.repoDataWithTags?.tags ?? []

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

	const hasEditPermission =
		canEditResult.status === 'fulfilled' &&
		canEditResult.value.isOk() &&
		canEditResult.value.value

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

export default async function RepositoryPage({
	params: { org, repo },
	searchParams,
}: {
	params: { org: string; repo: string }
	searchParams: { page?: string; pageSize?: string }
}) {
	const page = searchParams.page ? Number.parseInt(searchParams.page, 10) : 1
	const pageSize = searchParams.pageSize
		? Number.parseInt(searchParams.pageSize, 10)
		: 20

	return (
		<Suspense fallback={<RepoSkeleton />}>
			<RepoContent org={org} repo={repo} page={page} pageSize={pageSize} />
		</Suspense>
	)
}
