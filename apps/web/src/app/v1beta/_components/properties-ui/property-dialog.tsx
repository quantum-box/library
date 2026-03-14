'use client'

import { Button } from '@/components/ui/button'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import {
	Form,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from '@/components/ui/tooltip'
import {
	PropertyForPropertiesUiFragment,
	PropertyType,
	RelationTypeMetaForPropertiesUiFragment,
	SelectTypeMetaForPropertiesUiFragment,
} from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import { Database, Info, Search } from 'lucide-react'
import { useEffect } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'
import { PropertyOptionsField } from './property-options-field'

export interface DatabaseConfig {
	id: string
	org: string
	repo: string
	items: { id: string; name: string }[]
}

interface PropertyDialogProps {
	isOpen: boolean
	onClose: () => void
	editingProperty: PropertyForPropertiesUiFragment | null
	databases?: DatabaseConfig[]
	onSave: (property: PropertyForPropertiesUiFragment) => void
}

export const propertyFormSchema = z.object({
	property_name: z
		.string()
		.min(1, 'Property name is required')
		.refine(
			name => !name.startsWith('ext_'),
			'Property names starting with "ext_" are reserved for system extensions',
		),
	type: z.nativeEnum(PropertyType),
	options: z
		.array(
			z.object({
				name: z
					.string()
					.min(1, 'Option name is required')
					.regex(/^[a-z][a-zA-Z0-9]*$/, 'Must be in camelCase'),
				key: z.string(),
			}),
		)
		.optional(),
	relatedDatabase: z.string().optional(),
	searchTerm: z.string().default(''),
})

export type PropertyFormValues = z.infer<typeof propertyFormSchema>

export function PropertyDialog({
	isOpen,
	onClose,
	editingProperty,
	databases,
	onSave,
}: PropertyDialogProps) {
	const { t } = useTranslation()
	const form = useForm<PropertyFormValues>({
		resolver: zodResolver(propertyFormSchema),
		reValidateMode: 'onChange',
		defaultValues: {
			property_name: '',
			type: PropertyType.String,
			options: [],
			relatedDatabase: '',
			searchTerm: '',
		},
	})

	useEffect(() => {
		if (editingProperty) {
			form.reset({
				property_name: editingProperty.name,
				type: editingProperty.typ,
				options: (
					editingProperty.meta as SelectTypeMetaForPropertiesUiFragment
				)?.options?.map(o => ({ name: o.name, key: o.key })),
				relatedDatabase:
					(editingProperty.meta as RelationTypeMetaForPropertiesUiFragment)
						?.databaseId || '',
				searchTerm: '',
			})
		} else {
			form.reset()
		}
	}, [editingProperty, form])

	const onSubmit = (values: PropertyFormValues) => {
		const newProperty: PropertyForPropertiesUiFragment = {
			id: editingProperty?.id || crypto.randomUUID(),
			name: values.property_name,
			typ: values.type,
			meta:
				values.type === PropertyType.Select
					? {
							__typename: 'SelectType',
							options:
								values.options?.map(o => ({
									__typename: 'SelectItem',
									id: o.key || crypto.randomUUID(),
									key: o.name,
									name: o.name,
								})) ?? [],
						}
					: values.type === PropertyType.MultiSelect
						? {
								__typename: 'MultiSelectType',
								options:
									values.options?.map(o => ({
										__typename: 'SelectItem',
										id: o.key || crypto.randomUUID(),
										key: o.name,
										name: o.name,
									})) ?? [],
							}
						: values.type === PropertyType.Relation
							? {
									__typename: 'RelationType',
									databaseId: values.relatedDatabase ?? '',
								}
							: null,
		}
		onSave(newProperty)
		form.reset()
		onClose()
	}

	const filteredDatabases = databases?.filter(db =>
		`${db.org} / ${db.repo}`
			.toLowerCase()
			.includes(form.watch('searchTerm')?.toLowerCase()),
	)

	return (
		<Dialog open={isOpen} onOpenChange={onClose}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						{editingProperty
							? t.v1beta.properties.editProperty
							: t.v1beta.properties.addNewProperty}
					</DialogTitle>
					<DialogDescription>
						{editingProperty
							? t.v1beta.properties.modifyPropertySettings
							: t.v1beta.properties.createNewPropertyDescription}
					</DialogDescription>
				</DialogHeader>
				<Form {...form}>
					<form onSubmit={form.handleSubmit(onSubmit)} className='space-y-4'>
						<FormField
							control={form.control}
							name='property_name'
							render={({ field }) => (
								<FormItem>
									<FormLabel>{t.v1beta.properties.propertyName}</FormLabel>
									<FormControl>
										<Input
											{...field}
											placeholder={t.v1beta.properties.propertyNamePlaceholder}
										/>
									</FormControl>
									<FormMessage />
								</FormItem>
							)}
						/>

						<FormField
							control={form.control}
							name='type'
							render={({ field }) => (
								<FormItem>
									<FormLabel>{t.v1beta.properties.propertyType}</FormLabel>
									<Select
										onValueChange={field.onChange}
										defaultValue={field.value}
									>
										<FormControl>
											<SelectTrigger>
												<SelectValue
													placeholder={t.v1beta.properties.selectPropertyType}
												/>
											</SelectTrigger>
										</FormControl>
										<SelectContent>
											<SelectItem value={PropertyType.String}>
												{PropertyType.String}
											</SelectItem>
											<SelectItem value={PropertyType.Integer}>
												{PropertyType.Integer}
											</SelectItem>
											<SelectItem value={PropertyType.Markdown}>
												Markdown
												<span className='ml-1 text-xs text-muted-foreground'>
													(rich text, code blocks)
												</span>
											</SelectItem>
											<SelectItem value={PropertyType.Image}>
												{PropertyType.Image}
											</SelectItem>
											<SelectItem value={PropertyType.Select}>
												{PropertyType.Select}
											</SelectItem>
											<SelectItem value={PropertyType.MultiSelect}>
												{PropertyType.MultiSelect}
											</SelectItem>
											<SelectItem value={PropertyType.Relation}>
												{PropertyType.Relation}
											</SelectItem>
											<SelectItem value={PropertyType.Location}>
												{PropertyType.Location}
											</SelectItem>
											<SelectItem value={PropertyType.Date}>
												{PropertyType.Date}
											</SelectItem>
										</SelectContent>
									</Select>
									<FormMessage />
								</FormItem>
							)}
						/>

						{(form.watch('type') === PropertyType.Select ||
							form.watch('type') === PropertyType.MultiSelect) && (
							<PropertyOptionsField control={form.control} />
						)}

						{form.watch('type') === PropertyType.Relation && (
							<FormField
								control={form.control}
								name='relatedDatabase'
								render={({ field }) => (
									<FormItem>
										<FormLabel>
											{t.v1beta.properties.relatedRepository}
										</FormLabel>
										<div className='space-y-2'>
											<FormControl>
												<div className='relative'>
													<Input
														value={form.watch('searchTerm')}
														onChange={e =>
															form.setValue('searchTerm', e.target.value)
														}
														placeholder={t.v1beta.properties.searchRepository}
														className='pr-10'
													/>
													<Search className='absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-400' />
												</div>
											</FormControl>
											<ScrollArea className='h-40'>
												{filteredDatabases?.map(db => (
													<TooltipProvider key={db.id}>
														<Tooltip>
															<TooltipTrigger asChild>
																{/* biome-ignore lint/a11y/useKeyWithClickEvents: <explanation> */}
																<div
																	id='database-option'
																	aria-selected={
																		`${db.org} / ${db.repo}` === field.value
																	}
																	className={`p-2 cursor-pointer hover:bg-gray-100 ${
																		`${db.org} / ${db.repo}` === field.value
																			? 'bg-blue-100'
																			: ''
																	}`}
																	onClick={() => field.onChange(db.id)}
																>
																	<Database className='w-4 h-4 inline-block mr-2' />
																	{db.org} / {db.repo}
																	<Info className='w-4 h-4 inline-block ml-2 text-gray-400' />
																</div>
															</TooltipTrigger>
															<TooltipContent side='right' sideOffset={10}>
																<div className='space-y-2'>
																	<h4 className='font-medium'>
																		{t.v1beta.properties.repositoryPreview}
																	</h4>
																	<ul className='list-disc pl-4'>
																		{db.items.map(item => (
																			<li key={item.id}>{item.name}</li>
																		))}
																	</ul>
																</div>
															</TooltipContent>
														</Tooltip>
													</TooltipProvider>
												))}
											</ScrollArea>
											<FormMessage />
										</div>
									</FormItem>
								)}
							/>
						)}

						<DialogFooter>
							<Button type='button' variant='outline' onClick={onClose}>
								{t.v1beta.properties.cancel}
							</Button>
							<Button type='submit'>
								{editingProperty
									? t.v1beta.properties.saveChanges
									: t.v1beta.properties.addProperty}
							</Button>
						</DialogFooter>
					</form>
				</Form>
			</DialogContent>
		</Dialog>
	)
}
