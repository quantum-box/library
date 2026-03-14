'use client'

import { useEffect } from 'react'

export default function ErrorTestPage({
	error,
	reset,
}: {
	error: Error & { digest?: string }
	reset: () => void
}) {
	useEffect(() => {
		// エラーをログに記録
		console.error('Error:', error)
	}, [error])

	return (
		<div className='flex min-h-screen flex-col items-center justify-center'>
			<div className='text-center'>
				<h2 className='text-2xl font-bold mb-4'>Error Occurred</h2>
				<p className='text-gray-600 mb-4'>{error.message || 'Unknown error'}</p>
				<button
					type='button'
					onClick={reset}
					className='px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors'
				>
					Try Again
				</button>
			</div>
		</div>
	)
}
