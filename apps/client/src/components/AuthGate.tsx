import { type ReactNode, useEffect, useState } from 'react'
import { loadAuthTokens, signInWithCredentials, storeAuthTokens } from '../lib/auth'

interface AuthGateProps {
  children: ReactNode
}

export function AuthGate({ children }: AuthGateProps) {
  const [session, setSession] = useState(() => loadAuthTokens())
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [token, setToken] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const reload = () => setSession(loadAuthTokens())
    window.addEventListener('library-auth-change', reload)
    return () => window.removeEventListener('library-auth-change', reload)
  }, [])

  const handleCredentialSignIn = async () => {
    if (!username.trim() || !password) return
    setBusy(true)
    setError(null)
    try {
      const tokens = await signInWithCredentials(username.trim(), password)
      storeAuthTokens(tokens)
      setPassword('')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Sign in failed')
    } finally {
      setBusy(false)
    }
  }

  const handleTokenSignIn = () => {
    const accessToken = token.trim()
    if (!accessToken) return
    storeAuthTokens({
      accessToken,
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'manual-token-user',
      email: '',
      username: 'manual-token',
    })
    setToken('')
  }

  if (session?.accessToken) return <>{children}</>

  return (
    <main className="flex h-full min-h-0 bg-canvas text-foreground">
      <section className="hidden min-h-0 w-[42vw] min-w-[440px] border-r border-border bg-panel lg:flex">
        <div className="flex w-full flex-col justify-between p-10">
          <div className="flex items-center gap-3">
            <div className="flex h-9 w-9 items-center justify-center rounded border border-border bg-surface text-sm font-semibold">
              L
            </div>
            <div>
              <div className="text-sm font-semibold">Library</div>
              <div className="text-xs text-subtle">Production workspace</div>
            </div>
          </div>

          <div className="max-w-md">
            <div className="mb-6 h-px w-16 bg-foreground" />
            <h1 className="text-[38px] font-semibold leading-[1.05] tracking-normal">
              Sign in to your Library tenant.
            </h1>
            <p className="mt-5 text-sm leading-6 text-muted">
              Organizations and repositories are resolved from your account after authentication.
            </p>
          </div>

          <div className="grid grid-cols-3 gap-3 text-xs text-subtle">
            <div className="border-t border-border pt-3">Cognito</div>
            <div className="border-t border-border pt-3">Organizations</div>
            <div className="border-t border-border pt-3">Repositories</div>
          </div>
        </div>
      </section>

      <section className="flex min-w-0 flex-1 items-center justify-center px-5 py-8">
        <div className="w-full max-w-[380px]">
          <div className="mb-8 lg:hidden">
            <div className="mb-5 flex h-9 w-9 items-center justify-center rounded border border-border bg-surface text-sm font-semibold">
              L
            </div>
            <h1 className="text-2xl font-semibold leading-tight">Library</h1>
          </div>

          <div className="border border-border bg-surface p-5 shadow-soft">
            <div className="mb-5">
              <h2 className="text-base font-semibold">Sign in</h2>
              <p className="mt-1 text-xs text-subtle">library-api.txcloud.app</p>
            </div>

            <div className="flex flex-col gap-3">
              <label className="flex flex-col gap-1.5 text-xs font-medium text-muted">
                Email or username
                <input
                  className="h-10 rounded-md border border-border bg-surface px-3 text-sm text-foreground outline-none focus:border-accent"
                  autoComplete="username"
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                />
              </label>
              <label className="flex flex-col gap-1.5 text-xs font-medium text-muted">
                Password
                <input
                  className="h-10 rounded-md border border-border bg-surface px-3 text-sm text-foreground outline-none focus:border-accent"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') void handleCredentialSignIn()
                  }}
                />
              </label>
              <button
                type="button"
                className="mt-1 h-10 rounded-md bg-accent px-3 text-sm font-medium text-primary-foreground disabled:opacity-50"
                disabled={busy || !username.trim() || !password}
                onClick={() => void handleCredentialSignIn()}
              >
                {busy ? 'Signing in...' : 'Sign in'}
              </button>
            </div>

            <div className="my-5 flex items-center gap-3 text-[11px] uppercase tracking-wider text-subtle">
              <div className="h-px flex-1 bg-border" />
              Access token
              <div className="h-px flex-1 bg-border" />
            </div>

            <div className="flex flex-col gap-3">
              <textarea
                className="min-h-24 rounded-md border border-border bg-surface px-3 py-2 text-xs text-foreground outline-none focus:border-accent"
                value={token}
                onChange={(event) => setToken(event.target.value)}
              />
              <button
                type="button"
                className="h-9 rounded-md bg-surface-hover px-3 text-sm font-medium text-muted hover:text-foreground disabled:opacity-50"
                disabled={!token.trim()}
                onClick={handleTokenSignIn}
              >
                Use token
              </button>
            </div>

            {error && (
              <div className="mt-4 rounded-md border border-status-cancelled/30 bg-status-cancelled/10 px-3 py-2 text-xs leading-5 text-status-cancelled">
                {error}
              </div>
            )}
          </div>
        </div>
      </section>
    </main>
  )
}
