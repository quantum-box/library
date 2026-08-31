import { Badge, Button, Input, Label, Separator } from '@tachyon-sdk/native-ui'
import { GitBranch, LoaderCircle, LockKeyhole, Network } from 'lucide-react'
import { type FormEvent, type ReactNode, useEffect, useState } from 'react'
import libraryAppIcon from '../assets/brand/library-logo/app-icon.svg'
import {
  getValidAuthTokens,
  signInWithCredentials,
  storeAuthTokens,
  type AuthTokens,
} from '../lib/auth'
import { useI18n } from '../i18n'

interface AuthGateProps {
  children: ReactNode
}

const developmentTokenSignInEnabled =
  import.meta.env.VITE_ENABLE_DEV_TOKEN_AUTH === 'true'

export function AuthGate({ children }: AuthGateProps) {
  const { t } = useI18n()
  const [session, setSession] = useState<AuthTokens | null>(null)
  const [checkingSession, setCheckingSession] = useState(true)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [token, setToken] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true

    const reloadSession = async () => {
      const nextSession = await getValidAuthTokens()
      if (!active) return
      setSession(nextSession)
      setCheckingSession(false)
    }

    const handleAuthChange = () => {
      void reloadSession()
    }

    window.addEventListener('library-auth-change', handleAuthChange)
    void reloadSession()

    return () => {
      active = false
      window.removeEventListener('library-auth-change', handleAuthChange)
    }
  }, [])

  const handleCredentialSignIn = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!username.trim() || !password) return

    setBusy(true)
    setError(null)
    try {
      const tokens = await signInWithCredentials(username.trim(), password)
      storeAuthTokens(tokens)
      setPassword('')

      // appKitConfig scopes local databases at module initialization. Reloading
      // guarantees the authenticated user's cache is selected immediately.
      window.location.reload()
    } catch (signInError) {
      setError(signInError instanceof Error ? signInError.message : t('auth.signInFailed'))
      setBusy(false)
    }
  }

  const handleDevelopmentTokenSignIn = () => {
    const accessToken = token.trim()
    if (!accessToken) return

    storeAuthTokens({
      accessToken,
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'development-token-user',
      email: '',
      username: 'development-token',
    })
    setToken('')
    window.location.reload()
  }

  if (session?.accessToken) return <>{children}</>

  return (
    <main className="grid h-full min-h-0 bg-background text-foreground lg:grid-cols-[minmax(420px,0.9fr)_minmax(480px,1.1fr)]">
      <section className="relative hidden min-h-0 overflow-hidden border-r border-border bg-surface lg:flex">
        <div className="absolute inset-0 opacity-70 [background-image:linear-gradient(to_right,hsl(var(--nui-border))_1px,transparent_1px),linear-gradient(to_bottom,hsl(var(--nui-border))_1px,transparent_1px)] [background-size:36px_36px] [mask-image:linear-gradient(to_bottom,black,transparent_88%)]" />
        <div className="relative flex w-full flex-col justify-between p-10 xl:p-14">
          <div className="flex items-center gap-2.5">
            <span className="flex size-8 items-center justify-center rounded-md border border-border bg-background shadow-soft">
              <img src={libraryAppIcon} alt="" className="size-5" />
            </span>
            <div>
              <div className="text-sm font-semibold">Library</div>
              <div className="font-mono text-2xs text-subtle-foreground">
                {t('auth.tagline')}
              </div>
            </div>
          </div>

          <div className="max-w-lg">
            <Badge variant="outline">{t('auth.pathShape')}</Badge>
            <h1 className="mt-5 text-[34px] font-semibold leading-[1.08] tracking-tight xl:text-[40px]">
              {t('auth.headline')}
            </h1>
            <p className="mt-5 max-w-md text-sm leading-6 text-muted-foreground">
              {t('auth.subheadline')}
            </p>

            <div className="mt-9 grid gap-3 text-xs text-muted-foreground">
              <div className="flex items-center gap-3 rounded-lg border border-border bg-background/80 px-3 py-2.5">
                <Network className="size-4 text-primary" aria-hidden="true" />
                <span>{t('auth.pointAccess')}</span>
              </div>
              <div className="flex items-center gap-3 rounded-lg border border-border bg-background/80 px-3 py-2.5">
                <GitBranch className="size-4 text-primary" aria-hidden="true" />
                <span>{t('auth.pointRepositories')}</span>
              </div>
            </div>
          </div>

          <div className="font-mono text-2xs text-subtle-foreground">
            {t('auth.platforms')}
          </div>
        </div>
      </section>

      {/* The shell pins the viewport, so this column owns its own scrolling:
          `min-h-full` keeps the form centred when it fits and lets it scroll
          when it does not — a phone in landscape, or a tall error message. */}
      <section className="min-w-0 overflow-y-auto px-5 py-8 sm:px-8">
        <div className="mx-auto flex min-h-full w-full max-w-[380px] flex-col justify-center">
          <div className="mb-8 lg:hidden">
            <span className="mb-4 flex size-9 items-center justify-center rounded-md border border-border bg-surface shadow-soft">
              <img src={libraryAppIcon} alt="" className="size-5" />
            </span>
            <h1 className="text-2xl font-semibold tracking-tight">Library</h1>
            <p className="mt-1 text-sm text-muted-foreground">{t('auth.mobileSubtitle')}</p>
          </div>

          <div className="mb-6 hidden lg:block">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <LockKeyhole className="size-4 text-muted-foreground" aria-hidden="true" />
              {t('auth.signIn')}
            </div>
            <p className="mt-1.5 text-sm text-muted-foreground">{t('auth.signInHint')}</p>
          </div>

          {checkingSession ? (
            <div className="flex h-40 items-center justify-center rounded-lg border border-border bg-surface">
              <LoaderCircle className="size-5 animate-spin text-muted-foreground" aria-label={t('auth.checkingSession')} />
            </div>
          ) : (
            <form className="space-y-4" onSubmit={(event) => void handleCredentialSignIn(event)}>
              <div className="space-y-1.5">
                <Label htmlFor="library-username">{t('auth.usernameLabel')}</Label>
                <Input
                  id="library-username"
                  className="h-9"
                  autoComplete="username"
                  autoCapitalize="none"
                  spellCheck={false}
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                  disabled={busy}
                />
              </div>

              <div className="space-y-1.5">
                <Label htmlFor="library-password">{t('auth.passwordLabel')}</Label>
                <Input
                  id="library-password"
                  className="h-9"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  disabled={busy}
                />
              </div>

              {error && (
                <div
                  className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs leading-5 text-destructive"
                  role="alert"
                >
                  {error}
                </div>
              )}

              <Button
                type="submit"
                variant="primary"
                size="lg"
                className="w-full"
                disabled={busy || !username.trim() || !password}
              >
                {busy && <LoaderCircle className="animate-spin" aria-hidden="true" />}
                {busy ? t('auth.signingIn') : t('auth.signIn')}
              </Button>
            </form>
          )}

          {developmentTokenSignInEnabled && !checkingSession && (
            <div className="mt-6">
              <div className="mb-4 flex items-center gap-3">
                <Separator className="flex-1" />
                <span className="font-mono text-2xs uppercase tracking-wider text-subtle-foreground">
                  {t('auth.developmentOnly')}
                </span>
                <Separator className="flex-1" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="library-development-token">{t('auth.accessToken')}</Label>
                <textarea
                  id="library-development-token"
                  className="min-h-20 w-full resize-y rounded-md border border-input bg-background px-2 py-2 font-mono text-xs text-foreground outline-none transition-colors duration-fast placeholder:text-subtle-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25"
                  value={token}
                  onChange={(event) => setToken(event.target.value)}
                />
                <Button
                  type="button"
                  className="w-full"
                  disabled={!token.trim()}
                  onClick={handleDevelopmentTokenSignIn}
                >
                  {t('auth.useDevelopmentToken')}
                </Button>
              </div>
            </div>
          )}

          <p className="mt-6 text-center text-2xs leading-5 text-subtle-foreground">
            {t('auth.cognitoNote')}
          </p>
        </div>
      </section>
    </main>
  )
}
