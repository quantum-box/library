'use server'

import { auth } from '@/app/(auth)/auth'
import { convertPropertyData } from '@/app/v1beta/_lib/property-data-converter'
import {
	DataForDataDetailFragment,
	PropertyForEditorFragment,
} from '@/gen/graphql'
import { createSdkPlatform } from '@/lib/api-action'
import { revalidatePath } from 'next/cache'

export const createData = async ({
	org,
	repo,
	properties,
	input,
}: {
	org: string
	repo: string
	properties: PropertyForEditorFragment[]
	input: DataForDataDetailFragment
}) => {
	'use server'
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = await createSdkPlatform()
	const res = await sdk.addData({
		input: {
			actor: session.user.id ?? '',
			orgUsername: org,
			repoUsername: repo,
			dataName: input.name,
			propertyData: convertPropertyData(properties, input.propertyData),
		},
	})
	revalidatePath(`/v1beta/${org}/${repo}`)
	revalidatePath(`/v1beta/${org}/${repo}/data/${res.addData.id}`)
	return res.addData.id
}
