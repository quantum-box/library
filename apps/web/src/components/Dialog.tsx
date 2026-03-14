import { Cross1Icon } from '@radix-ui/react-icons'
import clsx from 'clsx'
import Link from 'next/link'

export function Dialog({
	isOpen,
	children,
	className = '',
}: {
	isOpen: boolean
	children: React.ReactNode
	className?: string
}) {
	return (
		<>
			{isOpen && (
				<div className='fixed inset-0 flex items-center justify-center z-10'>
					<div className='bg-black opacity-50 absolute inset-0' />
					<div
						className={clsx('bg-white p-8 rounded relative z-20', className)}
					>
						{children}
					</div>
				</div>
			)}
		</>
	)
}

export function DialogCloseIcon({ href }: { href: string }) {
	return (
		<Link href={href}>
			{/* biome-ignore lint/a11y/useButtonType: <explanation> */}
			<button className='absolute top-1 right-1 p-2 hover:bg-gray-200 rounded'>
				<Cross1Icon />
			</button>
		</Link>
	)
}
