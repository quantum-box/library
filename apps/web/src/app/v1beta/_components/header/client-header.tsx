'use client'

import { LanguageSwitcher } from '@/components/language-switcher'
import { ModeToggle } from '@/components/theme-toggle'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Menu, User } from 'lucide-react'
import { signOut } from 'next-auth/react'
import Link from 'next/link'
import { useParams } from 'next/navigation'
import { useState } from 'react'
import { ClientHeaderProps } from './types'

export function ClientHeader({ session, publicNavItems }: ClientHeaderProps) {
	const [isOpen, setIsOpen] = useState(false)
	const { org, repo } = useParams<{ org?: string; repo?: string }>()
	const { t } = useTranslation()

	// orgとrepoの表示用コンポーネント
	const RepoPath = () => {
		if (!org) return null

		return (
			<div className='hidden md:flex items-center gap-1 ml-4 text-sm text-muted-foreground'>
				<Link href={`/v1beta/${org}`} className='hover:text-foreground'>
					{org}
				</Link>
				{repo && (
					<>
						<span> / </span>
						<Link
							href={`/v1beta/${org}/${repo}`}
							className='hover:text-foreground'
						>
							{repo}
						</Link>
					</>
				)}
			</div>
		)
	}

	// ユーザー名の表示（存在する場合）
	const displayName =
		session?.user?.username ||
		session?.user?.name ||
		session?.user?.email ||
		'User'

	// ログイン後のユーザーメニュー
	const UserMenu = () => (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button variant='ghost' className='relative h-8 w-8 rounded-full'>
					<Avatar className='h-8 w-8'>
						<AvatarImage src={session?.user?.image || ''} alt={displayName} />
						<AvatarFallback>
							<User className='h-4 w-4' />
						</AvatarFallback>
					</Avatar>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align='end'>
				<div className='px-2 py-1.5 text-sm font-medium'>{displayName}</div>
				<DropdownMenuSeparator />
				<DropdownMenuItem>
					<Link href='/v1beta/profile'>Profile</Link>
				</DropdownMenuItem>
				<DropdownMenuItem>
					<Link href='/v1beta/settings'>{t.common.settings}</Link>
				</DropdownMenuItem>
				<DropdownMenuSeparator />
				<DropdownMenuItem onClick={() => signOut()}>
					{t.auth.signOut.confirm}
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	)

	// モバイルメニューの認証関連コンテンツ
	const MobileAuthContent = () =>
		session ? (
			<>
				<div className='py-2 font-medium'>{displayName}</div>
				<Link href='/v1beta/profile' onClick={() => setIsOpen(false)}>
					Profile
				</Link>
				<Link href='/v1beta/settings' onClick={() => setIsOpen(false)}>
					{t.common.settings}
				</Link>
				<Button variant='ghost' onClick={() => signOut()}>
					{t.auth.signOut.confirm}
				</Button>
			</>
		) : (
			<div className='flex flex-col gap-2'>
				<Button variant='ghost' asChild>
					<Link href='/sign_in' onClick={() => setIsOpen(false)}>
						{t.auth.signIn.title}
					</Link>
				</Button>
				<Button asChild>
					<Link href='/sign_up' onClick={() => setIsOpen(false)}>
						{t.auth.signUp.title}
					</Link>
				</Button>
			</div>
		)

	return (
		<>
			<RepoPath />

			{/* Desktop Navigation - 未ログイン時のみ表示 */}
			{!session && (
				<nav className='hidden md:flex md:flex-1'>
					<ul className='flex gap-6'>
						{publicNavItems.map(item => (
							<li key={item.href}>
								<Link
									href={item.href}
									className='text-sm font-medium transition-colors hover:text-foreground/80'
								>
									{item.label}
								</Link>
							</li>
						))}
					</ul>
				</nav>
			)}

			{/* Spacer to push auth content to the right */}
			<div className='flex-1' />

			<LanguageSwitcher variant='ghost' size='sm' />
			<ModeToggle className='ml-2 mr-4' />
			{/* Desktop Auth Buttons/User Menu */}
			<div className='hidden md:flex md:items-center md:gap-2'>
				{session ? (
					<UserMenu />
				) : (
					<>
						<Button variant='ghost' asChild>
							<Link href='/sign_in'>{t.auth.signIn.title}</Link>
						</Button>
						<Button asChild>
							<Link href='/sign_up'>{t.auth.signUp.title}</Link>
						</Button>
					</>
				)}
			</div>

			{/* Mobile Menu Button */}
			<div className='md:hidden'>
				<Sheet open={isOpen} onOpenChange={setIsOpen}>
					<SheetTrigger asChild>
						<Button variant='ghost' size='icon' className='md:hidden'>
							<Menu className='h-5 w-5' />
							<span className='sr-only'>Toggle menu</span>
						</Button>
					</SheetTrigger>
					<SheetContent side='right' className='w-[80vw] sm:w-[350px]'>
						<nav className='flex flex-col gap-4'>
							{/* モバイルナビゲーション - 未ログイン時のみ表示 */}
							{!session &&
								publicNavItems.map(item => (
									<Link
										key={item.href}
										href={item.href}
										className='text-lg font-medium'
										onClick={() => setIsOpen(false)}
									>
										{item.label}
									</Link>
								))}
							<hr className='my-4' />
							<MobileAuthContent />
						</nav>
					</SheetContent>
				</Sheet>
			</div>
		</>
	)
}
