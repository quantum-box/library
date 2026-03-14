'use client'

import { LanguageSwitcher } from '@/components/language-switcher'
import { useTranslation } from '@/lib/i18n/useTranslation'
import type { ReactNode } from 'react'

interface AuthLayoutProps {
	/** Page title displayed in the header */
	title: string
	/** Description text below the title */
	description: string
	/** Main content (form, etc.) */
	children: ReactNode
	/** Optional footer content below the form */
	footer?: ReactNode
}

/**
 * Shared layout component for authentication pages.
 * Provides consistent styling with sidebar and responsive design.
 */
export function AuthLayout({
	title,
	description,
	children,
	footer,
}: AuthLayoutProps) {
	const { t } = useTranslation()

	return (
		<div className='flex min-h-screen w-full flex-col lg:flex-row'>
			{/* Left sidebar */}
			<div className='flex flex-col justify-between bg-zinc-900 p-6 lg:basis-1/2 lg:p-10'>
				<div className='flex items-center justify-between'>
					<div className='flex items-center gap-2'>
						<div className='relative h-6 w-6' />
						<div className='text-lg font-bold leading-7 text-white'>
							{t.common.library}
						</div>
					</div>
					<div className='lg:hidden'>
						<LanguageSwitcher variant='ghost' />
					</div>
				</div>
				<div className='hidden lg:flex lg:flex-col lg:gap-2'>
					<div className='text-lg font-normal leading-7 text-white'>
						"{t.common.tagline}"
					</div>
					<div className='flex items-center justify-between'>
						<div className='text-sm font-normal leading-tight text-white'>
							{t.common.company}
						</div>
						<LanguageSwitcher variant='ghost' />
					</div>
				</div>
			</div>

			{/* Main content */}
			<div className='flex flex-1 flex-col items-center justify-center px-4 py-8 lg:px-8'>
				<div className='mb-6 text-center'>
					<h1 className='text-2xl font-semibold leading-loose'>{title}</h1>
					<p className='text-sm font-normal leading-tight text-zinc-500'>
						{description}
					</p>
				</div>

				{children}

				{footer}
			</div>

			{/* Mobile footer quote */}
			<div className='mt-8 flex flex-col gap-2 text-left p-6 lg:hidden'>
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
