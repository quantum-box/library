import { notFound } from 'next/navigation'
import { authWithCheck } from '../(auth)/auth'
import { ErrorCode, platformAction } from '../v1beta/_lib/platform-action'
import { NewRepoForm } from './form'


export default async function NewRepo() {
	const session = await authWithCheck()
	const { me } = await platformAction(async sdk => sdk.newRepoPage(), {
		onError: error => {
			if (error.code === ErrorCode.NOT_FOUND_ERROR) {
				notFound()
			}
		},
	})
	return (
		<NewRepoForm
			userId={session.user.id ?? ''}
			organizations={me.organizations}
			session={session}
		/>
	)
}
