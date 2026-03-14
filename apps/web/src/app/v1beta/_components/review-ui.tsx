'use client'

import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Link, MessageSquare, ThumbsDown, ThumbsUp } from 'lucide-react'
import { useState } from 'react'

export interface ReviewUiProps {
	reviewData: {
		title: string
		author: {
			name: string
			avatar: string
		}
		createdAt: string
		status: string
		description: string
		changes: {
			type: 'addition' | 'deletion' | 'modification'
			content?: string
			oldContent?: string
			newContent?: string
		}[]
		source?: {
			name: string
			url: string
		}[]
	}
}

export function ReviewUi({ reviewData }: ReviewUiProps) {
	const { t, locale } = useTranslation()
	const [activeTab, setActiveTab] = useState('diff')

	const formatDate = (dateString: string) => {
		return new Date(dateString).toLocaleDateString(
			locale === 'ja' ? 'ja-JP' : 'en-US',
		)
	}

	return (
		<main className='p-6'>
			<div className='max-w-4xl mx-auto'>
				<div className='flex justify-between items-center mb-6'>
					<h1 className='text-2xl font-bold'>
						{t.v1beta.review.review}: {reviewData.title}
					</h1>
					<Badge
						variant={reviewData.status === 'Open' ? 'default' : 'secondary'}
					>
						{reviewData.status}
					</Badge>
				</div>
				<div className='flex items-center space-x-4 mb-6'>
					<Avatar>
						<AvatarImage
							src={reviewData.author.avatar}
							alt={reviewData.author.name}
						/>
						<AvatarFallback>{reviewData.author.name.charAt(0)}</AvatarFallback>
					</Avatar>
					<div>
						<p className='font-medium'>{reviewData.author.name}</p>
						<p className='text-sm text-gray-500'>
							{t.v1beta.review.createdOn} {formatDate(reviewData.createdAt)}
						</p>
					</div>
				</div>
				<div className='bg-gray-100 p-4 rounded-lg mb-6'>
					<h2 className='font-semibold mb-2'>{t.v1beta.review.description}</h2>
					<p>{reviewData.description}</p>
				</div>
				{reviewData.source?.map(source => (
					<div className='bg-blue-50 p-4 rounded-lg mb-6' key={source.url}>
						<h2 className='font-semibold mb-2 flex items-center'>
							<Link className='w-4 h-4 mr-2' />
							{t.v1beta.review.source}
						</h2>
						<p className='font-medium mb-1'>{source.name}</p>
						<a
							href={source.url}
							target='_blank'
							rel='noopener noreferrer'
							className='text-blue-600 hover:underline'
						>
							{source.url}
						</a>
					</div>
				))}
				<Tabs value={activeTab} onValueChange={setActiveTab} className='mb-6'>
					<TabsList>
						<TabsTrigger value='diff'>{t.v1beta.review.diffView}</TabsTrigger>
						<TabsTrigger value='preview'>{t.v1beta.review.preview}</TabsTrigger>
					</TabsList>
					<TabsContent value='diff'>
						<div className='bg-white border rounded-lg'>
							<ScrollArea className='h-[400px]'>
								<div className='p-4 space-y-4'>
									{reviewData.changes.map((change, index) => (
										<div key={index} className='border-b pb-4 last:border-b-0'>
											{change.type === 'addition' && (
												<div className='bg-green-100 p-2 rounded'>
													<p className='text-green-800'>+ {change.content}</p>
												</div>
											)}
											{change.type === 'deletion' && (
												<div className='bg-red-100 p-2 rounded'>
													<p className='text-red-800'>- {change.content}</p>
												</div>
											)}
											{change.type === 'modification' && (
												<div className='space-y-2'>
													<div className='bg-red-100 p-2 rounded'>
														<p className='text-red-800'>
															- {change.oldContent}
														</p>
													</div>
													<div className='bg-green-100 p-2 rounded'>
														<p className='text-green-800'>
															+ {change.newContent}
														</p>
													</div>
												</div>
											)}
										</div>
									))}
								</div>
							</ScrollArea>
						</div>
					</TabsContent>
					<TabsContent value='preview'>
						<div className='bg-white border rounded-lg p-4'>
							<p>
								The champagne flowed freely, bubbling in crystal flutes that
								caught the light from the chandeliers overhead. Gatsby,
								resplendent in his pink suit, stood alone on the marble steps,
								his eyes scanning the crowd with a mixture of hope and anxiety.
							</p>
						</div>
					</TabsContent>
				</Tabs>
				<div className='space-y-4'>
					<div className='flex space-x-4'>
						<Button className='flex-1'>
							<ThumbsUp className='w-4 h-4 mr-2' />
							{t.v1beta.review.approve}
						</Button>
						<Button variant='outline' className='flex-1'>
							<ThumbsDown className='w-4 h-4 mr-2' />
							{t.v1beta.review.requestChanges}
						</Button>
					</div>
					<Textarea placeholder={t.v1beta.review.leaveComment} />
					<Button className='w-full'>
						<MessageSquare className='w-4 h-4 mr-2' />
						{t.v1beta.review.comment}
					</Button>
				</div>
			</div>
		</main>
	)
}
