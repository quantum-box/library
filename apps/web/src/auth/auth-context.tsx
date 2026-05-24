import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import {
  type AuthTokens,
  signInWithHostedUiCode,
  refreshAccessToken,
  signInWithCredentials,
} from './cognito'

const AUTH_STORAGE_KEY = 'library_auth'

export interface AuthSession {
  user: {
    id: string
    email: string
    username: string
    accessToken: string
    refreshToken: string
    name?: string | null
    image?: string | null
    emailVerified: null
  }
  error?: string
}

interface AuthContextValue {
  session: AuthSession | null
  isLoading: boolean
  signIn: (username: string, password: string) => Promise<void>
  signInWithHostedUiCode: (
    code: string,
    codeVerifier: string,
    redirectUri: string,
  ) => Promise<void>
  signOut: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

function loadStoredTokens(): AuthTokens | null {
  try {
    const stored = localStorage.getItem(AUTH_STORAGE_KEY)
    if (!stored) return null
    return JSON.parse(stored) as AuthTokens
  } catch {
    return null
  }
}

function storeTokens(tokens: AuthTokens): void {
  localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(tokens))
}

function clearTokens(): void {
  localStorage.removeItem(AUTH_STORAGE_KEY)
}

function tokensToSession(tokens: AuthTokens): AuthSession {
  return {
    user: {
      id: tokens.userId,
      email: tokens.email,
      username: tokens.username,
      accessToken: tokens.accessToken,
      refreshToken: tokens.refreshToken,
      emailVerified: null,
    },
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<AuthSession | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  // Initialize from stored tokens
  useEffect(() => {
    const tokens = loadStoredTokens()
    if (tokens) {
      // Check if token is expired
      if (Date.now() / 1000 > tokens.expiresAt) {
        // Try to refresh
        refreshAccessToken(tokens.refreshToken, tokens.username)
          .then((newTokens) => {
            // Preserve email from stored tokens if refresh doesn't return it
            newTokens.email = newTokens.email || tokens.email
            storeTokens(newTokens)
            setSession(tokensToSession(newTokens))
          })
          .catch(() => {
            clearTokens()
            setSession(null)
          })
          .finally(() => setIsLoading(false))
      } else {
        setSession(tokensToSession(tokens))
        setIsLoading(false)
      }
    } else {
      setIsLoading(false)
    }
  }, [])

  // Auto-refresh token before expiry
  useEffect(() => {
    const tokens = loadStoredTokens()
    if (!tokens) return

    const expiresInMs = (tokens.expiresAt - Date.now() / 1000) * 1000
    // Refresh 5 minutes before expiry
    const refreshIn = Math.max(expiresInMs - 5 * 60 * 1000, 0)

    const timer = setTimeout(async () => {
      try {
        const newTokens = await refreshAccessToken(
          tokens.refreshToken,
          tokens.username,
        )
        newTokens.email = newTokens.email || tokens.email
        storeTokens(newTokens)
        setSession(tokensToSession(newTokens))
      } catch {
        clearTokens()
        setSession(null)
      }
    }, refreshIn)

    return () => clearTimeout(timer)
  }, [session])

  const signIn = useCallback(async (username: string, password: string) => {
    const tokens = await signInWithCredentials(username, password)
    storeTokens(tokens)
    setSession(tokensToSession(tokens))
  }, [])

  const signInHostedUiCode = useCallback(
    async (code: string, codeVerifier: string, redirectUri: string) => {
      const tokens = await signInWithHostedUiCode(code, codeVerifier, redirectUri)
      storeTokens(tokens)
      setSession(tokensToSession(tokens))
    },
    [],
  )

  const signOut = useCallback(() => {
    clearTokens()
    setSession(null)
  }, [])

  const value = useMemo(
    () => ({
      session,
      isLoading,
      signIn,
      signInWithHostedUiCode: signInHostedUiCode,
      signOut,
    }),
    [session, isLoading, signIn, signInHostedUiCode, signOut],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}

export function useSession() {
  const { session, isLoading } = useAuth()
  return { data: session, status: isLoading ? 'loading' : session ? 'authenticated' : 'unauthenticated' } as const
}
