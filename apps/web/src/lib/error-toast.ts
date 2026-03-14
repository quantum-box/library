import { useToast } from '@/components/ui/use-toast'

export const errorMessage = ({
	error,
	// biome-ignore lint/suspicious/noExplicitAny: <explanation>
}: { error: any }): {
	title: string
	description: string
} => {
	// GraphQLエラーの場合
	if (error.message?.includes('GraphQL Error')) {
		const jsonStartIndex = error.message.indexOf('{')
		const jsonString = error.message.slice(jsonStartIndex)
		const parsedError = JSON.parse(jsonString)
		console.log('Error Response:', parsedError.response.status)
		console.log('Error Request:', parsedError.request)

		if (parsedError.response.status === 500) {
			return {
				title: 'Server Error',
				description: 'Please try again later',
			}
		}

		return {
			title: 'Logic Error',
			description: parsedError.response.errors[0].message,
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
			error.message.length > 20
				? `${error.message.slice(0, 20)}...`
				: error.message,
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
