import type { LpLanguage } from '@/app/lp'
import { LIBRARY_APP_URL } from './links'
import { ArrowRight, ExternalLink } from 'lucide-react'
import { Link } from '@tanstack/react-router'
import { PlanetMark } from './PlanetMark'

type FooterLink = {
	label: string
	href: string
}

type FooterContent = {
	primaryHeading: string
	primaryBody: string
	primaryCta: string
	secondaryCta: string
	columnLibraryTitle: string
	columnQuantumTitle: string
	chapters: FooterLink[]
	quantumLinks: FooterLink[]
	brandTagline: string
	summaryLine: string
	footerLinks: FooterLink[]
}

const copy: Record<LpLanguage, FooterContent> = {
	en: {
		primaryHeading: 'Bring humanity’s knowledge to everyone',
		primaryBody:
			'Library is built by Quantum Box, Inc. to make trusted knowledge programmable. We pair public infrastructure with sustainable operations.',
		primaryCta: 'Start with Library',
		secondaryCta: 'Talk to Quantum Box',
		columnLibraryTitle: 'Contents',
		columnQuantumTitle: 'Quantum Box',
		chapters: [
			{ label: 'Features', href: '#features' },
			{ label: 'Capabilities', href: '#capabilities' },
			{ label: 'Challenges', href: '#challenges' },
			{ label: 'Pricing', href: '#pricing' },
			{ label: 'Roadmap', href: '#roadmap' },
		],
		quantumLinks: [
			{ label: 'Corporate site', href: 'https://www.quantum-box.com/' },
			{ label: 'About us', href: 'https://www.quantum-box.com/about' },
			{ label: 'Services', href: 'https://www.quantum-box.com/services' },
			{ label: 'Products', href: 'https://www.quantum-box.com/products' },
			{ label: 'Contact', href: 'https://www.quantum-box.com/contact' },
		],
		brandTagline: 'Technology for everyone.',
		summaryLine:
			'We open information technology as a shared infrastructure for everyone.',
		footerLinks: [
			{ label: 'Terms', href: '/terms' },
			{ label: 'Privacy', href: '/privacy' },
			{ label: 'Security', href: '/security' },
		],
	},
	ja: {
		primaryHeading: '人類の知を、すべての人へ',
		primaryBody:
			'Library は Quantum Box, Inc. が提供するナレッジオペレーティングシステムです。公共性と持続性を両立させながら、信頼できる知識をプログラム可能にします。',
		primaryCta: '導入を始める',
		secondaryCta: 'Quantum Box に相談する',
		columnLibraryTitle: '目次',
		columnQuantumTitle: 'Quantum Box',
		chapters: [
			{ label: '特徴', href: '#features' },
			{ label: '提供価値', href: '#capabilities' },
			{ label: '課題と解決', href: '#challenges' },
			{ label: '料金', href: '#pricing' },
			{ label: 'ロードマップ', href: '#roadmap' },
		],
		quantumLinks: [
			{ label: '企業サイト', href: 'https://www.quantum-box.com/' },
			{ label: '私たちについて', href: 'https://www.quantum-box.com/about' },
			{ label: 'サービス', href: 'https://www.quantum-box.com/services' },
			{ label: 'プロダクト', href: 'https://www.quantum-box.com/products' },
			{ label: 'お問い合わせ', href: 'https://www.quantum-box.com/contact' },
		],
		brandTagline: 'テクノロジーを、みんなのものに。',
		summaryLine:
			'情報技術を公共財としてひらき、誰もが活用できる知識インフラを届けます。',
		footerLinks: [
			{ label: '利用規約', href: '/terms' },
			{ label: 'プライバシー', href: '/privacy' },
			{ label: 'セキュリティ', href: '/security' },
		],
	},
}

export function Footer({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]

	return (
		<>
			<section className='mx-auto max-w-6xl px-4 pb-24 pt-4 sm:px-6 lg:px-8'>
				<div className='relative overflow-hidden rounded-lg bg-slate-900 px-6 pb-28 pt-16 text-center sm:px-12 sm:pb-32 sm:pt-20'>
					{/* Engraved earth rising over the horizon, echoing fig. 1 in the hero */}
					<svg
						viewBox='0 0 1200 320'
						preserveAspectRatio='xMidYMax slice'
						aria-hidden='true'
						className='pointer-events-none absolute inset-x-0 bottom-0 h-52 w-full'
						fill='none'
					>
						<circle cx='600' cy='680' r='430' stroke='white' strokeOpacity='0.14' />
						<circle cx='600' cy='680' r='505' stroke='white' strokeOpacity='0.09' />
						<circle cx='600' cy='680' r='585' stroke='white' strokeOpacity='0.05' />
						<ellipse cx='600' cy='680' rx='180' ry='430' stroke='white' strokeOpacity='0.08' />
						<ellipse cx='600' cy='680' rx='330' ry='430' stroke='white' strokeOpacity='0.06' />
						{/* Rotating meridian: rx sweep = 2D projection of a turning globe */}
						<ellipse
							cx='600'
							cy='680'
							ry='430'
							stroke='white'
							strokeOpacity='0.09'
							className='motion-reduce:hidden'
						>
							<animate
								attributeName='rx'
								values='16;420;16'
								dur='36s'
								repeatCount='indefinite'
							/>
						</ellipse>
						<path
							d='M450 277 Q 615 205 780 290'
							stroke='#60a5fa'
							strokeOpacity='0.5'
						/>
						<circle cx='450' cy='277' r='3' fill='#60a5fa' fillOpacity='0.8' />
						<circle cx='780' cy='290' r='3' fill='#60a5fa' fillOpacity='0.8' />
						<circle cx='600' cy='252' r='2' fill='#60a5fa' fillOpacity='0.5' />
						{/* Knowledge pulses travelling along the route */}
						<circle
							r='2.5'
							fill='#60a5fa'
							fillOpacity='0.9'
							className='motion-reduce:hidden'
						>
							<animateMotion
								dur='7s'
								repeatCount='indefinite'
								path='M450 277 Q 615 205 780 290'
							/>
						</circle>
						<circle
							r='2.5'
							fill='#60a5fa'
							fillOpacity='0.55'
							className='motion-reduce:hidden'
						>
							<animateMotion
								dur='7s'
								begin='-3.5s'
								repeatCount='indefinite'
								path='M780 290 Q 615 205 450 277'
							/>
						</circle>
					</svg>
					<div className='pointer-events-none absolute inset-3 rounded-md border border-white/10' />
					<div className='relative'>
						<p className='font-mono text-xs uppercase tracking-[0.25em] text-slate-400'>
							Quantum Box, Inc.
						</p>
						<h2 className='mx-auto mt-5 max-w-2xl font-display text-3xl leading-tight text-white sm:text-4xl'>
							{t.primaryHeading}
						</h2>
						<p className='mx-auto mt-5 max-w-xl text-sm leading-relaxed text-slate-400 sm:text-base'>
							{t.primaryBody}
						</p>
						<div className='mt-9 flex flex-col items-center justify-center gap-3 sm:flex-row'>
							<a
								href={LIBRARY_APP_URL}
								className='inline-flex items-center justify-center gap-2 rounded-md bg-white px-6 py-2.5 text-sm font-medium text-slate-900 transition hover:bg-slate-200'
							>
								{t.primaryCta}
								<ArrowRight className='h-4 w-4' />
							</a>
							<a
								href='https://www.quantum-box.com/contact'
								target='_blank'
								rel='noreferrer'
								className='inline-flex items-center justify-center gap-2 rounded-md px-6 py-2.5 text-sm font-medium text-slate-300 transition hover:text-white'
							>
								{t.secondaryCta}
								<ExternalLink className='h-3.5 w-3.5' />
							</a>
						</div>
					</div>
				</div>
			</section>

			<footer className='border-t border-slate-200 bg-white'>
				<div className='mx-auto grid max-w-6xl gap-10 px-4 py-14 sm:grid-cols-2 sm:px-6 lg:grid-cols-[1.3fr_0.85fr_0.85fr] lg:px-8'>
					<div>
						<div className='flex items-center gap-2.5'>
							<span className='flex h-7 w-7 items-center justify-center rounded-md bg-slate-900 text-white'>
								<PlanetMark className='h-[18px] w-[18px]' />
							</span>
							<span className='font-display text-xl leading-none text-slate-900'>
								Library
							</span>
							<span className='flex items-center gap-2.5'>
								<span className='h-4 w-px bg-slate-200' />
								<span className='font-mono text-[10px] uppercase tracking-[0.25em] text-slate-400'>
									{t.brandTagline}
								</span>
							</span>
						</div>
						<p className='mt-5 max-w-xs text-sm leading-relaxed text-slate-500'>
							{t.summaryLine}
						</p>
					</div>

					<nav aria-label={t.columnLibraryTitle}>
						<p className='font-mono text-[10px] uppercase tracking-[0.25em] text-slate-400'>
							{t.columnLibraryTitle}
						</p>
						<ol className='mt-4 space-y-2.5'>
							{t.chapters.map((link, index) => (
								<li key={link.href}>
									<a
										href={link.href}
										className='group inline-flex items-baseline gap-2.5'
									>
										<span className='font-mono text-[10px] text-slate-400 transition-colors group-hover:text-blue-700'>
											{String(index + 1).padStart(2, '0')}
										</span>
										<span className='text-sm text-slate-600 transition-colors group-hover:text-slate-900'>
											{link.label}
										</span>
									</a>
								</li>
							))}
						</ol>
					</nav>

					<nav aria-label={t.columnQuantumTitle}>
						<p className='font-mono text-[10px] uppercase tracking-[0.25em] text-slate-400'>
							{t.columnQuantumTitle}
						</p>
						<ul className='mt-4 space-y-2.5'>
							{t.quantumLinks.map(link => (
								<li key={link.href}>
									<a
										href={link.href}
										target='_blank'
										rel='noreferrer'
										className='group inline-flex items-center gap-1.5 text-sm text-slate-600 transition-colors hover:text-slate-900'
									>
										{link.label}
										<ExternalLink className='h-3 w-3 text-slate-300 transition-colors group-hover:text-slate-400' />
									</a>
								</li>
							))}
						</ul>
					</nav>
				</div>

				<div className='border-t border-slate-200'>
					<div className='mx-auto flex max-w-6xl flex-col gap-3 px-4 py-5 font-mono text-[11px] text-slate-400 sm:flex-row sm:items-center sm:justify-between sm:px-6 lg:px-8'>
						<p>© {new Date().getFullYear()} Quantum Box, Inc.</p>
						<div className='flex flex-wrap items-center gap-x-5 gap-y-2'>
							{t.footerLinks.map(link => (
								<Link
									key={link.href}
									to={link.href}
									className='transition-colors hover:text-slate-600'
								>
									{link.label}
								</Link>
							))}
						</div>
					</div>
				</div>
			</footer>
		</>
	)
}
