export const runtime = 'edge'

import { auth, signIn } from '@/app/(auth)/auth'
import { redirect } from 'next/navigation'
import { SignInForm } from './form'


export default async function SignInPage() {
	const session = await auth()
	if (session) {
		redirect('/')
	}
	return (
		<SignInForm
			signInAction={async data => {
				'use server'
				await signIn('credentials', {
					username: data.username,
					password: data.password,
					callbackUrl: '/',
				})
			}}
		/>
	)
}
