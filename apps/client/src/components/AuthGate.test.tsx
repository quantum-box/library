import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AuthGate } from './AuthGate'

const getValidAuthTokens = vi.fn()

vi.mock('../lib/auth', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/auth')>()
  return {
    ...actual,
    getValidAuthTokens: () => getValidAuthTokens(),
    signInWithCredentials: vi.fn(),
    storeAuthTokens: vi.fn(),
  }
})

describe('AuthGate', () => {
  beforeEach(() => {
    getValidAuthTokens.mockResolvedValue(null)
  })

  it('does not expose the authentication provider on the sign-in screen', async () => {
    render(
      <AuthGate>
        <div>Authenticated content</div>
      </AuthGate>,
    )

    await screen.findByLabelText('Email or username')
    expect(screen.queryByText(/Cognito/i)).toBeNull()
  })
})
