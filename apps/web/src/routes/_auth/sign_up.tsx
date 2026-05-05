import { signUpWithCredentials } from '@/auth'
import { SignUpForm } from '@/app/(auth)/sign_up/form'
import type { SignUpFormData } from '@/app/(auth)/sign_up/type'
import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/_auth/sign_up')({
	component: SignUpPage,
})

function SignUpPage() {
	const handleSignUp = async (data: SignUpFormData) => {
		await signUpWithCredentials(data.username, data.email, data.password)
	}

	return <SignUpForm signUpAction={handleSignUp} />
}
