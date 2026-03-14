'use client'

import dynamic from 'next/dynamic'
import { getLlmsBaseUrl } from '@/lib/getLlmsBaseUrl'
import { usePathname } from 'next/navigation'
import { useMemo } from 'react'

type AgentChatFloatingProps = {
	children: React.ReactNode
	accessToken: string
	userId?: string
}

const TENANT_ID =
	process.env.NEXT_PUBLIC_PLATFORM_ID || 'tn_01j702qf86pc2j35s0kv0gv3gy'

function deriveContext(pathname: string): { page: string; hint?: string } {
	const segments = pathname.split('/').filter(Boolean)
	const pagePath = segments.slice(1).join('/') || 'home'
	const contextMap: Record<string, { page: string; hint?: string }> = {
		books: { page: 'books', hint: 'Book catalog' },
		shelves: { page: 'shelves', hint: 'Shelf management' },
		settings: { page: 'settings', hint: 'Library settings' },
	}
	return contextMap[pagePath] ?? { page: pagePath }
}

const LazyFloatingChatPanel = dynamic(
	() =>
		import('@tachyon-apps/agent-chat').then(mod => ({
			default: mod.FloatingChatPanel,
		})),
	{ ssr: false },
)

const LazyAgentChatProvider = dynamic(
	() =>
		import('@tachyon-apps/agent-chat').then(mod => ({
			default: mod.AgentChatProvider,
		})),
	{ ssr: false },
)

export function AgentChatFloating({
	children,
	accessToken,
	userId,
}: AgentChatFloatingProps) {
	const pathname = usePathname()
	const context = useMemo(() => deriveContext(pathname), [pathname])
	const apiBaseUrl = useMemo(() => getLlmsBaseUrl(), [])

	return (
		<LazyAgentChatProvider
			apiBaseUrl={apiBaseUrl}
			accessToken={accessToken}
			tenantId={TENANT_ID}
			userId={userId}
			context={context}
		>
			{children}
			<LazyFloatingChatPanel />
		</LazyAgentChatProvider>
	)
}
