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

  it('shows anonymous repository content when there is no session', async () => {
    render(<AuthGate anonymousContent={<div>Public repository</div>}>
      <div>Authenticated content</div>
    </AuthGate>)
    await screen.findByText('Public repository')
    expect(screen.queryByText('Authenticated content')).toBeNull()
    expect(screen.queryByLabelText('Email or username')).toBeNull()
  })

  it('keeps authenticated visitors in the workspace', async () => {
    getValidAuthTokens.mockResolvedValue({ accessToken: 'valid-token' })
    render(<AuthGate anonymousContent={<div>Public repository</div>}>
      <div>Authenticated content</div>
    </AuthGate>)
    await screen.findByText('Authenticated content')
    expect(screen.queryByText('Public repository')).toBeNull()
  })
})
