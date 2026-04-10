import { ThemeProvider } from '@/components/theme-provider'
import { AuthProvider } from '@/auth'
import type { ReactNode } from 'react'

const Providers = ({ children }: { children: ReactNode }) => {
	return (
		<AuthProvider>
			<ThemeProvider
				attribute='class'
				defaultTheme='system'
				enableSystem
				disableTransitionOnChange
			>
				{children}
			</ThemeProvider>
		</AuthProvider>
	)
}

export default Providers
