import { Button } from '@/components/ui/button'
import type { LpLanguage } from '@/app/lp'
import { ArrowRight, ExternalLink } from 'lucide-react'
import Link from 'next/link'

type FooterLink = {
	label: string
	href: string
	external?: boolean
}

type FooterContent = {
	primaryHeading: string
	primaryBody: string
	primaryCta: string
	secondaryCta: string
	columnLibraryTitle: string
	columnQuantumTitle: string
	columnLibraryLinks: FooterLink[]
	columnQuantumLinks: FooterLink[]
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
		columnLibraryTitle: 'Library',
		columnQuantumTitle: 'Quantum Box',
		columnLibraryLinks: [
			{ label: 'Features', href: '#features' },
			{ label: 'Pricing', href: '#pricing' },
			{ label: 'Roadmap', href: '#roadmap' },
		],
		columnQuantumLinks: [
			{
				label: 'Corporate site',
				href: 'https://www.quantum-box.com/',
				external: true,
			},
			{
				label: 'About us',
				href: 'https://www.quantum-box.com/about',
				external: true,
			},
			{
				label: 'Services',
				href: 'https://www.quantum-box.com/services',
				external: true,
			},
			{
				label: 'Products',
				href: 'https://www.quantum-box.com/products',
				external: true,
			},
			{
				label: 'Contact',
				href: 'https://www.quantum-box.com/contact',
				external: true,
			},
		],
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
		columnLibraryTitle: 'Library',
		columnQuantumTitle: 'Quantum Box',
		columnLibraryLinks: [
			{ label: '特徴', href: '#features' },
			{ label: '料金', href: '#pricing' },
			{ label: 'ロードマップ', href: '#roadmap' },
		],
		columnQuantumLinks: [
			{
				label: '企業サイト',
				href: 'https://www.quantum-box.com/',
				external: true,
			},
			{
				label: '私たちについて',
				href: 'https://www.quantum-box.com/about',
				external: true,
			},
			{
				label: 'サービス',
				href: 'https://www.quantum-box.com/services',
				external: true,
			},
			{
				label: 'プロダクト',
				href: 'https://www.quantum-box.com/products',
				external: true,
			},
			{
				label: 'お問い合わせ',
				href: 'https://www.quantum-box.com/contact',
				external: true,
			},
		],
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
			<section className='relative overflow-hidden rounded-[3rem] border border-white/10 bg-gradient-to-br from-sky-500/15 via-blue-700/20 to-slate-950 px-6 py-16 text-center shadow-[0_30px_100px_rgba(15,23,42,0.6)] sm:px-10 sm:py-20 lg:px-16'>
				<div className='pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(14,165,233,0.25),_transparent_65%)]' />
				<div className='relative mx-auto max-w-3xl space-y-6'>
					<p className='text-xs font-semibold uppercase tracking-[0.35em] text-sky-200/90'>
						Quantum Box, Inc.
					</p>
					<h2 className='text-3xl font-semibold text-white sm:text-4xl md:text-5xl'>
						{t.primaryHeading}
					</h2>
					<p className='text-base text-slate-100 sm:text-lg'>{t.primaryBody}</p>
					<div className='flex flex-col items-center justify-center gap-4 pt-4 sm:flex-row'>
						<Button
							className='inline-flex items-center justify-center gap-2 rounded-full bg-white px-6 py-3 text-base font-semibold text-slate-900 shadow-[0_20px_45px_rgba(255,255,255,0.35)] transition hover:shadow-[0_24px_55px_rgba(255,255,255,0.4)] sm:px-8 sm:py-4'
							asChild
						>
							<Link href='/sign_up'>
								{t.primaryCta}
								<ArrowRight className='h-5 w-5' />
							</Link>
						</Button>
						<Button
							variant='outline'
							className='inline-flex items-center justify-center gap-2 rounded-full border-white/30 bg-white/10 px-6 py-3 text-base font-semibold text-white transition hover:border-white/60 hover:bg-white/20 sm:px-8 sm:py-4'
							asChild
						>
							<Link
								href='https://www.quantum-box.com/contact'
								target='_blank'
								rel='noreferrer'
							>
								{t.secondaryCta}
								<ExternalLink className='h-5 w-5 text-sky-100' />
							</Link>
						</Button>
					</div>
				</div>
			</section>

			<footer className='mt-20 border-t border-white/10 bg-slate-950/90 backdrop-blur-lg'>
				<div className='container mx-auto grid gap-10 px-4 py-12 sm:grid-cols-[1.2fr_0.8fr_0.8fr] sm:px-6'>
					<div className='space-y-4 text-left'>
						<div className='flex items-center gap-3'>
							<div className='flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-cyan-400/90 via-sky-500/90 to-blue-600/90 text-white'>
								<svg
									xmlns='http://www.w3.org/2000/svg'
									viewBox='0 0 24 24'
									fill='none'
									stroke='currentColor'
									strokeWidth='2'
									strokeLinecap='round'
									strokeLinejoin='round'
									className='h-5 w-5'
								>
									<path d='M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20' />
								</svg>
							</div>
							<div>
								<p className='text-lg font-semibold text-white'>
									Quantum Box, Inc.
								</p>
								<p className='text-sm text-slate-300'>
									{lang === 'en'
										? 'Technology for everyone.'
										: 'テクノロジーを、みんなのものに。'}
								</p>
							</div>
						</div>
						<p className='text-sm text-slate-300'>{t.summaryLine}</p>
					</div>

					<div className='space-y-4 text-left'>
						<p className='text-sm font-semibold uppercase tracking-widest text-slate-400'>
							{t.columnLibraryTitle}
						</p>
						<ul className='space-y-3 text-sm text-slate-300'>
							{t.columnLibraryLinks.map(link => (
								<li key={link.href}>
									<Link
										href={link.href}
										className='transition hover:text-white'
									>
										{link.label}
									</Link>
								</li>
							))}
						</ul>
					</div>

					<div className='space-y-4 text-left'>
						<p className='text-sm font-semibold uppercase tracking-widest text-slate-400'>
							{t.columnQuantumTitle}
						</p>
						<ul className='space-y-3 text-sm text-slate-300'>
							{t.columnQuantumLinks.map(link => (
								<li key={link.href}>
									<Link
										href={link.href}
										className='inline-flex items-center gap-1 transition hover:text-white'
										target='_blank'
										rel='noreferrer'
									>
										{link.label}
										<ExternalLink className='h-3.5 w-3.5 text-slate-400' />
									</Link>
								</li>
							))}
						</ul>
					</div>
				</div>
				<div className='border-t border-white/10'>
					<div className='container mx-auto flex flex-col gap-4 px-4 py-6 text-xs text-slate-400 sm:flex-row sm:items-center sm:justify-between sm:px-6'>
						<p className='text-xs text-slate-400'>
							© {new Date().getFullYear()} Quantum Box, Inc. All rights
							reserved.
						</p>
						<div className='flex flex-wrap items-center gap-4 sm:gap-6'>
							{t.footerLinks.map(link => (
								<Link
									key={link.href}
									href={link.href}
									className='transition hover:text-white'
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
