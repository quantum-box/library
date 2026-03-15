import { authWithCheck } from '@/app/(auth)/auth'
import { NewOrgForm } from './component'

export const runtime = 'edge'

export default async function Page() {
	const session = await authWithCheck()
	return <NewOrgForm session={session} />
}
