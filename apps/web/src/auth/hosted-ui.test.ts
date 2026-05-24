import { describe, expect, it } from 'vitest'
import { buildCognitoAuthorizeUrl, normalizeReturnTo } from './hosted-ui'

describe('buildCognitoAuthorizeUrl', () => {
  it('builds a deterministic Cognito Hosted UI URL for passkey', () => {
    const url = buildCognitoAuthorizeUrl({
      hostedUiDomain: 'example.auth.ap-northeast-1.amazoncognito.com',
      clientId: 'client-id',
      redirectUri: 'https://planet-library.txcloud.app/auth/callback',
      state: 'state',
      codeChallenge: 'challenge',
    })

    expect(url).toBe(
      'https://example.auth.ap-northeast-1.amazoncognito.com/oauth2/authorize?response_type=code&client_id=client-id&redirect_uri=https%3A%2F%2Fplanet-library.txcloud.app%2Fauth%2Fcallback&scope=openid+email+profile&state=state&code_challenge=challenge&code_challenge_method=S256',
    )
  })

  it('adds the Google identity provider when requested', () => {
    const url = new URL(
      buildCognitoAuthorizeUrl({
        hostedUiDomain: 'https://example.auth.ap-northeast-1.amazoncognito.com',
        clientId: 'client-id',
        redirectUri: 'https://planet-library.txcloud.app/auth/callback',
        state: 'state',
        codeChallenge: 'challenge',
        identityProvider: 'Google',
      }),
    )

    expect(url.searchParams.get('identity_provider')).toBe('Google')
  })
})

describe('normalizeReturnTo', () => {
  it('keeps same-origin relative paths', () => {
    expect(normalizeReturnTo('/v1beta/acme')).toBe('/v1beta/acme')
  })

  it('falls back for empty or external values', () => {
    expect(normalizeReturnTo('')).toBe('/')
    expect(normalizeReturnTo('https://example.com')).toBe('/')
    expect(normalizeReturnTo('//example.com')).toBe('/')
  })
})
