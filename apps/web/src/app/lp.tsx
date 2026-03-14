import { Capabilities } from '@/components/lp/Capabilities'
import { Challenges } from '@/components/lp/Challenges'
import { Features } from '@/components/lp/Features'
import { Footer } from '@/components/lp/Footer'
import { Header } from '@/components/lp/Header'
import { Hero } from '@/components/lp/Hero'
import { Pricing } from '@/components/lp/Pricing'
import { Roadmap } from '@/components/lp/Roadmap'

export type LpLanguage = 'en' | 'ja'

export default function LP({ lang }: { lang: LpLanguage }) {
	return (
		<div className='relative min-h-screen overflow-hidden bg-slate-950 text-slate-100'>
			<div className='absolute inset-0 -z-10 overflow-hidden'>
				<div className='absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(59,130,246,0.22),_transparent_60%)]' />
				<div className='absolute inset-0 bg-[radial-gradient(circle_at_bottom,_rgba(14,165,233,0.18),_transparent_65%)]' />
				<div className='absolute inset-0 bg-[linear-gradient(115deg,_rgba(15,23,42,0.85)_0%,_rgba(15,23,42,0.4)_55%,_rgba(15,23,42,0.9)_100%)]' />
				<div
					className='absolute inset-0 opacity-25 mix-blend-soft-light'
					style={{
						backgroundImage:
							'linear-gradient(rgba(148, 163, 184, 0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(148, 163, 184, 0.08) 1px, transparent 1px)',
						backgroundSize: '80px 80px',
					}}
				/>
				<div className='absolute left-1/2 top-[6rem] h-[32rem] w-[32rem] -translate-x-1/2 rounded-full bg-blue-500/30 blur-3xl opacity-40' />
				<div className='absolute -left-32 bottom-[-12rem] h-[28rem] w-[28rem] rounded-full bg-cyan-500/20 blur-3xl opacity-50' />
			</div>

			<div className='relative z-10'>
				<Header lang={lang} />
				<main className='container mx-auto px-4 sm:px-6 lg:px-8 space-y-20 sm:space-y-24 pt-12 sm:pt-16 pb-24'>
					<Hero lang={lang} />
					<Features lang={lang} />
					<Capabilities lang={lang} />
					<Challenges lang={lang} />
					<Pricing lang={lang} />
					<Roadmap lang={lang} />
				</main>
				<Footer lang={lang} />
			</div>
		</div>
	)
}
