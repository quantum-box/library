import { PropertiesUi } from '@/app/v1beta/_components/properties-ui'
import type { GitHubRepoConfig } from '@/app/v1beta/_components/properties-ui/github-repos-editor'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { canEdit } from '@/app/v1beta/_lib/repo-permissions'
import {
	MultiSelectTypeMetaForPropertiesUiFragment,
	PropertyForPropertiesUiFragment,
	PropertyMetaInput,
	PropertyType,
	RelationTypeMetaForPropertiesUiFragment,
	SelectTypeMetaForPropertiesUiFragment,
} from '@/gen/graphql'
import { createSdkPlatform } from '@/lib/api-action'
import { notFound } from 'next/navigation'
import { getGitHubConnection } from '../../_components/github-settings-actions'

export default async function PropertiesPage({
	params: { org, repo },
}: { params: { org: string; repo: string } }) {
	const [{ properties }, gitHubConnection, dataCountResult] = await Promise.all(
		[
			platformAction(
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
						if (error.code === ErrorCode.PERMISSION_DENIED) {
							console.warn(
								'Permission denied during properties page load, continuing:',
								error.message,
							)
							return
						}
					},
					redirectOnError: false,
					allowAnonymous: true,
				},
			),
			getGitHubConnection().catch(() => null),
			platformAction(
				async sdk =>
					sdk.DataCountForBulkSync({
						org,
						repo,
					}),
				{
					redirectOnError: false,
					allowAnonymous: true,
				},
			).catch(() => ({ repo: { dataList: { paginator: { totalItems: 0 } } } })),
		],
	)

	// Check if user can edit (writer or owner)
	const canEditResult = await canEdit(org, repo)
	const hasEditPermission = canEditResult.isOk() && canEditResult.value

	const totalDataCount = dataCountResult.repo.dataList.paginator.totalItems
	const onAddProperty = async (property: PropertyForPropertiesUiFragment) => {
		'use server'
		const sdk = await createSdkPlatform()
		await sdk.addProperty({
			input: {
				orgUsername: org,
				repoUsername: repo,
				propertyName: property.name,
				propertyType: property.typ,
				meta: getPropertyMeta(property),
			},
		})
	}

	const onUpdateProperty = async (
		property: PropertyForPropertiesUiFragment,
	) => {
		'use server'
		const sdk = await createSdkPlatform()
		await sdk.updateProperty({
			id: property.id,
			input: {
				orgUsername: org,
				repoUsername: repo,
				propertyName: property.name,
				propertyType: property.typ,
				meta: getPropertyMeta(property),
			},
		})
	}

	const onRemoveProperty = async (propertyId: string) => {
		'use server'
		const sdk = await createSdkPlatform()
		await sdk.deleteProperty({
			orgUsername: org,
			repoUsername: repo,
			id: propertyId,
		})
	}

	const onBulkSyncGitHub = async (
		repoConfigs: GitHubRepoConfig[],
		extGithubPropertyId: string,
	) => {
		'use server'
		const sdk = await createSdkPlatform()

		await sdk.BulkSyncExtGithub({
			input: {
				orgUsername: org,
				repoUsername: repo,
				extGithubPropertyId,
				repoConfigs: repoConfigs.map(c => ({
					repo: c.repo,
					label: c.label,
					defaultPath: c.defaultPath,
				})),
			},
		})
	}

	return (
		<PropertiesUi
			properties={properties}
			onAddProperty={hasEditPermission ? onAddProperty : undefined}
			onUpdateProperty={hasEditPermission ? onUpdateProperty : undefined}
			onRemoveProperty={hasEditPermission ? onRemoveProperty : undefined}
			isGitHubConnected={gitHubConnection?.connected ?? false}
			onBulkSyncGitHub={hasEditPermission ? onBulkSyncGitHub : undefined}
			totalDataCount={totalDataCount}
			settingsUrl={`/v1beta/${org}/${repo}/settings`}
		/>
	)
}

const getPropertyMeta = (
	property: PropertyForPropertiesUiFragment,
): PropertyMetaInput | null => {
	// ext_github property stores JSON metadata
	if (property.name === 'ext_github') {
		const jsonMeta = (property.meta as { json?: string } | null)?.json
		if (jsonMeta) {
			return { json: jsonMeta }
		}
		return null
	}

	switch (property.typ) {
		case PropertyType.String:
			return null
		case PropertyType.Integer:
			return null
		case PropertyType.Html:
			return null
		case PropertyType.Markdown:
			return null
		case PropertyType.Select:
			return {
				select: (
					property.meta as SelectTypeMetaForPropertiesUiFragment
				).options.map(option => ({
					identifier: option.key,
					label: option.name,
				})),
			}
		case PropertyType.MultiSelect:
			return {
				multiSelect: (
					property.meta as MultiSelectTypeMetaForPropertiesUiFragment
				).options.map(option => ({
					identifier: option.key,
					label: option.name,
				})),
			}
		case PropertyType.Relation:
			return {
				relation: (property.meta as RelationTypeMetaForPropertiesUiFragment)
					.databaseId,
			}
		default:
			return null
	}
}
