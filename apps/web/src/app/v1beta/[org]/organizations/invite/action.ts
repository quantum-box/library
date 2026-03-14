'use server'

import { auth } from '@/app/(auth)/auth'
import { IdOrEmail } from '@/gen/graphql'
import { getSdkOperator, getSdkPlatform, platformId } from '@/lib/apiClient'

export async function inviteUser({
	org,
	invitee,
	sendNotification,
}: {
	org: string
	invitee: IdOrEmail
	sendNotification: boolean
}) {
	'use server'
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const { organization } = await getSdkPlatform(
		session.user.accessToken,
	).orgInvitePage({ orgUsername: org })
	await getSdkOperator(session.user.accessToken, organization.id).inviteUser({
		platformId,
		tenantId: organization.id,
		invitee,
		notifyUser: sendNotification,
	})
}
