import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { useEffect } from 'react'

export const Route = createFileRoute('/_auth/sign_out')({
  component: SignOutPage,
})

function SignOutPage() {
  const { signOut } = useAuth()
  const navigate = useNavigate()

  useEffect(() => {
    signOut()
    navigate({ to: '/sign_in' })
  }, [signOut, navigate])

  return (
    <div className='flex items-center justify-center min-h-screen'>
      Signing out...
    </div>
  )
}
