'use client'

import { useEffect, useMemo, useRef, useState } from 'react'
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'

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
	provider: WebsocketProvider
	doc: Y.Doc
	fragment: Y.XmlFragment
	connected: boolean
}

/**
 * Hook that manages Yjs document and WebSocket provider lifecycle.
 *
 * y-websocket handles reconnection automatically with exponential
 * backoff. Offline edits are preserved in the local Y.Doc and
 * merged when the connection is restored.
 */
export function useCollaboration(
	config: CollaborationConfig | undefined,
): CollaborationState | null {
	const [connected, setConnected] = useState(false)
	const providerRef = useRef<WebsocketProvider | null>(null)
	const docRef = useRef<Y.Doc | null>(null)

	const doc = useMemo(() => {
		if (!config) return null
		return new Y.Doc()
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [config?.documentKey, config?.wsUrl])

	const fragment = useMemo(() => {
		if (!doc) return null
		return doc.getXmlFragment('document-store')
	}, [doc])

	useEffect(() => {
		if (!config || !doc) return

		// y-websocket connects to: ${serverUrl}/${roomname}?params
		const serverUrl = `${config.wsUrl}/ws/collab`

		const provider = new WebsocketProvider(serverUrl, config.documentKey, doc, {
			connect: true,
			disableBc: true,
			params: { operator_id: config.operatorId },
			maxBackoffTime: 5000,
		})

		provider.awareness.setLocalStateField('user', {
			name: config.userName,
			color: config.userColor,
		})

		const handleStatus = ({ status }: { status: string }) => {
			setConnected(status === 'connected')
		}
		provider.on('status', handleStatus)

		providerRef.current = provider
		docRef.current = doc

		return () => {
			provider.off('status', handleStatus)
			provider.destroy()
			doc.destroy()
			providerRef.current = null
			docRef.current = null
			setConnected(false)
		}
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [config?.documentKey, config?.wsUrl, doc])

	if (!config || !doc || !fragment) return null

	return providerRef.current
		? { provider: providerRef.current, doc, fragment, connected }
		: null
}
