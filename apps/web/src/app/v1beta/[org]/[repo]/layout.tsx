import { NAV_ITEMS } from '@/app/v1beta/_components/constant'
import { Navigation } from '@/app/v1beta/_components/navigation'
import { isOwner } from '@/app/v1beta/_lib/repo-permissions'

export const runtime = 'edge'

export default async function V1BetaLayout({
	children,
	params,
}: {
	children: React.ReactNode
	params: { org: string; repo: string }
}) {
	// Check if current user is owner to show settings link
	const isOwnerResult = await isOwner(params.org, params.repo)
	const isCurrentUserOwner = isOwnerResult.isOk() && isOwnerResult.value

	const navItems = isCurrentUserOwner
		? NAV_ITEMS
		: NAV_ITEMS.filter(item => item.value !== 'settings')

	return (
		<>
			<Navigation items={navItems} />
			{children}
		</>
	)
}
