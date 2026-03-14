'use server'
import { auth } from '@/app/(auth)/auth'
import { getSdkOperator, getSdkPlatform } from './apiClient'

export const createSdkPlatform = async () => {
	'use server'
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	return getSdkPlatform(session.user.accessToken)
}

export const createSdkOperator = async (operatorId: string) => {
	'use server'
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	return getSdkOperator(session.user.accessToken, operatorId)
}
