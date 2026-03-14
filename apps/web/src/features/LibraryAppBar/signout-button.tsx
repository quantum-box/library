import { signOut } from '@/app/(auth)/auth'
import { ActionButton } from '@/components/action-button'
import { redirect } from 'next/navigation'

export function SignOutButton() {
	const handleSignOut = async () => {
		'use server'
		await signOut()
		redirect('/')
	}
	return <ActionButton action={handleSignOut}>Sign out</ActionButton>
}
