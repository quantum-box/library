import { parseFormula, formatFormula } from '@/lib/numeric-property'

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
import { useEffect, useMemo } from 'react'
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
	properties?: PropertyForPropertiesUiFragment[]
	onClose: () => void
	editingProperty: PropertyForPropertiesUiFragment | null
	databases?: DatabaseConfig[]
	onSave: (property: PropertyForPropertiesUiFragment) => void
}

export const propertyFormSchema = z.object({
	property_name: z.string().min(1, 'Property key is required'),
	display_name: z
		.string()
		.trim()
		.min(1, 'Display name is required')
		.max(255, 'Display name must be at most 255 characters'),
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
	precision: z.number().int().min(1).max(28).default(28),
	scale: z.number().int().min(0).max(28).default(12),
	unit: z.string().default('g'),
	expression: z.string().default('0'),
	relation: z.string().default(''),
	target: z.string().default(''),
	aggregation: z.string().default('sum'),
	relatedDatabase: z.string().optional(),
	searchTerm: z.string().default(''),
})

export type PropertyFormValues = z.infer<typeof propertyFormSchema>

export function PropertyDialog({
	isOpen,
	properties = [],
	onClose,
	editingProperty,
	databases,
	onSave,
}: PropertyDialogProps) {
	const { t } = useTranslation()
	const validationSchema = useMemo(
		() =>
			propertyFormSchema.superRefine((values, context) => {
				if (values.property_name === editingProperty?.name) return
				if (!/^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(values.property_name)) {
					context.addIssue({
						code: z.ZodIssueCode.custom,
						path: ['property_name'],
						message: 'Start with an ASCII letter; use ASCII letters, digits, underscores, or hyphens (max 64 characters)',
					})
					return
				}
				if (values.property_name.startsWith('ext_')) {
					context.addIssue({
						code: z.ZodIssueCode.custom,
						path: ['property_name'],
						message: 'Property names starting with "ext_" are reserved for system extensions',
					})
				}
			}),
		[editingProperty?.name],
	)
	const form = useForm<PropertyFormValues>({
		resolver: zodResolver(validationSchema),
		reValidateMode: 'onChange',
		defaultValues: {
			property_name: '',
			display_name: '',
			type: PropertyType.String,
			options: [],
			relatedDatabase: '',
			precision: 28, scale: 12, unit: 'g', expression: '0', relation: '', target: '', aggregation: 'sum',
			searchTerm: '',
		},
	})

	useEffect(() => {
		if (editingProperty) {
			form.reset({
				property_name: editingProperty.name,
				display_name: editingProperty.displayName ?? editingProperty.name,
				type: editingProperty.typ,
				options: (
					editingProperty.meta as SelectTypeMetaForPropertiesUiFragment
				)?.options?.map(o => ({ name: o.name, key: o.key })),
				relatedDatabase:
					(editingProperty.meta as RelationTypeMetaForPropertiesUiFragment)
						?.databaseId || '',
				...numericFormDefaults(editingProperty, properties),
				searchTerm: '',
			})
		} else {
			form.reset()
		}
	}, [editingProperty, form])

	const onSubmit = (values: PropertyFormValues) => {
		const newProperty: PropertyForPropertiesUiFragment = {
			id: editingProperty?.id || crypto.randomUUID(),
			name:
				values.property_name === editingProperty?.name
					? editingProperty.name
					: values.property_name.trim(),
			displayName: values.display_name.trim(),
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
        if ([PropertyType.Decimal, PropertyType.Quantity, PropertyType.Formula, PropertyType.Rollup].includes(values.type)) {
            try {
                const decimal = { precision: values.precision, scale: values.scale }
                if (values.scale > values.precision) throw new Error('Decimal places must not exceed precision')
                let config: unknown = decimal
                if (values.type === PropertyType.Quantity) config = { decimal, allowed_units: ['g','kg'].includes(values.unit) ? ['g','kg'] : ['mL','L'].includes(values.unit) ? ['mL','L'] : ['count'], default_unit: values.unit, unit_version: 1 }
                if (values.type === PropertyType.Formula) config = { decimal, expression: parseFormula(values.expression, properties) }
                if (values.type === PropertyType.Rollup) {
                    if (!values.relation || !values.target) throw new Error('Choose a relation and a target property ID')
                    config = { decimal, relation_property_id: values.relation, target_property_id: values.target, function: values.aggregation }
                }
                newProperty.meta = { __typename: 'JsonType', json: JSON.stringify(config) }
            } catch (error) { form.setError('expression', { message: error instanceof Error ? error.message : 'Invalid numeric configuration' }); return }
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
                        {[PropertyType.Decimal, PropertyType.Quantity, PropertyType.Formula, PropertyType.Rollup].includes(form.watch('type')) && <div className="space-y-2">
                          <label htmlFor="numeric-precision">Precision<Input id="numeric-precision" type="number" value={form.watch('precision')} onChange={e => form.setValue('precision', Number(e.target.value))} /></label>
                          <label htmlFor="numeric-scale">Decimal places<Input id="numeric-scale" type="number" value={form.watch('scale')} onChange={e => form.setValue('scale', Number(e.target.value))} /></label>
                          {form.watch('type') === PropertyType.Quantity && <label>Unit<select value={form.watch('unit')} onChange={e => form.setValue('unit', e.target.value)}>{['g','kg','mL','L','count'].map(unit => <option key={unit}>{unit}</option>)}</select></label>}
                          {form.watch('type') === PropertyType.Formula && <label htmlFor="numeric-expression">Formula<Input id="numeric-expression" value={form.watch('expression')} onChange={e => form.setValue('expression', e.target.value)} placeholder="total_cost / servings" /><small>{properties.map(p => p.name).join(', ')}</small></label>}
                          {form.watch('type') === PropertyType.Rollup && <>
                            <label>Relation<select value={form.watch('relation')} onChange={e => form.setValue('relation', e.target.value)}><option value="">—</option>{properties.filter(p => p.typ === PropertyType.Relation).map(p => <option key={p.id} value={p.id}>{p.displayName}</option>)}</select></label>
                            <label htmlFor="numeric-target">Target property ID<Input id="numeric-target" value={form.watch('target')} onChange={e => form.setValue('target', e.target.value)} /></label>
                            <label>Aggregation<select value={form.watch('aggregation')} onChange={e => form.setValue('aggregation', e.target.value)}>{['sum','count','avg','min','max'].map(op => <option key={op}>{op}</option>)}</select></label>
                          </>}
                          {form.formState.errors.expression && <p role="alert">{form.formState.errors.expression.message}</p>}
                        </div>}
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
							name='display_name'
							render={({ field }) => (
								<FormItem>
									<FormLabel>{t.v1beta.properties.displayName}</FormLabel>
									<FormControl>
										<Input {...field} placeholder={t.v1beta.properties.displayName} />
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
											<SelectItem value={PropertyType.Decimal}>{PropertyType.Decimal}</SelectItem>
<SelectItem value={PropertyType.Quantity}>{PropertyType.Quantity}</SelectItem>
<SelectItem value={PropertyType.Formula}>{PropertyType.Formula}</SelectItem>
<SelectItem value={PropertyType.Rollup}>{PropertyType.Rollup}</SelectItem>
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

function numericFormDefaults(property: PropertyForPropertiesUiFragment, properties: PropertyForPropertiesUiFragment[]) {
    try {
        const config = JSON.parse((property.meta as { json?: string })?.json ?? '{}')
        return { precision: config.decimal?.precision ?? config.precision ?? 28, scale: config.decimal?.scale ?? config.scale ?? 12, unit: config.default_unit ?? 'g', expression: config.expression ? formatFormula(config.expression, properties) : '0', relation: config.relation_property_id ?? '', target: config.target_property_id ?? '', aggregation: config.function ?? 'sum' }
    } catch { return {} }
}
