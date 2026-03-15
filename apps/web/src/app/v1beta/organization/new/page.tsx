import { authWithCheck } from '../../../(auth)/auth'
import { NewOrgForm } from './form'

export const runtime = 'edge'

export default async function NewOrg() {
	const session = await authWithCheck()
	return <NewOrgForm userId={session.user.id ?? ''} session={session} />
}
