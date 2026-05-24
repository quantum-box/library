export type HostedUiSignInKind = 'google' | 'passkey'

export type HostedUiConfig =
  | {
      ok: true
      hostedUiDomain: string
      clientId: string
      redirectUri: string
    }
  | {
      ok: false
      missing: string[]
    }

export type CognitoAuthorizeUrlInput = {
  hostedUiDomain: string
  clientId: string
  redirectUri: string
  state: string
  codeChallenge: string
  identityProvider?: 'Google'
}

export const HOSTED_UI_SESSION_STORAGE_KEY = 'library_hosted_ui_oauth'

type HostedUiSession = {
  state: string
  codeVerifier: string
  redirectUri: string
  returnTo: string
}

const trimEnv = (value: string | undefined): string => value?.trim() ?? ''

const stripTrailingSlash = (value: string): string => value.replace(/\/+$/, '')

const base64UrlEncode = (bytes: Uint8Array): string => {
  const binary = Array.from(bytes, (byte) => String.fromCharCode(byte)).join('')
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

const createRandomString = (byteLength: number): string => {
  const bytes = new Uint8Array(byteLength)
  crypto.getRandomValues(bytes)
  return base64UrlEncode(bytes)
}

export function getHostedUiConfig(origin: string = window.location.origin): HostedUiConfig {
  const hostedUiDomain = trimEnv(import.meta.env.VITE_COGNITO_HOSTED_UI_DOMAIN)
  const clientId = trimEnv(import.meta.env.VITE_COGNITO_CLIENT_ID)
  const redirectUri =
    trimEnv(import.meta.env.VITE_COGNITO_REDIRECT_URI) ||
    `${stripTrailingSlash(origin)}/auth/callback`

  const missing = [
    !hostedUiDomain ? 'VITE_COGNITO_HOSTED_UI_DOMAIN' : null,
    !clientId ? 'VITE_COGNITO_CLIENT_ID' : null,
  ].filter((name): name is string => Boolean(name))

  if (missing.length > 0) {
    return { ok: false, missing }
  }

  return {
    ok: true,
    hostedUiDomain,
    clientId,
    redirectUri,
  }
}

export function buildCognitoAuthorizeUrl(
  input: CognitoAuthorizeUrlInput,
): string {
  const domain = input.hostedUiDomain.replace(/^https?:\/\//, '')
  const url = new URL('/oauth2/authorize', `https://${domain}`)

  url.searchParams.set('response_type', 'code')
  url.searchParams.set('client_id', input.clientId)
  url.searchParams.set('redirect_uri', input.redirectUri)
  url.searchParams.set('scope', 'openid email profile')
  url.searchParams.set('state', input.state)
  url.searchParams.set('code_challenge', input.codeChallenge)
  url.searchParams.set('code_challenge_method', 'S256')

  if (input.identityProvider) {
    url.searchParams.set('identity_provider', input.identityProvider)
  }

  return url.toString()
}

export function normalizeReturnTo(value: string | null): string {
  if (!value) return '/'

  const trimmed = value.trim()
  if (!trimmed || !trimmed.startsWith('/') || trimmed.startsWith('//')) {
    return '/'
  }

  return trimmed
}

export async function createHostedUiAuthorizeUrl(
  kind: HostedUiSignInKind,
  returnTo: string,
): Promise<string> {
  const config = getHostedUiConfig()

  if (!config.ok) {
    throw new Error(`Missing Cognito Hosted UI env: ${config.missing.join(', ')}`)
  }

  const codeVerifier = createRandomString(64)
  const digest = await crypto.subtle.digest(
    'SHA-256',
    new TextEncoder().encode(codeVerifier),
  )
  const codeChallenge = base64UrlEncode(new Uint8Array(digest))
  const state = createRandomString(32)
  const session: HostedUiSession = {
    state,
    codeVerifier,
    redirectUri: config.redirectUri,
    returnTo: normalizeReturnTo(returnTo),
  }

  sessionStorage.setItem(HOSTED_UI_SESSION_STORAGE_KEY, JSON.stringify(session))

  return buildCognitoAuthorizeUrl({
    hostedUiDomain: config.hostedUiDomain,
    clientId: config.clientId,
    redirectUri: config.redirectUri,
    state,
    codeChallenge,
    identityProvider: kind === 'google' ? 'Google' : undefined,
  })
}

export function consumeHostedUiSession(state: string): HostedUiSession {
  const stored = sessionStorage.getItem(HOSTED_UI_SESSION_STORAGE_KEY)
  sessionStorage.removeItem(HOSTED_UI_SESSION_STORAGE_KEY)

  if (!stored) {
    throw new Error('Missing Hosted UI session')
  }

  const session = JSON.parse(stored) as HostedUiSession
  if (session.state !== state) {
    throw new Error('Invalid Hosted UI state')
  }

  return session
}
