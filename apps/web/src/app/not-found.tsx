import { Button } from '@/components/ui/button'
import { Link } from '@tanstack/react-router'

export default function NotFound() {
	return (
		<div className='flex flex-col items-center justify-center min-h-screen bg-background text-foreground'>
			<h1 className='text-4xl font-bold mb-4'>404 - Page Not Found</h1>
			<p className='text-lg mb-8'>
				The page you're looking for doesn't exist or has been moved.
			</p>
			<Button asChild>
				<Link to='/'>Return to Home</Link>
			</Button>
		</div>
	)
}
