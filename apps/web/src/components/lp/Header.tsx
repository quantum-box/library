import { Link, useNavigate, useRouterState } from '@tanstack/react-router'
import { Menu, X } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import type { LpLanguage } from '@/app/lp'

const copy = {
	en: {
		tagline: 'Knowledge OS',
		nav: {
			features: 'Features',
			capabilities: 'Capabilities',
			challenges: 'Challenges',
			pricing: 'Pricing',
			roadmap: 'Roadmap',
		},
		login: 'Log in',
		signup: 'Get started',
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
			challenges: '課題と解決',
			pricing: '料金',
			roadmap: 'ロードマップ',
		},
		login: 'ログイン',
		signup: '導入を始める',
		languageNames: {
			en: 'English',
			ja: '日本語',
		},
	},
} satisfies Record<
	LpLanguage,
	{
		tagline: string
		nav: Record<
			'features' | 'capabilities' | 'challenges' | 'pricing' | 'roadmap',
			string
		>
		login: string
		signup: string
		languageNames: Record<LpLanguage, string>
	}
>

const navOrder = [
	'features',
	'capabilities',
	'challenges',
	'pricing',
	'roadmap',
] as const

export function Header({ lang }: { lang: LpLanguage }) {
	const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
	const [scrolled, setScrolled] = useState(false)
	const [activeSection, setActiveSection] = useState<string | null>(null)
	const navigate = useNavigate()
	const pathname = useRouterState({ select: (s) => s.location.pathname })
	const searchParams = useRouterState({
		select: (s) => new URLSearchParams(s.location.search),
	})
	const t = useMemo(() => copy[lang], [lang])

	useEffect(() => {
		const onScroll = () => {
			setScrolled(window.scrollY > 8)
			if (window.scrollY < 200) setActiveSection(null)
		}
		onScroll()
		window.addEventListener('scroll', onScroll, { passive: true })
		return () => window.removeEventListener('scroll', onScroll)
	}, [])

	useEffect(() => {
		const observer = new IntersectionObserver(
			entries => {
				for (const entry of entries) {
					if (entry.isIntersecting) setActiveSection(entry.target.id)
				}
			},
			{ rootMargin: '-35% 0px -60% 0px' },
		)
		for (const key of navOrder) {
			const element = document.getElementById(key)
			if (element) observer.observe(element)
		}
		return () => observer.disconnect()
	}, [])

	const updateLanguage = useCallback(
		(next: LpLanguage) => {
			const params = new URLSearchParams(searchParams?.toString() ?? '')
			params.set('lang', next)
			navigate({ to: `${pathname}?${params.toString()}` })
			setMobileMenuOpen(false)
		},
		[pathname, navigate, searchParams],
	)

	const langOptions: LpLanguage[] = ['ja', 'en']

	return (
		<header
			className={`sticky top-0 z-50 border-b bg-white/90 backdrop-blur transition-colors ${
				scrolled || mobileMenuOpen ? 'border-slate-200' : 'border-transparent'
			}`}
		>
			<div className='mx-auto flex h-14 max-w-6xl items-stretch justify-between gap-6 px-4 sm:px-6 lg:px-8'>
				<Link to='/' className='flex items-center gap-2.5' aria-label='Library home'>
					<img
						src='/brand/library-logo-appbar.png'
						alt='Library'
						className='h-7 w-auto'
					/>
					<span className='hidden items-center gap-2.5 md:flex'>
						<span className='h-4 w-px bg-slate-200' />
						<span className='font-mono text-[10px] uppercase tracking-[0.25em] text-slate-400'>
							{t.tagline}
						</span>
					</span>
				</Link>

				<nav className='hidden items-stretch lg:flex'>
					{navOrder.map((key, index) => {
						const isActive = activeSection === key
						return (
							<a
								key={key}
								href={`#${key}`}
								className='group relative flex items-center gap-1.5 px-3'
							>
								<span
									className={`font-mono text-[10px] transition-colors ${
										isActive ? 'text-blue-700' : 'text-slate-400'
									}`}
								>
									{String(index + 1).padStart(2, '0')}
								</span>
								<span
									className={`text-sm transition-colors ${
										isActive
											? 'text-slate-900'
											: 'text-slate-600 group-hover:text-slate-900'
									}`}
								>
									{t.nav[key]}
								</span>
								<span
									className={`absolute inset-x-3 bottom-0 h-[2px] transition-colors ${
										isActive
											? 'bg-blue-700'
											: 'bg-transparent group-hover:bg-slate-200'
									}`}
								/>
							</a>
						)
					})}
				</nav>

				<div className='hidden items-center gap-4 lg:flex'>
					<div className='flex items-center gap-1.5 font-mono text-xs'>
						{langOptions.map((option, index) => (
							<span key={option} className='flex items-center gap-1.5'>
								{index > 0 && <span className='text-slate-300'>/</span>}
								<button
									type='button'
									className={`transition-colors ${
										lang === option
											? 'text-slate-900'
											: 'text-slate-400 hover:text-slate-600'
									}`}
									onClick={() => updateLanguage(option)}
								>
									{option.toUpperCase()}
								</button>
							</span>
						))}
					</div>
					<span className='h-4 w-px bg-slate-200' />
					<Link
						to='/sign_in'
						className='text-sm text-slate-600 transition-colors hover:text-slate-900'
					>
						{t.login}
					</Link>
					<Link
						to='/sign_up'
						className='rounded-md bg-slate-900 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-slate-700'
					>
						{t.signup}
					</Link>
				</div>

				<button
					type='button'
					className='inline-flex items-center self-center rounded-md p-2 text-slate-700 transition hover:bg-slate-50 lg:hidden'
					onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
				>
					{mobileMenuOpen ? (
						<X className='h-5 w-5' />
					) : (
						<Menu className='h-5 w-5' />
					)}
				</button>
			</div>

			{mobileMenuOpen && (
				<div className='border-t border-slate-200 bg-white px-4 pb-6 pt-3 lg:hidden'>
					<nav className='flex flex-col'>
						{navOrder.map((key, index) => (
							<button
								key={key}
								type='button'
								className='flex items-baseline gap-3 rounded-md px-3 py-2.5 text-left transition hover:bg-slate-50'
								onClick={() => {
									setMobileMenuOpen(false)
									document
										.getElementById(key)
										?.scrollIntoView({ behavior: 'smooth' })
								}}
							>
								<span className='font-mono text-[10px] text-blue-700'>
									{String(index + 1).padStart(2, '0')}
								</span>
								<span className='text-sm text-slate-700'>{t.nav[key]}</span>
							</button>
						))}
						<div className='mt-3 flex items-center gap-2 border-t border-slate-200 pt-4'>
							{langOptions.map(option => (
								<button
									key={option}
									type='button'
									className={`flex-1 rounded-md border px-3 py-2 text-sm transition ${
										lang === option
											? 'border-slate-900 bg-slate-900 text-white'
											: 'border-slate-200 text-slate-600 hover:bg-slate-50'
									}`}
									onClick={() => updateLanguage(option)}
								>
									{t.languageNames[option]}
								</button>
							))}
						</div>
						<Link
							to='/sign_in'
							className='mt-2 rounded-md border border-slate-200 px-3 py-2 text-center text-sm font-medium text-slate-700 transition hover:bg-slate-50'
						>
							{t.login}
						</Link>
						<Link
							to='/sign_up'
							className='mt-2 rounded-md bg-slate-900 px-3 py-2 text-center text-sm font-medium text-white transition hover:bg-slate-700'
						>
							{t.signup}
						</Link>
					</nav>
				</div>
			)}
		</header>
	)
}
