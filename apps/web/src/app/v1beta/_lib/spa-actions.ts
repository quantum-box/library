import { ClientError } from 'graphql-request'

import { loadStoredTokens } from '@/auth/token-manager'

interface AuthContext {
  accessToken: string
  userId: string
}

/**
 * The stored token may be stale; callers hand it to the API client, which
 * refreshes before the request goes out.
 */
export const getAuthContext = (): AuthContext | null => {
  const tokens = loadStoredTokens()
  if (!tokens?.accessToken || !tokens.userId) {
    return null
  }
  return { accessToken: tokens.accessToken, userId: tokens.userId }
}

export const getGraphQLErrorMessage = (error: unknown): string => {
  if (!error) {
    return 'Unknown error'
  }

  if (error instanceof ClientError) {
    const message = error.response.errors?.[0]?.message
    return message ?? error.message
  }

  if (error instanceof Error) {
    return error.message
  }

  const stringified = String(error)
  try {
    const parsed = JSON.parse(stringified) as {
      response?: {
        errors?: Array<{ message?: string }>
      }
    }
    const message = parsed?.response?.errors?.[0]?.message
    if (message) {
      return message
    }
  } catch {
    // not JSON
  }

  return stringified || 'Unknown error'
}
