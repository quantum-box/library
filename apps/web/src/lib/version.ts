export const frontendVersion = '1.11.1'

declare const __GIT_COMMIT_HASH__: string
export const commitHash: string = (() => {
  try {
    return __GIT_COMMIT_HASH__
  } catch {
    return 'dev'
  }
})()

export const fetchBackendVersion = async (): Promise<string | null> => {
  const baseUrl =
    import.meta.env.VITE_BACKEND_API_URL || 'http://localhost:50053'
  const normalizedBaseUrl = baseUrl.replace(/\/$/, '')

  try {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), 5_000)
    const response = await fetch(`${normalizedBaseUrl}/version`, {
      signal: controller.signal,
    })
    clearTimeout(timeoutId)

    if (!response.ok) return null
    const data: unknown = await response.json()
    if (typeof data === 'object' && data !== null && 'version' in data) {
      return (data as { version: string }).version
    }
    return null
  } catch {
    return null
  }
}
