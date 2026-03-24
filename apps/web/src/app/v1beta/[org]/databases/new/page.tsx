export const runtime = 'edge'

import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { handleNotFoundOrThrow } from '@/app/v1beta/_lib/platform-error-handler'
import { CreateDatabase } from './form'


export default async function NewDatabasePage() {
	const { me } = await platformAction(async sdk => sdk.newDatabase(), {
		onError: handleNotFoundOrThrow,
	})
	return <CreateDatabase organizations={me.organizations} />
}
