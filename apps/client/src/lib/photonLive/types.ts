import type { Awareness } from 'y-protocols/awareness'
import type * as Y from 'yjs'

export type PhotonLiveFormat = 'markdown' | 'richText'

export interface PhotonLiveRecordTarget {
  org: string
  repo: string
  dataId: string
  propertyId: string
  operatorId?: string
}

export interface PhotonLiveSession {
  ticket: string
  roomId: string
  actorId: string
  format: PhotonLiveFormat
  body: string
  recordVersion: string
}

export type PhotonLiveConnectionStatus =
  | 'authorizing'
  | 'connecting'
  | 'initializing'
  | 'connected'
  | 'disconnected'
  | 'failed'

export type PhotonLiveErrorKind =
  | 'unauthorized'
  | 'disabled'
  | 'timeout'
  | 'transport'
  | 'protocol'
  | 'conflict'
  | 'server'

export class PhotonLiveError extends Error {
  readonly kind: PhotonLiveErrorKind
  readonly status?: number
  readonly retryable: boolean

  constructor(
    message: string,
    kind: PhotonLiveErrorKind,
    status?: number,
    retryable = false,
  ) {
    super(message)
    this.name = 'PhotonLiveError'
    this.kind = kind
    this.status = status
    this.retryable = retryable
  }
}

export type PhotonLiveSaveStatus = 'idle' | 'saving' | 'saved' | 'conflict' | 'error'

export interface PhotonLiveState {
  status: PhotonLiveConnectionStatus
  saveStatus: PhotonLiveSaveStatus
  error: PhotonLiveError | null
  initialized: boolean
  version: number
  recordVersion: string
  hasUnackedChanges: boolean
  canEdit: boolean
}

export interface PhotonLiveProvider {
  readonly doc: Y.Doc
  readonly fragment: Y.XmlFragment
  readonly awareness: Awareness
  readonly user: { name: string; color: string }
  readonly session: PhotonLiveSession | null
  getState(): PhotonLiveState
  subscribe(listener: (state: PhotonLiveState) => void): () => void
  subscribeSave(listener: (state: PhotonLiveState) => void): () => void
  queueCheckpoint(body: string): void
  flushCheckpoint(): void
  destroy(): void
}
