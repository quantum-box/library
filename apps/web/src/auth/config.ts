import { CognitoIdentityProvider } from '@aws-sdk/client-cognito-identity-provider'

export interface CognitoConfig {
  clientId: string
  region: string
  hostedUiDomain: string
}

const requireCognitoEnv = (name: string, value: string | undefined): string => {
  const resolvedValue = value?.trim()

  if (!resolvedValue) {
    throw new Error(`${name} is required for Cognito authentication`)
  }

  return resolvedValue
}

/**
 * Resolved on demand rather than at import time so a missing client id fails
 * the sign-in that needs it instead of blanking the whole app.
 */
export const getCognitoConfig = (): CognitoConfig => ({
  clientId: requireCognitoEnv(
    'VITE_COGNITO_CLIENT_ID',
    import.meta.env.VITE_COGNITO_CLIENT_ID,
  ),
  region: import.meta.env.VITE_COGNITO_REGION?.trim() || 'ap-northeast-1',
  hostedUiDomain: import.meta.env.VITE_COGNITO_HOSTED_UI_DOMAIN?.trim() ?? '',
})

export const cognitoClient = () =>
  new CognitoIdentityProvider({
    region: getCognitoConfig().region,
  })
