import { auth } from '@/app/(auth)/auth'
import { redirect } from 'next/navigation'
import { signUp } from './action'
import { SignUpForm } from './form'


export default async function SignUpPage() {
	const session = await auth()
	if (session) {
		redirect('/')
	}
	return <SignUpForm signUpAction={signUp} />
}
