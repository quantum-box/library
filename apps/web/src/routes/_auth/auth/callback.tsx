import { useAuth } from '@/auth'
import { consumeHostedUiSession } from '@/auth/hosted-ui'
import { useToastWithError } from '@/lib/error-toast'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { Loader2 } from 'lucide-react'
import { useEffect, useState } from 'react'

export const Route = createFileRoute('/_auth/auth/callback')({
  component: AuthCallbackPage,
})

function AuthCallbackPage() {
  const { signInWithHostedUiCode } = useAuth()
  const navigate = useNavigate()
  const { errorToast } = useToastWithError()
  const [message, setMessage] = useState('Completing sign in...')

  useEffect(() => {
    let isMounted = true

    const completeSignIn = async () => {
      const searchParams = new URLSearchParams(window.location.search)
      const code = searchParams.get('code')
      const state = searchParams.get('state')
      const error = searchParams.get('error')

      try {
        if (error) {
          throw new Error(searchParams.get('error_description') || error)
        }

        if (!code || !state) {
          throw new Error('Missing Hosted UI callback parameters')
        }

        const session = consumeHostedUiSession(state)
        await signInWithHostedUiCode(
          code,
          session.codeVerifier,
          session.redirectUri,
        )
        navigate({ to: session.returnTo })
      } catch (callbackError) {
        if (!isMounted) return
        console.error('Hosted UI sign-in error:', callbackError)
        setMessage('Sign in failed. Please try again.')
        errorToast(callbackError)
        navigate({ to: '/sign_in' })
      }
    }

    void completeSignIn()

    return () => {
      isMounted = false
    }
  }, [errorToast, navigate, signInWithHostedUiCode])

  return (
    <div className='flex min-h-screen items-center justify-center'>
      <div className='flex items-center gap-2 text-sm text-muted-foreground'>
        <Loader2 className='h-4 w-4 animate-spin' />
        {message}
      </div>
    </div>
  )
}
