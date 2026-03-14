'use client'

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import {
	DataFieldOnRepoPageFragment,
	PropertyFieldOnRepoPageFragment,
	PropertyType,
} from '@/gen/graphql'
import { cn } from '@/lib/utils'
import { Calendar, Clock } from 'lucide-react'
import NextLink from 'next/link'

interface DataCardViewProps {
	data: DataFieldOnRepoPageFragment[]
	properties: PropertyFieldOnRepoPageFragment[]
	selectedIds: Set<string>
	onSelectItem: (id: string, checked: boolean) => void
	org: string
	repo: string
}

export function DataCardView({
	data,
	properties,
	selectedIds,
	onSelectItem,
	org,
	repo,
}: DataCardViewProps) {
	const formatDate = (value?: string | null) => {
		if (!value) return null
		const date = new Date(value)
		if (Number.isNaN(date.getTime())) return null
		return new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
		}).format(date)
	}

	const formatValue = (
		value: unknown,
		propertyType?: PropertyType,
	): string | null => {
		if (value == null) return null

		if (typeof value === 'boolean') {
			return value ? 'Yes' : 'No'
		}

		if (typeof value === 'number') {
			return value.toLocaleString()
		}

		if (typeof value === 'string') {
			if (value.length > 50) {
				return `${value.slice(0, 50)}...`
			}
			return value
		}

		if (Array.isArray(value)) {
			const formatted = value
				.slice(0, 3)
				.map(v => formatValue(v, propertyType))
				.filter(Boolean)
			if (value.length > 3) {
				return `${formatted.join(', ')} +${value.length - 3}`
			}
			return formatted.join(', ')
		}

		if (typeof value === 'object') {
			// Check __typename first for type safety
			const typename =
				'__typename' in value
					? (value as { __typename?: string }).__typename
					: null

			if (
				typename === 'DateValue' ||
				('date' in value && typeof value.date === 'string')
			) {
				return formatDate((value as { date: string }).date)
			}

			if (
				typename === 'IdValue' ||
				('id' in value &&
					typeof value.id === 'string' &&
					!('optionId' in value))
			) {
				return (value as { id: string }).id
			}

			if (
				typename === 'StringValue' ||
				('string' in value && typeof value.string === 'string')
			) {
				const str = (value as { string: string }).string
				if (str.trim() === '') return null
				if (str.length > 50) {
					return `${str.slice(0, 50)}...`
				}
				return str
			}

			if (typename === 'IntegerValue' || 'number' in value) {
				return String((value as { number: string | number }).number)
			}

			if (
				typename === 'SelectValue' ||
				('optionId' in value && typeof value.optionId === 'string')
			) {
				// For Select, we'd need property meta to get option name, but for now just return the ID
				return (value as { optionId: string }).optionId
			}

			if (
				typename === 'MultiSelectValue' ||
				('optionIds' in value && Array.isArray(value.optionIds))
			) {
				const optionIds = (value as { optionIds: string[] }).optionIds
				if (optionIds.length === 0) return null
				return optionIds.join(', ')
			}

			if (
				typename === 'LocationValue' ||
				('latitude' in value && 'longitude' in value)
			) {
				const loc = value as { latitude: number; longitude: number }
				return `${loc.latitude.toFixed(2)}, ${loc.longitude.toFixed(2)}`
			}

			return null
		}

		return String(value)
	}

	// Get first few properties to display on card
	const displayProperties = properties.slice(0, 3)

	return (
		<div className='grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4'>
			{data.map(item => {
				const isSelected = selectedIds.has(item.id)

				return (
					<Card
						key={item.id}
						className={cn(
							'group relative transition-all hover:shadow-md',
							isSelected && 'ring-2 ring-primary bg-primary/5',
						)}
					>
						{/* Checkbox overlay */}
						<div
							className={cn(
								'absolute top-3 left-3 z-10 transition-opacity',
								isSelected
									? 'opacity-100'
									: 'opacity-0 group-hover:opacity-100',
							)}
						>
							<Checkbox
								checked={isSelected}
								onCheckedChange={checked =>
									onSelectItem(item.id, Boolean(checked))
								}
								className='bg-background'
								aria-label={`Select ${item.name}`}
							/>
						</div>

						<NextLink
							href={`/v1beta/${org}/${repo}/data/${item.id}`}
							className='block'
						>
							<CardHeader className='pb-2'>
								<CardTitle className='text-base font-medium line-clamp-2 pr-6'>
									{item.name}
								</CardTitle>
							</CardHeader>
							<CardContent className='space-y-3'>
								{/* Property values */}
								{displayProperties.length > 0 && (
									<div className='space-y-1.5'>
										{displayProperties.map(prop => {
											const propData = item.propertyData.find(
												pd => pd.propertyId === prop.id,
											)
											const formattedValue = formatValue(
												propData?.value,
												prop.typ,
											)
											if (!formattedValue) return null

											return (
												<div
													key={prop.id}
													className='flex items-start gap-2 text-sm'
												>
													<span className='text-muted-foreground shrink-0'>
														{prop.name}:
													</span>
													<span className='text-foreground truncate'>
														{formattedValue}
													</span>
												</div>
											)
										})}
									</div>
								)}

								{/* Dates */}
								<div className='flex items-center gap-4 text-xs text-muted-foreground pt-2 border-t'>
									{item.createdAt && (
										<div className='flex items-center gap-1'>
											<Calendar className='h-3 w-3' />
											{formatDate(item.createdAt)}
										</div>
									)}
									{item.updatedAt && (
										<div className='flex items-center gap-1'>
											<Clock className='h-3 w-3' />
											{formatDate(item.updatedAt)}
										</div>
									)}
								</div>
							</CardContent>
						</NextLink>
					</Card>
				)
			})}
		</div>
	)
}
