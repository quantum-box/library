import { useEffect, useMemo, useRef, useState } from 'react'
import { appKitConfig } from '../../app/kitConfig'
import {
  createPhotonLiveProvider,
  type PhotonLiveProviderOptions,
} from './client'
import {
  type PhotonLiveError,
  type PhotonLiveProvider,
  type PhotonLiveState,
} from './types'

export type UsePhotonLiveRecordOptions = Omit<PhotonLiveProviderOptions, 'config'>

export interface PhotonLiveRecordState {
  provider: PhotonLiveProvider | null
  state: PhotonLiveState | null
  /** True after the first server snapshot and ready handshake. */
  mounted: boolean
  /** An initial failure that should use the normal editor as a fallback. */
  initialError: PhotonLiveError | null
}

/**
 * Creates one room-scoped Live provider for the lifetime of a record body.
 * The provider keeps its Y.Doc when a socket reconnects so an unsent edit is
 * still available while the editor is temporarily read-only.
 */
export function usePhotonLiveRecord(
  options: UsePhotonLiveRecordOptions | null,
): PhotonLiveRecordState {
  const [provider, setProvider] = useState<PhotonLiveProvider | null>(null)
  const [state, setState] = useState<PhotonLiveState | null>(null)
  const [mounted, setMounted] = useState(false)
  const providerKeyRef = useRef<string | null>(null)

  const key = useMemo(() => {
    if (!options) return null
    return [
      options.target.org,
      options.target.repo,
      options.target.dataId,
      options.target.propertyId,
      options.target.operatorId ?? '',
      options.format,
    ].join('\u0000')
  }, [options])

  // Callers commonly construct the record target inline. Keep the provider
  // lifetime tied to the scalar room scope above, rather than to that object
  // identity, so a parent save/re-render cannot drop the Y.Doc under the
  // user's caret.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const stableOptions = useMemo(() => options, [key])

  useEffect(() => {
    if (!stableOptions || !appKitConfig.dataLive.baseUrl || !key) {
      return
    }

    let disposed = false
    providerKeyRef.current = null
    const nextProvider = createPhotonLiveProvider({
      ...stableOptions,
      config: appKitConfig.dataLive,
    })
    // State updates are delivered from the provider subscription. Deferring
    // the provider identity itself avoids a synchronous effect update while
    // still preventing a parent render from observing the previous room.
    queueMicrotask(() => {
      if (!disposed) {
        providerKeyRef.current = key
        setProvider(nextProvider)
      }
    })
    const unsubscribe = nextProvider.subscribe((nextState) => {
      if (disposed) return
      setState(nextState)
      // A new record/socket must wait for its own handshake. Keeping the
      // previous room's mounted=true flag here would mount BlockNote against
      // an uninitialized Y.Doc and seed the wrong record.
      setMounted(nextState.initialized)
    })

    return () => {
      disposed = true
      providerKeyRef.current = null
      unsubscribe()
      nextProvider.destroy()
    }
  }, [key, stableOptions])

  const active = Boolean(stableOptions && appKitConfig.dataLive.baseUrl && key)
  const roomActive = active && providerKeyRef.current === key
  const initialError = roomActive && state?.status === 'failed' ? state.error : null
  return {
    provider: roomActive ? provider : null,
    state: roomActive ? state : null,
    mounted: roomActive ? mounted : false,
    initialError,
  }
}
