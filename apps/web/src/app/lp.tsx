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
		<div className='min-h-screen bg-white text-slate-900 antialiased'>
			<Header lang={lang} />
			<main className='mx-auto max-w-6xl px-4 sm:px-6 lg:px-8'>
				<Hero lang={lang} />
				<Features lang={lang} />
				<Capabilities lang={lang} />
				<Challenges lang={lang} />
				<Pricing lang={lang} />
				<Roadmap lang={lang} />
			</main>
			<Footer lang={lang} />
		</div>
	)
}
