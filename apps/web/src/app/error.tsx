'use client'
import { ActionButton } from '@/components/action-button'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/use-toast'
import { useRouter } from 'next/navigation'
import { useEffect } from 'react'

export default function ErrorPage({
	error,
	reset,
}: {
	error: Error
	reset: () => void
}) {
	const router = useRouter()
	const { toast } = useToast()

	if (error.message?.includes('GraphQL Error')) {
		const jsonStartIndex = error.message.indexOf('{')
		const jsonString = error.message.slice(jsonStartIndex)
		const parsedError = JSON.parse(jsonString)

		if (parsedError.response.status === 401) {
			toast({
				variant: 'destructive',
				title: 'Session expired',
				description: 'Please sign in again',
			})
			router.replace('/sign_out')
		}
	}

	useEffect(() => {
		// Log the error to an error reporting service
		errorMessage({ error })
	}, [error])

	const onRedirect = () => {
		router.replace('/sign_out')
	}

	return (
		<div className='flex flex-col p-4 min-h-screen  '>
			<Alert variant='destructive'>
				<AlertTitle>500 - Server Error</AlertTitle>
				<AlertDescription>
					We're sorry, but something went wrong on our end.
				</AlertDescription>
			</Alert>
			<div className='flex gap-4 mt-8'>
				<Button onClick={() => reset()}>Try Again</Button>
				<ActionButton action={onRedirect}>Sign out</ActionButton>
			</div>
		</div>
	)
}

// biome-ignore lint/suspicious/noExplicitAny: <explanation>
const errorMessage = ({ error }: { error: any }) => {
	// GraphQLエラーの場合
	if (error.message?.includes('GraphQL Error')) {
		const jsonStartIndex = error.message.indexOf('{')
		const jsonString = error.message.slice(jsonStartIndex)
		const parsedError = JSON.parse(jsonString)
		console.log('Error Response:', parsedError.response.status)
		console.log('Error Request:', parsedError.request)
	}
}
