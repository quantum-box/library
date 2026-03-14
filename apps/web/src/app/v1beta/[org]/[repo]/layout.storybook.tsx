'use client'

import { NAV_ITEMS } from '@/app/v1beta/_components/constant'
import { Navigation } from '@/app/v1beta/_components/navigation'

/**
 * Storybook用のV1BetaLayoutラッパーコンポーネント
 * paramsをpropsとして受け取り、isOwnerのチェックをスキップしてNavigationを表示する
 */
export default function V1BetaLayoutForStorybook({
	children,
	params,
}: {
	children: React.ReactNode
	params: { org: string; repo: string }
}) {
	// Storybookでは常にownerとして扱い、settingsリンクを表示する
	return (
		<>
			<Navigation items={NAV_ITEMS} />
			{children}
		</>
	)
}
