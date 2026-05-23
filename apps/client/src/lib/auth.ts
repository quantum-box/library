export interface AuthTokens {
  accessToken: string
  refreshToken: string
  expiresAt: number
  userId: string
  email: string
  username: string
}

const authStorageKey = 'library_auth'

function configuredCognitoClientId(): string {
  return import.meta.env.VITE_COGNITO_CLIENT_ID ?? '5b0a8ncbi6v189p3un146fb6rf'
}

function configuredCognitoRegion(): string {
  return import.meta.env.VITE_COGNITO_REGION ?? 'ap-northeast-1'
}

function emitAuthChange() {
  window.dispatchEvent(new CustomEvent('library-auth-change'))
}

export function loadAuthTokens(): AuthTokens | null {
  try {
    const stored = localStorage.getItem(authStorageKey)
    if (!stored) return null
    const tokens = JSON.parse(stored) as AuthTokens
    if (tokens.expiresAt && tokens.expiresAt <= Math.floor(Date.now() / 1000)) {
      localStorage.removeItem(authStorageKey)
      return null
    }
    return tokens.accessToken ? tokens : null
  } catch {
    return null
  }
}

export function storeAuthTokens(tokens: AuthTokens) {
  localStorage.setItem(authStorageKey, JSON.stringify(tokens))
  emitAuthChange()
}

export function clearAuthTokens() {
  localStorage.removeItem(authStorageKey)
  emitAuthChange()
}

async function requestCognito<T>(target: string, body: Record<string, unknown>): Promise<T> {
  const response = await fetch(`https://cognito-idp.${configuredCognitoRegion()}.amazonaws.com/`, {
    method: 'POST',
    headers: {
      'content-type': 'application/x-amz-json-1.1',
      'x-amz-target': `AWSCognitoIdentityProviderService.${target}`,
    },
    body: JSON.stringify(body),
  })
  const payload = await response.json().catch(() => ({})) as { message?: string; __type?: string }
  if (!response.ok) {
    throw new Error(payload.message ?? payload.__type ?? `Cognito ${target} failed`)
  }
  return payload as T
}

export async function signInWithCredentials(username: string, password: string): Promise<AuthTokens> {
  const auth = await requestCognito<{
    AuthenticationResult?: {
      AccessToken?: string
      RefreshToken?: string
      ExpiresIn?: number
    }
  }>('InitiateAuth', {
    AuthFlow: 'USER_PASSWORD_AUTH',
    ClientId: configuredCognitoClientId(),
    AuthParameters: {
      USERNAME: username,
      PASSWORD: password,
    },
  })

  const accessToken = auth.AuthenticationResult?.AccessToken
  if (!accessToken) throw new Error('No access token returned from Cognito')

  const user = await requestCognito<{
    UserAttributes?: Array<{ Name?: string; Value?: string }>
  }>('GetUser', { AccessToken: accessToken })
  const email = user.UserAttributes?.find((attr) => attr.Name === 'email')?.Value ?? ''
  const userId = user.UserAttributes?.find((attr) => attr.Name === 'sub')?.Value ?? username

  return {
    accessToken,
    refreshToken: auth.AuthenticationResult?.RefreshToken ?? '',
    expiresAt: Math.floor(Date.now() / 1000 + (auth.AuthenticationResult?.ExpiresIn ?? 3600)),
    userId,
    email,
    username,
  }
}
