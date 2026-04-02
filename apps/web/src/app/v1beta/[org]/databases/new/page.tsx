export const runtime = 'edge'

import { authWithCheck } from '@/app/(auth)/auth'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { handleNotFoundOrThrow } from '@/app/v1beta/_lib/platform-error-handler'
import { CreateDatabase } from './form'


export default async function NewDatabasePage() {
	await authWithCheck()
	const result = await platformAction(async sdk => sdk.newDatabase(), {
		onError: handleNotFoundOrThrow,
	})
	if (!result?.me) {
		return null
	}
	return <CreateDatabase organizations={result.me.organizations} />
}
