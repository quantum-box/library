export const runtime = 'edge'

import { DataDetailUi } from '@/app/v1beta/_components/data-detail-ui'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { requireEditPermission } from '@/app/v1beta/_lib/repo-permissions'
import {
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import { notFound } from 'next/navigation'
import { createData } from './action'


export default async function NewDataPage({
	params: { org, repo },
}: {
	params: { org: string; repo: string }
}) {
	// Require edit permission (writer or owner)
	await requireEditPermission(org, repo)

	const { properties, dataList } = await platformAction(
		async sdk =>
			sdk.newData({
				orgUsername: org,
				repoUsername: repo,
			}),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
			},
		},
	)

	return (
		<>
			<DataDetailUi
				properties={properties}
				data={{
					id: '',
					name: '',
					propertyData: createEmptyPropertyData(properties),
				}}
				dataList={dataList}
				onSave={createData}
				onlyEdit
			/>
		</>
	)
}

const createEmptyPropertyData = (
	properties: PropertyForEditorFragment[],
): PropertyDataForEditorFragment[] => {
	return properties.map(x => {
		switch (x.typ) {
			case PropertyType.Html:
				return {
					propertyId: x.id,
					value: {
						__typename: 'HtmlValue' as const,
						html: '',
					},
				}
			case PropertyType.Markdown:
				return {
					propertyId: x.id,
					value: {
						__typename: 'MarkdownValue' as const,
						markdown: '',
					},
				}
			case PropertyType.String:
				return {
					propertyId: x.id,
					value: {
						__typename: 'StringValue' as const,
						string: '',
					},
				}
			case PropertyType.Integer:
				return {
					propertyId: x.id,
					value: {
						__typename: 'IntegerValue' as const,
						number: '',
					},
				}
			case PropertyType.MultiSelect:
				return {
					propertyId: x.id,
					value: {
						__typename: 'MultiSelectValue' as const,
						optionIds: [],
					},
				}
			case PropertyType.Select:
				return {
					propertyId: x.id,
					value: {
						__typename: 'SelectValue' as const,
						optionId: '',
					},
				}
			case PropertyType.Relation:
				return {
					propertyId: x.id,
					value: {
						__typename: 'RelationValue' as const,
						databaseId: '',
						dataIds: [],
					},
				}
			case PropertyType.Location:
				return {
					propertyId: x.id,
					value: {
						__typename: 'LocationValue' as const,
						latitude: 0,
						longitude: 0,
					},
				}
			default:
				throw new Error(`Unsupported property type: ${x.typ}`)
		}
	})
}
