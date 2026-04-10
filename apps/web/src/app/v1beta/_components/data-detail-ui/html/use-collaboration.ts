
import { useEffect, useRef, useState } from 'react'

export type CollaborationConfig = {
	/** WebSocket server base URL (e.g. ws://localhost:50053) */
	wsUrl: string
	/** Unique document key for this property */
	documentKey: string
	/** Operator ID for tenant isolation */
	operatorId: string
	/** Current user's display name */
	userName: string
	/** Current user's cursor colour */
	userColor: string
}

export type CollaborationState = {
	provider: import('y-websocket').WebsocketProvider
	doc: import('yjs').Doc
	fragment: import('yjs').XmlFragment
	connected: boolean
}

/**
 * Hook that manages Yjs document and WebSocket provider lifecycle.
 *
 * yjs and y-websocket are loaded dynamically to avoid bundling ~50KB
 * for pages that don't use collaboration.
 *
 * y-websocket handles reconnection automatically with exponential
 * backoff. Offline edits are preserved in the local Y.Doc and
 * merged when the connection is restored.
 */
export function useCollaboration(
	config: CollaborationConfig | undefined,
): CollaborationState | null {
	const [connected, setConnected] = useState(false)
	const [state, setState] = useState<{
		doc: import('yjs').Doc
		fragment: import('yjs').XmlFragment
		provider: import('y-websocket').WebsocketProvider
	} | null>(null)
	const stateRef = useRef(state)
	stateRef.current = state

	useEffect(() => {
		if (!config) {
			setState(null)
			return
		}

		let cancelled = false

		const setup = async () => {
			const [Y, { WebsocketProvider }] = await Promise.all([
				import('yjs'),
				import('y-websocket'),
			])

			if (cancelled) return

			const doc = new Y.Doc()
			const fragment = doc.getXmlFragment('document-store')

			const serverUrl = `${config.wsUrl}/ws/collab`
			const provider = new WebsocketProvider(
				serverUrl,
				config.documentKey,
				doc,
				{
					connect: true,
					disableBc: true,
					params: { operator_id: config.operatorId },
					maxBackoffTime: 5000,
				},
			)

			provider.awareness.setLocalStateField('user', {
				name: config.userName,
				color: config.userColor,
			})

			const handleStatus = ({ status }: { status: string }) => {
				setConnected(status === 'connected')
			}
			provider.on('status', handleStatus)

			if (cancelled) {
				provider.off('status', handleStatus)
				provider.destroy()
				doc.destroy()
				return
			}

			const next = { doc, fragment, provider }
			stateRef.current = next
			setState(next)
		}

		void setup()

		return () => {
			cancelled = true
			const s = stateRef.current
			if (s) {
				s.provider.destroy()
				s.doc.destroy()
			}
			setState(null)
			setConnected(false)
		}
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [config?.documentKey, config?.wsUrl])

	if (!config || !state) return null

	return { ...state, connected }
}
