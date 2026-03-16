export const runtime = 'edge'

import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { notFound } from 'next/navigation'
import { CreateDatabase } from './form'


export default async function NewDatabasePage() {
	const { me } = await platformAction(async sdk => sdk.newDatabase(), {
		onError: error => {
			if (error.code === ErrorCode.NOT_FOUND_ERROR) {
				notFound()
			}
			throw error
		},
	})
	return <CreateDatabase organizations={me.organizations} />
}
