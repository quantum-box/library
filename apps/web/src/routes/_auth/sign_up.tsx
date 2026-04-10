import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/_auth/sign_up')({
  component: SignUpPage,
})

function SignUpPage() {
  return <div className='flex items-center justify-center min-h-screen'>Sign Up - TODO</div>
}
