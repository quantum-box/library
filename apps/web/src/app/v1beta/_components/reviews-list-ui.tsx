'use client'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { ArrowUpDown, Filter, MoreHorizontal } from 'lucide-react'
import { useState } from 'react'

export type ReviewListUiProps = {
	reviews: Array<{
		id: string
		title: string
		author: string
		status: string
		createdAt: string
		type: string
	}>
}
export function ReviewListUi({
	reviews,
}: {
	reviews: Array<{
		id: string
		title: string
		author: string
		status: string
		createdAt: string
		type: string
	}>
}) {
	const { t, locale } = useTranslation()
	const [selectedReviews, setSelectedReviews] = useState<string[]>([])

	const toggleSelectAll = () => {
		if (selectedReviews.length === reviews.length) {
			setSelectedReviews([])
		} else {
			setSelectedReviews(reviews.map(review => review.id))
		}
	}

	const toggleSelectReview = (id: string) => {
		setSelectedReviews(prev =>
			prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id],
		)
	}

	const formatDate = (dateString: string) => {
		return new Date(dateString).toLocaleDateString(
			locale === 'ja' ? 'ja-JP' : 'en-US',
		)
	}

	return (
		<main className='flex-1 overflow-auto p-6'>
			<div className='max-w-6xl mx-auto'>
				<div className='flex justify-between items-center mb-6'>
					<h1 className='text-2xl font-bold'>
						{t.v1beta.reviewList.reviewRequests}
					</h1>
					<div className='flex space-x-2'>
						<Select>
							<SelectTrigger className='w-[180px]'>
								<SelectValue placeholder={t.v1beta.reviewList.filterByStatus} />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value='all'>
									{t.v1beta.reviewList.allStatuses}
								</SelectItem>
								<SelectItem value='open'>{t.v1beta.reviewList.open}</SelectItem>
								<SelectItem value='in-progress'>
									{t.v1beta.reviewList.inProgress}
								</SelectItem>
								<SelectItem value='closed'>
									{t.v1beta.reviewList.closed}
								</SelectItem>
							</SelectContent>
						</Select>
						<Button variant='outline'>
							<Filter className='w-4 h-4 mr-2' />
							{t.v1beta.reviewList.moreFilters}
						</Button>
					</div>
				</div>
				<div className='bg-white border rounded-lg overflow-hidden'>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead className='w-[50px]'>
									<Checkbox
										checked={selectedReviews.length === reviews.length}
										onCheckedChange={toggleSelectAll}
										aria-label={t.v1beta.reviewList.selectAll}
									/>
								</TableHead>
								<TableHead className='max-w-[300px]'>
									{t.v1beta.reviewList.title}
								</TableHead>
								<TableHead>{t.v1beta.reviewList.author}</TableHead>
								<TableHead>{t.v1beta.reviewList.type}</TableHead>
								<TableHead>
									<Button variant='ghost' className='hover:bg-transparent'>
										{t.v1beta.reviewList.status}
										<ArrowUpDown className='ml-2 h-4 w-4' />
									</Button>
								</TableHead>
								<TableHead>
									<Button variant='ghost' className='hover:bg-transparent'>
										{t.v1beta.reviewList.created}
										<ArrowUpDown className='ml-2 h-4 w-4' />
									</Button>
								</TableHead>
								<TableHead className='w-[50px]' />
							</TableRow>
						</TableHeader>
						<TableBody>
							{reviews.map(review => (
								<TableRow key={review.id}>
									<TableCell>
										<Checkbox
											checked={selectedReviews.includes(review.id)}
											onCheckedChange={() => toggleSelectReview(review.id)}
											aria-label={`${t.v1beta.reviewList.selectReview} ${review.title}`}
										/>
									</TableCell>
									<TableCell className='font-medium max-w-[300px] truncate'>
										{review.title}
									</TableCell>
									<TableCell>{review.author}</TableCell>
									<TableCell>{review.type}</TableCell>
									<TableCell>
										<Badge
											variant={
												review.status === 'Open'
													? 'default'
													: review.status === 'In Progress'
														? 'secondary'
														: 'outline'
											}
										>
											{review.status}
										</Badge>
									</TableCell>
									<TableCell>{formatDate(review.createdAt)}</TableCell>
									<TableCell>
										<DropdownMenu>
											<DropdownMenuTrigger asChild>
												<Button variant='ghost' className='h-8 w-8 p-0'>
													<span className='sr-only'>
														{t.v1beta.reviewList.openMenu}
													</span>
													<MoreHorizontal className='h-4 w-4' />
												</Button>
											</DropdownMenuTrigger>
											<DropdownMenuContent align='end'>
												<DropdownMenuLabel>
													{t.v1beta.reviewList.actions}
												</DropdownMenuLabel>
												<DropdownMenuItem>
													{t.v1beta.reviewList.viewDetails}
												</DropdownMenuItem>
												<DropdownMenuItem>
													{t.v1beta.reviewList.assignReviewer}
												</DropdownMenuItem>
												<DropdownMenuSeparator />
												<DropdownMenuItem>
													{t.v1beta.reviewList.changeStatus}
												</DropdownMenuItem>
											</DropdownMenuContent>
										</DropdownMenu>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</div>
				<div className='mt-4 flex justify-between items-center'>
					<p className='text-sm text-gray-500'>
						{t.v1beta.reviewList.selectedOf
							.replace('{selected}', String(selectedReviews.length))
							.replace('{total}', String(reviews.length))}
					</p>
					<div className='space-x-2'>
						<Button variant='outline' size='sm'>
							{t.v1beta.reviewList.previous}
						</Button>
						<Button variant='outline' size='sm'>
							{t.v1beta.reviewList.next}
						</Button>
					</div>
				</div>
			</div>
		</main>
	)
}
