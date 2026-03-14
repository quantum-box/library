'use client'

import { Button } from '@/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { cn } from '@/lib/utils'
import { ChevronDown, Menu } from 'lucide-react'
import Link from 'next/link'
import { useParams, usePathname } from 'next/navigation'

interface NavigationItem {
	value: string
	label: string
	badgeCount?: number
}

export interface NavigationProps {
	items: NavigationItem[]
	className?: string
}

export function Navigation({ items, className }: NavigationProps) {
	const { t } = useTranslation()
	const pathname = usePathname()
	const { org, repo } = useParams<{ org: string; repo: string }>()

	// Get translated label based on value
	const getTranslatedLabel = (value: string) => {
		const tabTranslations = t.v1beta.repositoryPage.tabs as Record<
			string,
			string
		>
		return tabTranslations[value] || value
	}

	// パスがルートパスか確認する関数
	const isRootPath = () => {
		return pathname === `/v1beta/${org}/${repo}`
	}

	return (
		<nav className={cn('border-b', className)}>
			{/* モバイル用ドロップダウンナビゲーション */}
			<div className='block sm:hidden p-2'>
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<Button
							variant='outline'
							size='sm'
							className='w-full justify-between'
						>
							<span className='flex items-center'>
								<Menu className='w-4 h-4 mr-2' />
								{t.v1beta.navigation.navigation}
							</span>
							<ChevronDown className='w-4 h-4 ml-2' />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent className='w-full min-w-[200px]'>
						{items.map(item => {
							const isActive =
								item.value === 'contents'
									? isRootPath()
									: pathname.includes(item.value)
							return (
								<DropdownMenuItem key={item.value} asChild>
									<Link
										href={
											item.value === 'contents'
												? `/v1beta/${org}/${repo}`
												: `/v1beta/${org}/${repo}/${item.value}`
										}
										className={cn(
											'flex items-center justify-between w-full px-4 py-2',
											isActive && 'bg-accent',
										)}
									>
										<span>{getTranslatedLabel(item.value)}</span>
										{item.badgeCount && (
											<span className='ml-2 rounded-full bg-muted px-2 py-0.5 text-xs'>
												{item.badgeCount}
											</span>
										)}
									</Link>
								</DropdownMenuItem>
							)
						})}
					</DropdownMenuContent>
				</DropdownMenu>
			</div>

			{/* デスクトップ用ナビゲーション */}
			<div className='hidden sm:block'>
				<div className='flex h-10 items-center px-4 space-x-4 container'>
					{items.map(item => {
						const isActive =
							item.value === 'contents'
								? isRootPath()
								: pathname.includes(item.value)
						return (
							<Link
								key={item.value}
								href={
									item.value === 'contents'
										? `/v1beta/${org}/${repo}`
										: `/v1beta/${org}/${repo}/${item.value}`
								}
								className={cn(
									'flex items-center space-x-2 text-sm font-medium transition-colors hover:text-primary',
									isActive ? 'text-foreground' : 'text-muted-foreground',
								)}
							>
								<span>{getTranslatedLabel(item.value)}</span>
								{item.badgeCount && (
									<span className='rounded-full bg-muted px-2 py-0.5 text-xs'>
										{item.badgeCount}
									</span>
								)}
							</Link>
						)
					})}
				</div>
			</div>
		</nav>
	)
}
