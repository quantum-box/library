import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { DatePicker } from '@/components/ui/date-picker'
import { Input } from '@/components/ui/input'
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
import type { SelectItem as GraphQLSelectItem } from '@/gen/graphql'
import {
	DataForDataDetailFragment,
	DateValueForEditorFragment,
	IdValueForEditorFragment,
	ImageValueForEditorFragment,
	IntegerValueForEditorFragment,
	LocationValueForEditorFragment,
	MultiSelectType,
	MultiSelectValueForEditorFragment,
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
	RelationType,
	RelationValueForEditorFragment,
	SelectType,
	SelectValueForEditorFragment,
	StringValueForEditorFragment,
} from '@/gen/graphql'
import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { type AvailableRepo, ExtGithubEditor } from '../ext-github-editor'
import { LocationMap, LocationMapCompact } from '../../location-map'

/** Check if property is ext_github */
function isExtGithubProperty(property: PropertyForEditorFragment): boolean {
	return property.name === 'ext_github'
}

/** Check if property is ext_linear */
function isExtLinearProperty(property: PropertyForEditorFragment): boolean {
	return property.name === 'ext_linear'
}

/** Check if property is ext_github_repos (repository configuration) */
function isExtGithubReposProperty(
	property: PropertyForEditorFragment,
): boolean {
	return property.name === 'ext_github_repos'
}

/** Check if property is a system property that should be read-only */
function isSystemProperty(property: PropertyForEditorFragment): boolean {
	return ['id', 'createdAt', 'updatedAt'].includes(property.name)
}

/** Parse ext_github property meta JSON to get available repo configs */
function parseGitHubRepos(
	properties: PropertyForEditorFragment[],
): AvailableRepo[] {
	// Find ext_github property and get its meta.json
	const extGithubProp = properties.find(isExtGithubProperty)
	if (!extGithubProp?.meta) return []

	const jsonMeta = (extGithubProp.meta as { json?: string } | null)?.json
	if (!jsonMeta) return []

	try {
		const parsed = JSON.parse(jsonMeta)
		if (Array.isArray(parsed)) {
			return parsed.map(item => ({
				repo: item.repo,
				label: item.label,
				defaultPath: item.defaultPath,
			}))
		}
		return []
	} catch {
		return []
	}
}

export function PropertiesSection({
	properties,
	data,
	isEditing,
	onPropertyChange,
}: {
	properties: PropertyForEditorFragment[]
	data?: DataForDataDetailFragment
	isEditing: boolean
	onPropertyChange: (input: PropertyDataForEditorFragment) => void
}) {
	// Get available GitHub repos from ext_github property meta
	const availableRepos = useMemo(
		() => parseGitHubRepos(properties),
		[properties],
	)

	const filteredProperties = useMemo(
		() =>
			properties.filter(
				property =>
					property.typ !== PropertyType.Html &&
					property.typ !== PropertyType.Markdown &&
					// Hide ext_github_repos from the data editor (it's for property config)
					!isExtGithubReposProperty(property) &&
					// Hide ext_linear from the data editor (shown in Linear Sync section)
					!isExtLinearProperty(property),
			),
		[properties],
	)

	return (
		<section className='px-3 py-5 sm:px-5'>
			<div className='flex items-center justify-between pb-2'>
				<p className='text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground'>
					Properties
				</p>
				{isEditing && (
					<span className='text-[11px] uppercase tracking-wide text-muted-foreground'>
						Editing
					</span>
				)}
			</div>
			{filteredProperties.length ? (
				<Table>
					<TableHeader>
						<TableRow className='border-border/60'>
							<TableHead className='w-48 text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground'>
								Field
							</TableHead>
							<TableHead className='text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground'>
								Value
							</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{filteredProperties.map(property => (
							<TableRow key={property.id} className='border-border/40'>
								<TableCell className='align-top text-sm font-medium text-foreground'>
									{property.name}
									{isExtGithubProperty(property) && (
										<span className='ml-2 text-xs text-muted-foreground'>
											(GitHub Sync)
										</span>
									)}
									{isSystemProperty(property) && (
										<span className='ml-2 text-xs text-muted-foreground'>
											(System)
										</span>
									)}
								</TableCell>
								<TableCell className='align-top'>
									<PropertyValue
										property={property}
										propertyDatas={data?.propertyData}
										dataId={data?.id}
										isEditing={isEditing && !isSystemProperty(property)}
										onChange={onPropertyChange}
										availableRepos={
											isExtGithubProperty(property) ? availableRepos : undefined
										}
									/>
								</TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			) : (
				<div className='rounded-xl border border-dashed border-border/60 bg-background/60 px-4 py-6 text-sm text-muted-foreground'>
					No secondary properties configured yet.
				</div>
			)}
		</section>
	)
}

/**
 *
 * PropertyDataを一つだけとりだしたい
 * PropertyData以外のところはUIを自由に組み替えたい可能性あり
 */
function getPropertyValue(
	property: PropertyForEditorFragment,
	propertyData: PropertyDataForEditorFragment | undefined,
) {
	if (!propertyData) return null

	switch (property.typ) {
		case PropertyType.String:
			return (propertyData.value as StringValueForEditorFragment).string
		case PropertyType.Integer:
			return (propertyData.value as IntegerValueForEditorFragment).number
		case PropertyType.Id:
			return (propertyData.value as IdValueForEditorFragment).id
		case PropertyType.Relation:
			return (
				propertyData.value as RelationValueForEditorFragment
			).dataIds.join(', ')
		case PropertyType.Select:
			return (propertyData.value as SelectValueForEditorFragment).optionId
		case PropertyType.MultiSelect:
			return (propertyData.value as MultiSelectValueForEditorFragment).optionIds
		case PropertyType.Location: {
			const locationVal = propertyData.value as LocationValueForEditorFragment
			return {
				latitude: locationVal.latitude,
				longitude: locationVal.longitude,
			}
		}
		case PropertyType.Date: {
			const dateVal = propertyData.value as
				| DateValueForEditorFragment
				| undefined
			return dateVal?.date ?? null
		}
		case PropertyType.Image: {
			const imageVal = propertyData.value as
				| ImageValueForEditorFragment
				| undefined
			return imageVal?.url ?? null
		}
		default:
			return null
	}
}

export function PropertyValue({
	property,
	propertyDatas,
	dataId,
	isEditing,
	onChange,
	availableRepos,
}: {
	property: PropertyForEditorFragment
	propertyDatas?: PropertyDataForEditorFragment[]
	/** The ID of the data entity (used for id property) */
	dataId?: string
	isEditing: boolean
	onChange: (input: PropertyDataForEditorFragment) => void
	availableRepos?: AvailableRepo[]
}) {
	const propertyData = propertyDatas?.find(v => v.propertyId === property.id)
	// For id property, use the data's ID directly since it's not stored in propertyData
	const value =
		property.name === 'id' ? dataId : getPropertyValue(property, propertyData)
	const [selectedOptions, setSelectedOptions] = useState<string[]>([])

	useEffect(() => {
		if (property.typ === PropertyType.MultiSelect && Array.isArray(value)) {
			setSelectedOptions(value)
		}
	}, [property.typ, value])

	const emit = useCallback(
		(payload: PropertyDataForEditorFragment['value']) => {
			onChange({
				propertyId: property.id,
				value: payload,
			} as PropertyDataForEditorFragment)
		},
		[onChange, property.id],
	)

	// Special handling for ext_github property
	if (isExtGithubProperty(property)) {
		const handleExtGithubChange = (jsonValue: string) => {
			emit({
				__typename: 'StringValue',
				string: jsonValue,
			})
		}

		return (
			<ExtGithubEditor
				value={typeof value === 'string' ? value : undefined}
				isEditing={isEditing}
				onChange={handleExtGithubChange}
				availableRepos={availableRepos}
			/>
		)
	}

	const handleTextChange = useCallback(
		(event: React.ChangeEvent<HTMLInputElement>) => {
			const nextValue = event.target.value
			if (property.typ === PropertyType.Integer) {
				emit({
					__typename: 'IntegerValue',
					number: nextValue,
				})
				return
			}
			if (property.typ === PropertyType.Relation) {
				const relationMeta = property.meta as RelationType | undefined
				emit({
					__typename: 'RelationValue',
					databaseId: relationMeta?.databaseId ?? '',
					dataIds: nextValue
						.split(',')
						.map(item => item.trim())
						.filter(Boolean),
				})
				return
			}
			emit({
				__typename: 'StringValue',
				string: nextValue,
			})
		},
		[emit, property.meta, property.typ],
	)

	const handleSelectChange = useCallback(
		(optionId: string) => {
			emit({
				__typename: 'SelectValue',
				optionId,
			})
		},
		[emit],
	)

	const handleMultiSelectChange = useCallback(
		(optionId: string) => {
			const next = selectedOptions.includes(optionId)
				? selectedOptions.filter(id => id !== optionId)
				: [...selectedOptions, optionId]
			setSelectedOptions(next)
			emit({
				__typename: 'MultiSelectValue',
				optionIds: next,
			})
		},
		[emit, selectedOptions],
	)

	const handleLocationChange = useCallback(
		(field: 'latitude' | 'longitude', inputValue: string) => {
			const currentLocation =
				value && typeof value === 'object' && 'latitude' in value
					? (value as { latitude: number; longitude: number })
					: { latitude: 0, longitude: 0 }
			const numValue = Number.parseFloat(inputValue) || 0
			emit({
				__typename: 'LocationValue',
				latitude: field === 'latitude' ? numValue : currentLocation.latitude,
				longitude: field === 'longitude' ? numValue : currentLocation.longitude,
			})
		},
		[emit, value],
	)

	if (
		!value &&
		property.typ !== PropertyType.MultiSelect &&
		property.typ !== PropertyType.Location &&
		property.typ !== PropertyType.Select &&
		property.typ !== PropertyType.Date
	) {
		return isEditing ? (
			<Input
				name={`property-${property.name}`}
				value=''
				onChange={handleTextChange}
				placeholder='Add a value'
				className='h-10 w-full rounded-xl border-border/60 bg-background/80'
			/>
		) : (
			<span className='text-xs text-muted-foreground'>No value</span>
		)
	}

	// Get options from property meta based on type
	const getOptions = () => {
		if (!property.meta) return []

		if (property.typ === PropertyType.Select) {
			return (property.meta as SelectType).options
		}

		if (property.typ === PropertyType.MultiSelect) {
			return (property.meta as MultiSelectType).options
		}

		return []
	}

	const options = getOptions()

	if (isEditing) {
		switch (property.typ) {
			case PropertyType.String:
			case PropertyType.Integer:
			case PropertyType.Relation:
				return (
					<Input
						name={`property-${property.name}`}
						type='text'
						value={typeof value === 'string' ? value : ''}
						onChange={handleTextChange}
						placeholder='Add a value'
						className='h-10 w-full rounded-xl border-border/60 bg-background/80'
					/>
				)
			case PropertyType.Image: {
				const imageUrl = typeof value === 'string' ? value : ''
				return (
					<div className='space-y-2'>
						<Input
							name={`property-${property.name}`}
							type='url'
							value={imageUrl}
							onChange={e => {
								emit({
									__typename: 'ImageValue',
									url: e.target.value,
								})
							}}
							placeholder='Enter image URL'
							className='h-10 w-full rounded-xl border-border/60 bg-background/80'
						/>
						{imageUrl && (
							<img
								src={imageUrl}
								alt={property.name}
								className='max-h-48 max-w-full rounded-lg border border-border/60 object-contain'
								onError={e => {
									;(e.target as HTMLImageElement).style.display = 'none'
								}}
							/>
						)}
					</div>
				)
			}
			case PropertyType.Date: {
				const dateValue =
					typeof value === 'string' && value ? new Date(value) : undefined
				const isValidDate = dateValue && !Number.isNaN(dateValue.getTime())
				return (
					<DatePicker
						date={isValidDate ? dateValue : undefined}
						onDateChange={date => {
							if (date) {
								const dateString = date.toISOString().split('T')[0]
								emit({
									__typename: 'DateValue',
									date: dateString,
								})
							} else {
								emit({
									__typename: 'DateValue',
									date: '',
								})
							}
						}}
						placeholder='Pick a date'
					/>
				)
			}
			case PropertyType.Select:
				return (
					<Select
						value={typeof value === 'string' ? value : ''}
						onValueChange={handleSelectChange}
					>
						<SelectTrigger className='h-10 w-full rounded-xl border-border/60 bg-background/80'>
							<SelectValue placeholder='Select an option' />
						</SelectTrigger>
						<SelectContent>
							{options.map((option: GraphQLSelectItem) => (
								<SelectItem key={option.id} value={option.id}>
									{option.name}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				)
			case PropertyType.MultiSelect:
				return (
					<div className='space-y-2'>
						{options.map((option: GraphQLSelectItem) => (
							<div key={option.id} className='flex items-center space-x-2'>
								<Checkbox
									id={`option-${option.id}`}
									checked={selectedOptions.includes(option.id)}
									onCheckedChange={() => handleMultiSelectChange(option.id)}
								/>
								<label
									htmlFor={`option-${option.id}`}
									className='text-sm font-medium leading-none text-foreground'
								>
									{option.name}
								</label>
							</div>
						))}
					</div>
				)
			case PropertyType.Location: {
				const locationValue =
					value && typeof value === 'object' && 'latitude' in value
						? (value as { latitude: number; longitude: number })
						: { latitude: 35.6895, longitude: 139.6917 }
				return (
					<div className='space-y-3'>
						<LocationMap
							latitude={locationValue.latitude}
							longitude={locationValue.longitude}
							editable
							onChange={(lat, lng) => {
								emit({
									__typename: 'LocationValue',
									latitude: lat,
									longitude: lng,
								})
							}}
						/>
						<div className='flex gap-2'>
							<div className='flex items-center gap-2 flex-1'>
								<span className='text-xs text-muted-foreground w-12'>Lat</span>
								<Input
									name={`property-${property.name}-latitude`}
									type='number'
									step='any'
									min={-90}
									max={90}
									value={locationValue.latitude}
									onChange={e =>
										handleLocationChange('latitude', e.target.value)
									}
									placeholder='-90 to 90'
									className='h-8 rounded-lg border-border/60 bg-background/80 text-sm'
								/>
							</div>
							<div className='flex items-center gap-2 flex-1'>
								<span className='text-xs text-muted-foreground w-12'>Lng</span>
								<Input
									name={`property-${property.name}-longitude`}
									type='number'
									step='any'
									min={-180}
									max={180}
									value={locationValue.longitude}
									onChange={e =>
										handleLocationChange('longitude', e.target.value)
									}
									placeholder='-180 to 180'
									className='h-8 rounded-lg border-border/60 bg-background/80 text-sm'
								/>
							</div>
						</div>
					</div>
				)
			}
			default:
				return null
		}
	}

	// Display mode
	switch (property.typ) {
		case PropertyType.MultiSelect:
			if (!Array.isArray(value)) return null
			return (
				<div className='flex flex-wrap gap-2'>
					{value.map(optionId => {
						const option = options.find(
							(o: GraphQLSelectItem) => o.id === optionId,
						)
						return option ? (
							<Badge
								key={optionId}
								variant='outline'
								className='rounded-full border-border/70 bg-muted/70 px-3 py-1 text-xs font-medium'
							>
								{option.name}
							</Badge>
						) : null
					})}
					{!value.length && (
						<span className='text-xs text-muted-foreground'>No values</span>
					)}
				</div>
			)
		case PropertyType.Select: {
			const selectValue = typeof value === 'string' ? value : ''
			const selectedOption = options.find(
				(o: GraphQLSelectItem) => o.id === selectValue,
			)
			return (
				<Badge
					variant='outline'
					className='rounded-full border-border/70 bg-muted/70 px-3 py-1 text-xs font-medium'
				>
					{selectedOption?.name || selectValue}
				</Badge>
			)
		}
		case PropertyType.Location: {
			if (!value || typeof value !== 'object' || !('latitude' in value)) {
				return <span className='text-xs text-muted-foreground'>No value</span>
			}
			const locationValue = value as { latitude: number; longitude: number }
			return (
				<LocationMapCompact
					latitude={locationValue.latitude}
					longitude={locationValue.longitude}
				/>
			)
		}
		case PropertyType.Date: {
			if (!value || typeof value !== 'string') {
				return <span className='text-xs text-muted-foreground'>No value</span>
			}
			const date = new Date(value)
			if (Number.isNaN(date.getTime())) {
				return <span className='text-xs text-muted-foreground'>No value</span>
			}
			return (
				<span className='text-sm'>
					{date.toLocaleDateString('ja-JP', {
						year: 'numeric',
						month: '2-digit',
						day: '2-digit',
					})}
				</span>
			)
		}
		case PropertyType.Image: {
			if (!value || typeof value !== 'string') {
				return <span className='text-xs text-muted-foreground'>No value</span>
			}
			return (
				<img
					src={value}
					alt={property.name}
					className='max-h-48 max-w-full rounded-lg border border-border/60 object-contain'
					onError={e => {
						const target = e.target as HTMLImageElement
						target.style.display = 'none'
						target.insertAdjacentHTML(
							'afterend',
							'<span class="text-xs text-muted-foreground">Failed to load image</span>',
						)
					}}
				/>
			)
		}
		default: {
			// Only display primitive string values, not objects
			const displayValue = typeof value === 'string' ? value : null
			return displayValue ? (
				<Badge
					variant='secondary'
					className='rounded-full bg-muted/80 px-3 py-1 text-xs font-medium text-foreground'
				>
					{displayValue}
				</Badge>
			) : (
				<span className='text-xs text-muted-foreground'>No value</span>
			)
		}
	}
}
