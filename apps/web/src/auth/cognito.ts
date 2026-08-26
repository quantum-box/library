import {
  CognitoIdentityProvider,
  ConfirmSignUpCommand,
  GetUserCommand,
  InitiateAuthCommand,
  ResendConfirmationCodeCommand,
  SignUpCommand,
  ForgotPasswordCommand,
  ConfirmForgotPasswordCommand,
} from '@aws-sdk/client-cognito-identity-provider'
import { getSdkPlatform, platformId } from '@/lib/apiClient'

const requireCognitoEnv = (name: string, value: string | undefined): string => {
  const resolvedValue = value?.trim()

  if (!resolvedValue) {
    throw new Error(`${name} is required for Cognito authentication`)
  }

  return resolvedValue
}

const cognitoConfig = {
  clientId: requireCognitoEnv(
    'VITE_COGNITO_CLIENT_ID',
    import.meta.env.VITE_COGNITO_CLIENT_ID,
  ),
  region: import.meta.env.VITE_COGNITO_REGION?.trim() || 'ap-northeast-1',
  hostedUiDomain: import.meta.env.VITE_COGNITO_HOSTED_UI_DOMAIN?.trim() ?? '',
}

const getCognitoConfig = () => cognitoConfig

const cognitoClient = () =>
  new CognitoIdentityProvider({
    region: getCognitoConfig().region,
  })

export interface AuthTokens {
  accessToken: string
  refreshToken: string
  expiresAt: number
  userId: string
  email: string
  username: string
}

export async function signInWithCredentials(
  username: string,
  password: string,
): Promise<AuthTokens> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  const response = await client.send(
    new InitiateAuthCommand({
      AuthFlow: 'USER_PASSWORD_AUTH',
      ClientId: config.clientId,
      AuthParameters: {
        USERNAME: username,
        PASSWORD: password,
      },
    }),
  )

  if (!response.AuthenticationResult?.AccessToken) {
    throw new Error('No Access Token')
  }

  const { AccessToken, ExpiresIn, RefreshToken } = response.AuthenticationResult

  // Get user info
  const userResponse = await client.send(
    new GetUserCommand({ AccessToken }),
  )

  const email =
    userResponse.UserAttributes?.find((attr) => attr.Name === 'email')?.Value ?? ''
  // Sign-in accepts an email alias, so trust Cognito for the account's username.
  const resolvedUsername = userResponse.Username ?? username

  // Register with platform
  const sdk = getSdkPlatform(AccessToken)
  const { signIn: user } = await sdk.signInOrSignUp({
    platformId,
    accessToken: AccessToken,
    allowSignUp: true,
  })

  return {
    accessToken: AccessToken,
    refreshToken: RefreshToken ?? '',
    expiresAt: Math.floor(Date.now() / 1000 + (ExpiresIn ?? 3600)),
    userId: user.id,
    email,
    username: resolvedUsername,
  }
}

export async function signInWithHostedUiCode(
  code: string,
  codeVerifier: string,
  redirectUri: string,
): Promise<AuthTokens> {
  const config = getCognitoConfig()

  if (!config.hostedUiDomain) {
    throw new Error('VITE_COGNITO_HOSTED_UI_DOMAIN is required for Hosted UI')
  }

  const domain = config.hostedUiDomain.replace(/^https?:\/\//, '')
  const tokenUrl = new URL('/oauth2/token', `https://${domain}`)
  const body = new URLSearchParams()
  body.set('grant_type', 'authorization_code')
  body.set('client_id', config.clientId)
  body.set('code', code)
  body.set('redirect_uri', redirectUri)
  body.set('code_verifier', codeVerifier)

  const response = await fetch(tokenUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body,
  })

  const tokenResponse = (await response.json().catch(() => null)) as {
    access_token?: string
    refresh_token?: string
    expires_in?: number
    error?: string
  } | null

  if (!response.ok || !tokenResponse?.access_token) {
    throw new Error(tokenResponse?.error || 'Hosted UI token exchange failed')
  }

  const userInfoResponse = await fetch(new URL('/oauth2/userInfo', `https://${domain}`), {
    headers: {
      Authorization: `Bearer ${tokenResponse.access_token}`,
    },
  })

  const userInfo = (await userInfoResponse.json().catch(() => null)) as {
    sub?: string
    username?: string
    email?: string
  } | null

  if (!userInfoResponse.ok || !userInfo) {
    throw new Error('Hosted UI user info request failed')
  }

  const email = userInfo.email ?? ''
  const username = userInfo.username || email || userInfo.sub || ''

  const sdk = getSdkPlatform(tokenResponse.access_token)
  const { signIn: user } = await sdk.signInOrSignUp({
    platformId,
    accessToken: tokenResponse.access_token,
    allowSignUp: true,
  })

  return {
    accessToken: tokenResponse.access_token,
    refreshToken: tokenResponse.refresh_token ?? '',
    expiresAt: Math.floor(Date.now() / 1000 + (tokenResponse.expires_in ?? 3600)),
    userId: user.id,
    email,
    username,
  }
}

export async function refreshAccessToken(
  refreshToken: string,
  username: string,
): Promise<AuthTokens> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  const response = await client.send(
    new InitiateAuthCommand({
      AuthFlow: 'REFRESH_TOKEN_AUTH',
      ClientId: config.clientId,
      AuthParameters: {
        REFRESH_TOKEN: refreshToken,
      },
    }),
  )

  if (!response.AuthenticationResult?.AccessToken) {
    throw new Error('No access token from refresh')
  }

  const { AccessToken, ExpiresIn, RefreshToken } = response.AuthenticationResult

  const sdk = getSdkPlatform(AccessToken)
  const { signIn: user } = await sdk.signInOrSignUp({
    platformId,
    accessToken: AccessToken,
    allowSignUp: true,
  })

  return {
    accessToken: AccessToken,
    refreshToken: RefreshToken ?? refreshToken,
    expiresAt: Math.floor(Date.now() / 1000 + (ExpiresIn ?? 3600)),
    userId: user.id,
    email: '', // email not returned on refresh
    username,
  }
}

export async function signUpWithCredentials(
  username: string,
  email: string,
  password: string,
): Promise<void> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  await client.send(
    new SignUpCommand({
      ClientId: config.clientId,
      Username: username,
      Password: password,
      UserAttributes: [{ Name: 'email', Value: email }],
    }),
  )
}

export async function confirmSignUpWithCode(
  username: string,
  code: string,
): Promise<void> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  await client.send(
    new ConfirmSignUpCommand({
      ClientId: config.clientId,
      Username: username,
      ConfirmationCode: code,
    }),
  )
}

export async function resendSignUpConfirmationCode(
  username: string,
): Promise<void> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  await client.send(
    new ResendConfirmationCodeCommand({
      ClientId: config.clientId,
      Username: username,
    }),
  )
}

export async function forgotPassword(usernameOrEmail: string): Promise<void> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  await client.send(
    new ForgotPasswordCommand({
      ClientId: config.clientId,
      Username: usernameOrEmail,
    }),
  )
}

export async function resetPasswordWithToken(
  username: string,
  token: string,
  newPassword: string,
): Promise<void> {
  const config = getCognitoConfig()
  const client = cognitoClient()

  await client.send(
    new ConfirmForgotPasswordCommand({
      ClientId: config.clientId,
      Username: username,
      ConfirmationCode: token,
      Password: newPassword,
    }),
  )
}
