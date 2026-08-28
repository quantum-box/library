import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { signInWithHostedUiCode, signInWithCredentials } from './cognito'
import {
  clearTokens,
  getValidAccessToken,
  loadStoredTokens,
  startAuthTokenWatch,
  storeTokens,
  subscribeAuthChange,
  type AuthTokens,
} from './token-manager'

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

function tokensToSession(tokens: AuthTokens | null): AuthSession | null {
  if (!tokens) return null
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
  // Stored tokens are trusted up front even when the access token is stale:
  // the token manager refreshes it, and only the manager decides to sign out.
  const [session, setSession] = useState<AuthSession | null>(() =>
    tokensToSession(loadStoredTokens()),
  )
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const unsubscribe = subscribeAuthChange(() => {
      setSession(tokensToSession(loadStoredTokens()))
    })
    const stopWatch = startAuthTokenWatch()

    // Refreshes now if the stored token is at or near expiry.
    void getValidAccessToken().finally(() => setIsLoading(false))

    return () => {
      unsubscribe()
      stopWatch()
    }
  }, [])

  const signIn = useCallback(async (username: string, password: string) => {
    const tokens = await signInWithCredentials(username, password)
    storeTokens(tokens, 'signed-in')
  }, [])

  const signInHostedUiCode = useCallback(
    async (code: string, codeVerifier: string, redirectUri: string) => {
      const tokens = await signInWithHostedUiCode(code, codeVerifier, redirectUri)
      storeTokens(tokens, 'signed-in')
    },
    [],
  )

  const signOut = useCallback(() => {
    clearTokens('signed-out')
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
  return {
    data: session,
    status: isLoading ? 'loading' : session ? 'authenticated' : 'unauthenticated',
  } as const
}
