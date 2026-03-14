'use server'

import { auth } from '@/app/(auth)/auth'
import { convertPropertyData } from '@/app/v1beta/_lib/property-data-converter'
import {
	DataForDataDetailFragment,
	PropertyForEditorFragment,
} from '@/gen/graphql'
import { createSdkPlatform } from '@/lib/api-action'
import { revalidatePath } from 'next/cache'

export const updateData = async ({
	org,
	repo,
	dataId,
	properties,
	input,
}: {
	org: string
	repo: string
	dataId: string
	properties: PropertyForEditorFragment[]
	input: DataForDataDetailFragment
}) => {
	'use server'
	const session = await auth()
	const sdk = await createSdkPlatform()

	if (!session) {
		throw new Error('Unauthorized')
	}
	await sdk.updateData({
		input: {
			actor: session.user.id ?? '',
			orgUsername: org,
			repoUsername: repo,
			dataId,
			dataName: input.name,
			propertyData: convertPropertyData(properties, input.propertyData),
		},
	})
	revalidatePath(`/v1beta/${org}/${repo}`)
	return dataId
}
