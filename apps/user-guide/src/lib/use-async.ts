import { useEffect, useState } from 'react'

export interface AsyncState<T> {
  data?: T
  error?: string
  loading: boolean
}

/**
 * Run a fetch on mount and report its three states.
 *
 * Everything on this site that comes from the API is optional decoration
 * around prose that stands on its own, so a failure is surfaced as a
 * message beside the prose rather than replacing the page.
 */
export function useAsync<T>(
  load: (signal: AbortSignal) => Promise<T>,
  deps: unknown[] = [],
): AsyncState<T> {
  const [state, setState] = useState<AsyncState<T>>({ loading: true })

  useEffect(() => {
    const controller = new AbortController()
    let active = true

    setState({ loading: true })
    load(controller.signal)
      .then(data => {
        if (active) setState({ data, loading: false })
      })
      .catch((error: unknown) => {
        if (!active || controller.signal.aborted) return
        setState({
          error: error instanceof Error ? error.message : String(error),
          loading: false,
        })
      })

    return () => {
      active = false
      controller.abort()
    }
  }, deps)

  return state
}
