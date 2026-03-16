export const runtime = 'edge'


import { signOut } from '@/app/(auth)/auth'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { ActionButton } from '@/components/action-button'
import { ToastClient } from '@/components/toast-client'

export default async function SignOutPage({
	searchParams: { error },
}: {
	searchParams: { error?: string }
}) {
	const locale = detectLocale()
	const dictionary = getDictionary(locale)
	const t = dictionary

	return (
		<div className='flex min-h-screen w-full flex-col lg:flex-row'>
			{error === 'expired' && (
				<ToastClient
					title={
						locale === 'ja' ? 'セッションが失効しました' : 'Session expired'
					}
					description={
						locale === 'ja'
							? 'サインアウト後、もう一度サインインしてください'
							: 'Please sign in again after signing out'
					}
					variant='default'
				/>
			)}
			{/* Left sidebar */}
			<div className='flex flex-col justify-between bg-zinc-900 p-6 lg:basis-1/2 lg:p-10'>
				<div className='flex items-center gap-2'>
					<div className='relative h-6 w-6' />
					<div className='text-lg font-bold leading-7 text-white'>
						{t.common.library}
					</div>
				</div>
				<div className='hidden lg:flex lg:flex-col lg:gap-2'>
					<div className='text-lg font-normal leading-7 text-white'>
						"{t.common.tagline}"
					</div>
					<div className='text-sm font-normal leading-tight text-white'>
						{t.common.company}
					</div>
				</div>
			</div>

			{/* Main content */}
			<div className='flex flex-1 flex-col items-center justify-center px-4 py-8 lg:px-8'>
				<div className='mb-6 text-center'>
					<div className='text-2xl font-semibold leading-loose'>
						{t.auth.signOut.title}
					</div>
					<div className='text-sm font-normal leading-tight text-zinc-500'>
						{t.auth.signOut.description}
					</div>
				</div>
				<ActionButton
					action={async () => {
						'use server'
						await signOut({ redirect: true, redirectTo: '/' })
					}}
					className='w-full max-w-[350px]'
				>
					{t.auth.signOut.confirm}
				</ActionButton>
			</div>

			<div className='mt-8 flex flex-col gap-2 p-6 text-left lg:hidden'>
				<div className='text-lg font-normal leading-7'>
					"{t.common.tagline}"
				</div>
				<div className='text-sm font-normal leading-tight'>
					{t.common.company}
				</div>
			</div>
		</div>
	)
}
