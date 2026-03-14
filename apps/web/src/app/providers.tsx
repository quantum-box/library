'use client'

import { ThemeProvider } from '@/components/theme-provider'
import { SessionProvider } from 'next-auth/react'
import { NuqsAdapter } from 'nuqs/adapters/next/app'
import type { ReactNode } from 'react'

const Providers = ({ children }: { children: ReactNode }) => {
	return (
		<SessionProvider>
			<NuqsAdapter>
				<ThemeProvider
					attribute='class'
					defaultTheme='system'
					enableSystem
					disableTransitionOnChange
				>
					{children}
				</ThemeProvider>
			</NuqsAdapter>
		</SessionProvider>
	)
}

export default Providers
