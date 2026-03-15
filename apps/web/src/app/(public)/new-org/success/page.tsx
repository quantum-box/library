
import { signIn } from '@/app/(auth)/auth'
import { ActionButton } from '@/components/action-button'
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'

export default function Page() {
	return (
		<Card className='w-full max-w-xl mx-auto my-8 p-6 bg-white shadow-md'>
			<CardHeader>
				<CardTitle>Create a new organization and project</CardTitle>
				<CardDescription>
					A project contains all project files, including the revision history.
					Already have a project project
				</CardDescription>
			</CardHeader>
			<CardContent>
				<p className='text-center text-gray-600'>
					Registration successful. Please continue to create your project.
				</p>
			</CardContent>
			<CardFooter className='flex justify-end'>
				<ActionButton
					action={async () => {
						'use server'
						await signIn('keycloak', { callbackUrl: '/' })
					}}
				>
					Sign in again
				</ActionButton>
			</CardFooter>
		</Card>
	)
}
