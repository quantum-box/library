import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/_auth/forgot-password')({
  component: ForgotPasswordPage,
})

function ForgotPasswordPage() {
  return <div className='flex items-center justify-center min-h-screen'>Forgot Password - TODO</div>
}
