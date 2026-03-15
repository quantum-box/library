import { Toaster } from '@/components/ui/toaster'
import { fetchBackendVersion, frontendVersion } from '@/lib/version'
import type { Metadata, Viewport } from 'next'
import { Inter } from 'next/font/google'
import './animations.css'
import './globals.css'
import Providers from './providers'

const inter = Inter({ subsets: ['latin'] })

// https://nextjs.org/docs/app/api-reference/file-conventions/route-segment-config
export const fetchCache = 'force-no-store'
export const revalidate = 10
export const runtime = 'edge'

export const viewport: Viewport = {
	width: 'device-width',
	initialScale: 1,
	maximumScale: 1,
}

export const metadata: Metadata = {
	title:
		'Library | Organizing and Accumulating Intellectual Property on a Earth Scale',
	description:
		'In a world where there is one fact but multiple truths, the Library aims to visualize all connections. By visualizing the value through interconnected information, the Library organizes all information in a human-scale repository and provides it in a user-friendly API format.',
}
export default async function RootLayout({
	children,
}: {
	children: React.ReactNode
}) {
	const backendVersion = await fetchBackendVersion()

	return (
		<html lang='en' suppressHydrationWarning>
			<body className={inter.className}>
				<Providers>
					{children}
					<footer className='mt-10 w-full border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60'>
						<div className='container flex h-10 items-center justify-end text-xs text-muted-foreground'>
							<span>v{frontendVersion}</span>
							<span className='mx-2'>·</span>
							<span>API v{backendVersion ?? 'unknown'}</span>
						</div>
					</footer>
					<Toaster />
				</Providers>
			</body>
		</html>
	)
}
