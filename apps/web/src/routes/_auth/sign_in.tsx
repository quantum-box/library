import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { SignInForm } from '@/app/(auth)/sign_in/form'
import type { SignInFormData } from '@/app/(auth)/sign_in/type'
import { useEffect } from 'react'

export const Route = createFileRoute('/_auth/sign_in')({
  component: SignInPage,
})

function SignInPage() {
  const { session, signIn } = useAuth()
  const navigate = useNavigate()

  useEffect(() => {
    if (session) {
      navigate({ to: '/' })
    }
  }, [session, navigate])

  const handleSignIn = async (data: SignInFormData) => {
    await signIn(data.username, data.password)
  }

  return <SignInForm signInAction={handleSignIn} />
}
