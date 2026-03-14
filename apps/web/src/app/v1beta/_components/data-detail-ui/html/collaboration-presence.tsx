'use client'

import { useEffect, useState } from 'react'
import type { CollaborationState } from './use-collaboration'

type UserPresence = {
	clientId: number
	name: string
	color: string
}

/**
 * Shows avatars of users currently connected to the document.
 * Listens to Yjs Awareness state changes.
 */
export function CollaborationPresence({
	collaboration,
}: {
	collaboration: CollaborationState
}) {
	const [users, setUsers] = useState<UserPresence[]>([])

	useEffect(() => {
		const awareness = collaboration.provider.awareness

		const updateUsers = () => {
			const states = awareness.getStates()
			const localClientId = awareness.clientID
			const result: UserPresence[] = []
			states.forEach((state, clientId) => {
				if (clientId === localClientId) return
				if (!state.user) return
				result.push({
					clientId,
					name: state.user.name ?? 'Anonymous',
					color: state.user.color ?? '#6b7280',
				})
			})
			setUsers(result)
		}

		awareness.on('change', updateUsers)
		updateUsers()

		return () => {
			awareness.off('change', updateUsers)
		}
	}, [collaboration.provider])

	if (users.length === 0) return null

	return (
		<div className='flex items-center gap-1'>
			{users.map(user => (
				<div
					key={user.clientId}
					className='flex h-6 w-6 items-center justify-center rounded-full text-[10px] font-medium text-white'
					style={{ backgroundColor: user.color }}
					title={user.name}
				>
					{user.name.charAt(0).toUpperCase()}
				</div>
			))}
			<span className='ml-1 text-xs text-muted-foreground'>
				{users.length === 1
					? '1 other editor'
					: `${users.length} other editors`}
			</span>
		</div>
	)
}
