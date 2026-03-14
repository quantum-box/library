'use client'

export default function ErrorPage({
	error,
	reset,
}: {
	error: Error
	reset: () => void
}) {
	return (
		<div className='container mx-auto py-6'>
			<div className='rounded-md bg-red-50 p-4'>
				<div className='flex'>
					<div className='ml-3'>
						<h3 className='text-sm font-medium text-red-800'>
							エラーが発生しました
						</h3>
						<div className='mt-2 text-sm text-red-700'>
							<p>{error.message}</p>
						</div>
						<div className='mt-4'>
							<button
								type='button'
								onClick={reset}
								className='rounded-md bg-red-50 px-2 py-1.5 text-sm font-medium text-red-800 hover:bg-red-100 focus:outline-none focus:ring-2 focus:ring-red-600 focus:ring-offset-2'
							>
								再試行
							</button>
						</div>
					</div>
				</div>
			</div>
		</div>
	)
}
