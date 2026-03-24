export const runtime = 'edge'

import { authWithCheck } from '../(auth)/auth'
import { platformAction } from '../v1beta/_lib/platform-action'
import { handleNotFound } from '../v1beta/_lib/platform-error-handler'
import { NewRepoForm } from './form'


export default async function NewRepo() {
	const session = await authWithCheck()
	const { me } = await platformAction(async sdk => sdk.newRepoPage(), {
		onError: handleNotFound,
	})
	return (
		<NewRepoForm
			userId={session.user.id ?? ''}
			organizations={me.organizations}
			session={session}
		/>
	)
}
