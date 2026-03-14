'use client'

import { Button } from '@/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { type Locale, locales } from '@/lib/i18n/translations'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Globe } from 'lucide-react'

const languageNames: Record<Locale, string> = {
	en: 'English',
	ja: '日本語',
}

interface LanguageSwitcherProps {
	variant?: 'default' | 'ghost' | 'outline'
	size?: 'default' | 'sm' | 'lg' | 'icon'
	showLabel?: boolean
}

export function LanguageSwitcher({
	variant = 'ghost',
	size = 'sm',
	showLabel = true,
}: LanguageSwitcherProps) {
	const { locale, changeLocale, t } = useTranslation()

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button variant={variant} size={size} className='gap-2'>
					<Globe className='h-4 w-4' />
					{showLabel && (
						<span className='hidden sm:inline'>{languageNames[locale]}</span>
					)}
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align='end'>
				{locales.map(lang => (
					<DropdownMenuItem
						key={lang}
						onClick={() => changeLocale(lang)}
						className={locale === lang ? 'bg-accent' : ''}
					>
						{languageNames[lang]}
					</DropdownMenuItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	)
}
