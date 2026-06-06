import { useToast } from '@/components/ui/use-toast'

export const errorMessage = ({
	error,
	// biome-ignore lint/suspicious/noExplicitAny: <explanation>
}: { error: any }): {
	title: string
	description: string
} => {
	const fallbackMessage =
		error instanceof Error && error.message
			? error.message
			: 'Unexpected error occurred'

	// GraphQL client errors include a serialized response payload in the
	// message, but the response shape differs between GraphQL and REST errors.
	if (error.message?.includes('GraphQL Error')) {
		try {
			const jsonStartIndex = error.message.indexOf('{')
			const jsonString = error.message.slice(jsonStartIndex)
			const parsedError = JSON.parse(jsonString)
			const response = parsedError.response ?? {}
			const status = response.status
			const responseMessage =
				response.errors?.[0]?.message ??
				response.message ??
				parsedError.message ??
				fallbackMessage
			console.log('Error Response:', status)
			console.log('Error Request:', parsedError.request)

			if (status === 500) {
				return {
					title: 'Server Error',
					description: 'Please try again later',
				}
			}

			return {
				title: status === 403 ? 'Forbidden' : 'Logic Error',
				description: responseMessage,
			}
		} catch {
			return {
				title: 'Error',
				description: fallbackMessage,
			}
		}
	}

	if (error.message?.includes('Unauthorized')) {
		return {
			title: 'Unauthorized',
			description: 'You are not authorized to access this resource',
		}
	}

	return {
		title: 'Error',
		description:
			fallbackMessage.length > 80
				? `${fallbackMessage.slice(0, 80)}...`
				: fallbackMessage,
	}
}

export const useToastWithError = () => {
	const { toast } = useToast()

	return {
		toast,
		// biome-ignore lint/suspicious/noExplicitAny: <explanation>
		errorToast: (error: any) => {
			const { title, description } = errorMessage({ error })
			toast({
				variant: 'destructive',
				title,
				description,
			})
		},
	}
}
