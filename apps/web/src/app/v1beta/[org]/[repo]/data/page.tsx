import { auth } from '@/app/(auth)/auth'
import {
	ErrorCode,
	PlatformActionError,
	platformAction,
} from '@/app/v1beta/_lib/platform-action'
import { RepositoryPageQuery, RepositoryPageWithTagsQuery } from '@/gen/graphql'
import { notFound } from 'next/navigation'
import { DataViewComponent } from './components/data-view'


export default async function DataPage({
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
		: 50 // Increased default for better UX

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
					if (error.code === ErrorCode.PERMISSION_DENIED) {
						console.warn(
							'Permission denied during data page load, continuing:',
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
		repoDataWithTags = resWithTags.repo
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
								'Permission denied during data page load, continuing:',
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
			repoDataWithoutTags = resWithoutTags.repo
		} else {
			throw error
		}
	}

	const repoData = repoDataWithTags ?? repoDataWithoutTags

	if (!repoData) {
		throw new Error('Failed to load repository data')
	}

	return (
		<DataViewComponent
			org={org}
			repo={repo}
			dataList={{
				items: repoData.dataList.items,
				paginator: repoData.dataList.paginator,
			}}
			properties={repoData.properties}
			canEdit={Boolean(session)}
		/>
	)
}
