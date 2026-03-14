'use client'

import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { DataListForDataListCardFragment } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { cn } from '@/lib/utils'
import { FileText, Plus } from 'lucide-react'
import Link from 'next/link'
import { useParams } from 'next/navigation'

export function DataListCard({
	orgUsername,
	repoUsername,
	dataItems,
	canCreate = true,
	hideHeader = false,
	onItemClick,
}: {
	orgUsername: string
	repoUsername: string
	dataItems?: DataListForDataListCardFragment
	canCreate?: boolean
	hideHeader?: boolean
	onItemClick?: () => void
}) {
	const { t } = useTranslation()
	const params = useParams<{ dataId?: string }>()
	const activeId = params?.dataId
	const newHref = `/v1beta/${orgUsername}/${repoUsername}/data/new`

	return (
		<div
			className={cn(
				'overflow-hidden',
				!hideHeader &&
					'rounded-2xl border border-border/60 bg-card/80 shadow-sm backdrop-blur',
			)}
		>
			{!hideHeader && (
				<div className='flex items-center justify-between border-b border-border/60 px-4 py-3'>
					<span className='text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground'>
						{t.v1beta.dataDetail.dataList}
					</span>
					{canCreate && (
						<Button
							asChild
							variant='ghost'
							size='sm'
							className='gap-2 text-xs font-medium'
						>
							<Link href={newHref}>
								<Plus className='h-4 w-4' />
								{t.v1beta.dataDetail.createNew}
							</Link>
						</Button>
					)}
				</div>
			)}
			{hideHeader && canCreate && (
				<div className='flex justify-end px-2 pt-2'>
					<Button
						asChild
						variant='ghost'
						size='sm'
						className='gap-2 text-xs font-medium'
					>
						<Link href={newHref}>
							<Plus className='h-4 w-4' />
							{t.v1beta.dataDetail.createNew}
						</Link>
					</Button>
				</div>
			)}
			<ScrollArea
				className={cn(
					hideHeader
						? 'h-[calc(100vh-120px)]'
						: 'h-[280px] lg:h-[calc(100vh-220px)]',
				)}
			>
				<div className='flex flex-col gap-1 px-2 py-2'>
					{dataItems?.items.length ? (
						dataItems.items.map(item => {
							const href = `/v1beta/${orgUsername}/${repoUsername}/data/${item.id}`
							const isActive = activeId === item.id
							return (
								<Button
									key={item.id}
									variant={isActive ? 'secondary' : 'ghost'}
									size='sm'
									asChild
									className={cn(
										'justify-start gap-3 rounded-xl px-3 py-5 text-left',
										isActive
											? 'font-semibold'
											: 'font-medium text-muted-foreground hover:text-foreground',
									)}
								>
									<Link href={href} onClick={onItemClick}>
										<FileText className='h-4 w-4 flex-shrink-0 text-muted-foreground' />
										<span className='truncate'>
											{item.name?.trim() || 'Untitled'}
										</span>
									</Link>
								</Button>
							)
						})
					) : (
						<div className='rounded-xl border border-dashed border-border/60 bg-background/40 px-4 py-6 text-sm text-muted-foreground'>
							{t.v1beta.dataDetail.noData}
						</div>
					)}
				</div>
			</ScrollArea>
		</div>
	)
}
