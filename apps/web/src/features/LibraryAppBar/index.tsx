import { auth } from '@/app/(auth)/auth'
import { Button } from '@/components/ui/button'
import Link from 'next/link'
import { AccountButton } from './account-button'

export async function AppBar({
	onDrawer,
	onNotification,
	onAdd,
	onTodo,
	children,
}: {
	onDrawer?: () => void
	onNotification?: () => void
	onAdd?: () => void
	onTodo?: () => void
	children?: React.ReactNode
}) {
	const session = await auth()
	const user = session?.user
	return (
		<header className='sticky top-0 z-50 border-b border-border/60 bg-white/90 backdrop-blur shadow-sm'>
			<div className='mx-auto flex h-14 w-full max-w-6xl items-center justify-between px-4 text-sm font-medium text-foreground'>
				<div className='flex items-center gap-3'>
					{/* {onDrawer && (
						<NavigationButton onClick={onDrawer}>
							<HamburgerMenuIcon className=' w-5 h-5 relative' />
						</NavigationButton>
					)} */}
					<Link href='/' className='flex items-center gap-2'>
						<BookIcon className='h-5 w-5 text-primary' />
						<span className='text-base font-semibold'>Library</span>
					</Link>

					{children}
				</div>
				<div className='flex items-center gap-3'>
					{!user && (
						<span className='hidden rounded-full bg-slate-100 px-3 py-1 text-xs font-semibold text-slate-700 sm:inline'>
							View only
						</span>
					)}
					{user ? (
						<div className='flex h-9 items-center gap-2'>
							{/* {onAdd && (
								<NavigationButton onClick={onAdd}>
									<PlusIcon className='w-5 h-5 relative' />
							</NavigationButton>
						)}
						{onTodo && (
							<NavigationButton onClick={onTodo}>
								<CheckboxIcon className='w-5 h-5 relative' />
							</NavigationButton>
						)}
						{onNotification && (
							<NavigationButton onClick={onNotification}>
								<BellIcon className='w-5 h-5 relative' />
							</NavigationButton>
						)} */}
							<AccountButton account={user} />
						</div>
					) : (
						<Button size='sm' className='h-9 px-4 font-semibold' asChild>
							<Link href='/sign_in'>Sign in</Link>
						</Button>
					)}
				</div>
			</div>
		</header>
	)
}
function MountainIcon(props: React.SVGProps<SVGSVGElement>) {
	return (
		<svg
			{...props}
			xmlns='http://www.w3.org/2000/svg'
			width='24'
			height='24'
			viewBox='0 0 24 24'
			fill='none'
			stroke='currentColor'
			strokeWidth='2'
			strokeLinecap='round'
			strokeLinejoin='round'
		>
			<path d='m8 3 4 8 5-5 5 15H2L8 3z' />
		</svg>
	)
}

const BookIcon = (props: React.SVGProps<SVGSVGElement>) => (
	<svg
		xmlns='http://www.w3.org/2000/svg'
		width={682.667}
		height={682.667}
		viewBox='0 0 512 512'
		{...props}
	>
		<path d='M192 224v160h64l.1-138.3V107.5l42.5 138c23.4 75.9 42.7 138.1 42.8 138.2.3.3 62.7-20.5 63.3-21.1.4-.4-35.3-117.1-82.8-271.3l-2-6.2L288 95.9l-32 10.9V64h-64v160zM107 245.5V384h64V107h-64v138.5zM64 426.5V448h384v-43H64v21.5z' />
	</svg>
)
