import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'

export const runtime = 'edge'

export default function RepoNotFound() {
	return (
		<div className='flex flex-col min-h-screen p-4 container'>
			<Alert variant='destructive'>
				<AlertTitle>Repository Not Found</AlertTitle>
				<AlertDescription>
					The repository you are looking for does not exist or has been deleted.
				</AlertDescription>
			</Alert>
		</div>
	)
}
