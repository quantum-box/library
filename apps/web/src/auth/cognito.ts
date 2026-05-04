import {
  CognitoIdentityProvider,
  GetUserCommand,
  InitiateAuthCommand,
  SignUpCommand,
  ForgotPasswordCommand,
  ConfirmForgotPasswordCommand,
} from '@aws-sdk/client-cognito-identity-provider'
import { getSdkPlatform, platformId } from '@/lib/apiClient'

const getCognitoConfig = () => ({
  clientId: import.meta.env.VITE_COGNITO_CLIENT_ID ?? '',
  clientSecret: import.meta.env.VITE_COGNITO_CLIENT_SECRET ?? '',
  region: import.meta.env.VITE_COGNITO_REGION ?? 'ap-northeast-1',
})

const cognitoClient = () =>
  new CognitoIdentityProvider({
    region: getCognitoConfig().region,
  })

export async function generateSecretHash(
  username: string,
  clientId: string,
  clientSecret: string,
): Promise<string> {
  const encoder = new TextEncoder()
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(clientSecret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  )
  const signature = await crypto.subtle.sign(
    'HMAC',
    key,
    encoder.encode(username + clientId),
  )
  return btoa(String.fromCharCode(...Array.from(new Uint8Array(signature))))
}

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
        SECRET_HASH: await generateSecretHash(
          username,
          config.clientId,
          config.clientSecret,
        ),
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
        SECRET_HASH: await generateSecretHash(
          username,
          config.clientId,
          config.clientSecret,
        ),
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
      SecretHash: await generateSecretHash(
        username,
        config.clientId,
        config.clientSecret,
      ),
      UserAttributes: [{ Name: 'email', Value: email }],
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
      SecretHash: await generateSecretHash(
        usernameOrEmail,
        config.clientId,
        config.clientSecret,
      ),
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
      SecretHash: await generateSecretHash(
        username,
        config.clientId,
        config.clientSecret,
      ),
    }),
  )
}
