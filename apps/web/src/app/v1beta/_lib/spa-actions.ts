import { ClientError } from 'graphql-request'

import type { AuthTokens } from '@/auth/cognito'

const AUTH_STORAGE_KEY = 'library_auth'

interface AuthContext {
  accessToken: string
  userId: string
}

export const getAuthContext = (): AuthContext | null => {
  if (typeof window === 'undefined' || typeof localStorage === 'undefined') {
    return null
  }

  const stored = localStorage.getItem(AUTH_STORAGE_KEY)
  if (!stored) {
    return null
  }

  try {
    const parsed = JSON.parse(stored) as Partial<AuthTokens>
    if (!parsed.accessToken || !parsed.userId) {
      return null
    }
    return { accessToken: parsed.accessToken, userId: parsed.userId }
  } catch {
    return null
  }
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
