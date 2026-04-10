import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/_auth/reset-password')({
  component: ResetPasswordPage,
})

function ResetPasswordPage() {
  return <div className='flex items-center justify-center min-h-screen'>Reset Password - TODO</div>
}
