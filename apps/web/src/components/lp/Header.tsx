'use client'

import { loginAction } from '@/actions/auth'
import { ActionButton } from '@/components/action-button'
import { ArrowRight, Globe, Menu, X } from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import type { LpLanguage } from '@/app/lp'

const copy = {
	en: {
		tagline: 'Knowledge OS',
		nav: {
			features: 'Features',
			capabilities: 'Capabilities',
			pricing: 'Pricing',
			roadmap: 'Roadmap',
		},
		login: 'Log in',
		language: 'Language',
		languageNames: {
			en: 'English',
			ja: '日本語',
		},
	},
	ja: {
		tagline: 'ナレッジOS',
		nav: {
			features: '特徴',
			capabilities: '提供価値',
			pricing: '料金',
			roadmap: 'ロードマップ',
		},
		login: 'ログイン',
		language: '言語',
		languageNames: {
			en: 'English',
			ja: '日本語',
		},
	},
} satisfies Record<
	LpLanguage,
	{
		tagline: string
		nav: Record<'features' | 'capabilities' | 'pricing' | 'roadmap', string>
		login: string
		language: string
		languageNames: Record<LpLanguage, string>
	}
>

export function Header({ lang }: { lang: LpLanguage }) {
	const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
	const router = useRouter()
	const pathname = usePathname()
	const searchParams = useSearchParams()
	const t = useMemo(() => copy[lang], [lang])

	const toggleMobileMenu = () => {
		setMobileMenuOpen(!mobileMenuOpen)
	}

	const updateLanguage = useCallback(
		(next: LpLanguage) => {
			const params = new URLSearchParams(searchParams?.toString() ?? '')
			if (next === 'ja') {
				params.delete('lang')
			} else {
				params.set('lang', next)
			}
			const query = params.toString()
			router.push(query ? `${pathname}?${query}` : pathname, { scroll: false })
			setMobileMenuOpen(false)
		},
		[pathname, router, searchParams],
	)

	const langOptions: LpLanguage[] = ['ja', 'en']

	return (
		<header className='sticky top-0 z-50 border-b border-white/5 bg-slate-950/75 backdrop-blur-lg'>
			<div className='container mx-auto flex items-center justify-between gap-4 px-4 py-3 sm:px-6'>
				<div className='flex w-full items-center justify-between sm:w-auto'>
					<div className='flex items-center gap-3'>
						<div className='relative flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-cyan-400/90 via-sky-500/90 to-blue-600/90 shadow-[0_12px_40px_rgba(56,189,248,0.22)]'>
							<svg
								xmlns='http://www.w3.org/2000/svg'
								viewBox='0 0 24 24'
								fill='none'
								stroke='currentColor'
								strokeWidth='2'
								strokeLinecap='round'
								strokeLinejoin='round'
								className='h-5 w-5 text-white'
							>
								<path d='M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20' />
							</svg>
						</div>
						<div className='flex flex-col leading-tight'>
							<span className='text-lg font-semibold text-white sm:text-2xl'>
								Library
							</span>
							<span className='text-xs font-medium uppercase tracking-widest text-sky-300/80 sm:text-[0.7rem]'>
								{t.tagline}
							</span>
						</div>
					</div>
					<button
						type='button'
						className='inline-flex items-center justify-center rounded-full border border-white/10 bg-white/5 p-2 text-white transition hover:bg-white/10 sm:hidden'
						onClick={toggleMobileMenu}
					>
						{mobileMenuOpen ? (
							<X className='h-5 w-5' />
						) : (
							<Menu className='h-5 w-5' />
						)}
					</button>
				</div>

				<nav className='hidden items-center gap-2 sm:flex'>
					<a
						href='#features'
						className='rounded-full px-4 py-2 text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
					>
						{t.nav.features}
					</a>
					<a
						href='#capabilities'
						className='rounded-full px-4 py-2 text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
					>
						{t.nav.capabilities}
					</a>
					<a
						href='#pricing'
						className='rounded-full px-4 py-2 text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
					>
						{t.nav.pricing}
					</a>
					<a
						href='#roadmap'
						className='rounded-full px-4 py-2 text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
					>
						{t.nav.roadmap}
					</a>
					<div className='flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold uppercase tracking-widest text-slate-200'>
						<Globe className='h-4 w-4 text-sky-200' />
						<span>{t.language}</span>
						<div className='flex gap-1 rounded-full bg-slate-900/60 p-1'>
							{langOptions.map(option => (
								<button
									key={option}
									type='button'
									className={`rounded-full px-2 py-1 text-[0.7rem] font-semibold transition ${
										lang === option
											? 'bg-white text-slate-900'
											: 'text-slate-200 hover:bg-white/10'
									}`}
									onClick={() => updateLanguage(option)}
								>
									{option.toUpperCase()}
								</button>
							))}
						</div>
					</div>
					<ActionButton
						className='inline-flex items-center justify-center gap-2 rounded-full border border-sky-400/50 bg-gradient-to-r from-sky-500/80 to-blue-600/80 px-5 py-2.5 text-sm font-semibold text-white shadow-[0_15px_40px_rgba(56,189,248,0.3)] transition hover:shadow-[0_18px_45px_rgba(56,189,248,0.36)]'
						action={loginAction}
					>
						{t.login}
						<ArrowRight className='h-4 w-4' />
					</ActionButton>
				</nav>
			</div>

			{mobileMenuOpen && (
				<div className='border-t border-white/5 bg-slate-950/95 px-4 pb-6 pt-4 backdrop-blur sm:hidden'>
					<nav className='flex flex-col gap-2'>
						<button
							type='button'
							className='rounded-lg border border-white/5 bg-white/5 px-4 py-2 text-left text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
							onClick={() => {
								toggleMobileMenu()
								document
									.getElementById('features')
									?.scrollIntoView({ behavior: 'smooth' })
							}}
						>
							{t.nav.features}
						</button>
						<button
							type='button'
							className='rounded-lg border border-white/5 bg-white/5 px-4 py-2 text-left text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
							onClick={() => {
								toggleMobileMenu()
								document
									.getElementById('capabilities')
									?.scrollIntoView({ behavior: 'smooth' })
							}}
						>
							{t.nav.capabilities}
						</button>
						<button
							type='button'
							className='rounded-lg border border-white/5 bg-white/5 px-4 py-2 text-left text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
							onClick={() => {
								toggleMobileMenu()
								document
									.getElementById('pricing')
									?.scrollIntoView({ behavior: 'smooth' })
							}}
						>
							{t.nav.pricing}
						</button>
						<button
							type='button'
							className='rounded-lg border border-white/5 bg-white/5 px-4 py-2 text-left text-sm font-medium text-slate-200 transition hover:bg-white/10 hover:text-white'
							onClick={() => {
								toggleMobileMenu()
								document
									.getElementById('roadmap')
									?.scrollIntoView({ behavior: 'smooth' })
							}}
						>
							{t.nav.roadmap}
						</button>
						<div className='mt-1 flex flex-col gap-2 rounded-lg border border-white/5 bg-white/5 p-3 text-slate-200'>
							<div className='flex items-center gap-2 text-xs font-semibold uppercase tracking-widest'>
								<Globe className='h-4 w-4 text-sky-200' />
								<span>{t.language}</span>
							</div>
							{langOptions.map(option => (
								<button
									key={option}
									type='button'
									className={`flex items-center justify-between rounded-md px-3 py-2 text-sm transition ${
										lang === option
											? 'bg-white text-slate-900'
											: 'hover:bg-white/10'
									}`}
									onClick={() => updateLanguage(option)}
								>
									<span>{t.languageNames[option]}</span>
									<span className='text-xs text-slate-400'>
										{option.toUpperCase()}
									</span>
								</button>
							))}
						</div>
						<ActionButton
							className='mt-2 inline-flex items-center justify-center gap-2 rounded-full border border-sky-400/50 bg-gradient-to-r from-sky-500/80 to-blue-600/80 px-4 py-2 text-sm font-semibold text-white shadow-[0_15px_40px_rgba(56,189,248,0.3)] transition hover:shadow-[0_18px_45px_rgba(56,189,248,0.36)]'
							action={loginAction}
						>
							{t.login}
							<ArrowRight className='h-4 w-4' />
						</ActionButton>
					</nav>
				</div>
			)}
		</header>
	)
}
