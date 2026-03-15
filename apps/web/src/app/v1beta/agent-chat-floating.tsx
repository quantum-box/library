'use client'

type AgentChatFloatingProps = {
	children: React.ReactNode
	accessToken: string
	userId?: string
}

export function AgentChatFloating({ children }: AgentChatFloatingProps) {
	return <>{children}</>
}
