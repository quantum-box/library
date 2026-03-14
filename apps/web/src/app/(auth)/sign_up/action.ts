'use server'

import 'server-only'
import { signUpWithCognito } from './cognito-actions'
import { SignUpFormData } from './type'

export async function signUp(data: SignUpFormData) {
	await signUpWithCognito({
		username: data.username,
		email: data.email,
		password: data.password,
	})
}
