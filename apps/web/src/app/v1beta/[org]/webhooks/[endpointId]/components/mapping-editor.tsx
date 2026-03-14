'use client'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { ArrowRight, Code2, Plus, Settings2, Trash2, Wand2 } from 'lucide-react'
import { useRouter } from 'next/navigation'
import { useCallback, useState, useTransition } from 'react'
import { toast } from 'sonner'
import { updateEndpointMapping } from '../../actions'

// Transform options available in PropertyMapping
const TRANSFORM_OPTIONS = [
	{ value: 'none', label: 'None', description: 'No transformation' },
	{
		value: 'split_comma',
		label: 'Split by Comma',
		description: 'Split string into array',
	},
	{
		value: 'lowercase',
		label: 'Lowercase',
		description: 'Convert to lowercase',
	},
	{
		value: 'uppercase',
		label: 'Uppercase',
		description: 'Convert to uppercase',
	},
	{
		value: 'slugify',
		label: 'Slugify',
		description: 'Convert to URL-safe slug',
	},
	{ value: 'parse_json', label: 'Parse JSON', description: 'Parse as JSON' },
	{
		value: 'to_date',
		label: 'To Date',
		description: 'Extract date from timestamp',
	},
	{
		value: 'to_time',
		label: 'To Time',
		description: 'Extract time from timestamp',
	},
	{
		value: 'to_bool',
		label: 'To Boolean',
		description: 'Convert to boolean',
	},
	{
		value: 'to_number',
		label: 'To Number',
		description: 'Convert to number',
	},
	{ value: 'trim', label: 'Trim', description: 'Trim whitespace' },
	{
		value: 'cents_to_dollars',
		label: 'Cents to Dollars',
		description: 'Divide by 100',
	},
	{
		value: 'unix_to_iso',
		label: 'Unix to ISO',
		description: 'Convert Unix timestamp to ISO date',
	},
] as const

interface FieldMapping {
	source_field: string
	target_property: string
	transform?:
		| string
		| { split?: { delimiter: string } }
		| { regex?: { pattern: string; replacement: string } }
}

interface ComputedMapping {
	target_property: string
	expression: string
}

interface PropertyMapping {
	target_repository_id?: string | null
	static_mappings: FieldMapping[]
	computed_mappings: ComputedMapping[]
	defaults: Record<string, unknown>
}

interface MappingEditorProps {
	endpointId: string
	initialMapping: string | null
	provider: string
	operatorId?: string
}

const PROVIDER_EXAMPLES: Record<string, PropertyMapping> = {
	github: {
		static_mappings: [
			{ source_field: 'frontmatter.title', target_property: 'title' },
			{
				source_field: 'frontmatter.description',
				target_property: 'description',
			},
			{
				source_field: 'frontmatter.tags',
				target_property: 'tags',
				transform: 'split_comma',
			},
			{ source_field: 'frontmatter.author', target_property: 'author' },
			{ source_field: 'path', target_property: 'source_path' },
		],
		computed_mappings: [
			{
				target_property: 'slug',
				expression: 'slugify(source.frontmatter.title)',
			},
		],
		defaults: { status: 'draft' },
	},
	linear: {
		static_mappings: [
			{ source_field: 'identifier', target_property: 'issue_id' },
			{ source_field: 'title', target_property: 'title' },
			{ source_field: 'description', target_property: 'description' },
			{ source_field: 'state.name', target_property: 'status' },
			{ source_field: 'priority', target_property: 'priority' },
			{ source_field: 'assignee.name', target_property: 'assignee' },
		],
		computed_mappings: [],
		defaults: {},
	},
	hubspot: {
		static_mappings: [
			{ source_field: 'properties.firstname', target_property: 'first_name' },
			{ source_field: 'properties.lastname', target_property: 'last_name' },
			{ source_field: 'properties.email', target_property: 'email' },
			{ source_field: 'properties.company', target_property: 'company' },
			{ source_field: 'properties.phone', target_property: 'phone' },
		],
		computed_mappings: [],
		defaults: { source: 'hubspot' },
	},
	stripe: {
		static_mappings: [
			{ source_field: 'name', target_property: 'name' },
			{ source_field: 'description', target_property: 'description' },
			{
				source_field: 'unit_amount',
				target_property: 'price',
				transform: 'cents_to_dollars',
			},
			{
				source_field: 'currency',
				target_property: 'currency',
				transform: 'uppercase',
			},
			{ source_field: 'active', target_property: 'is_active' },
			{ source_field: 'metadata.category', target_property: 'category' },
		],
		computed_mappings: [],
		defaults: { source: 'stripe' },
	},
}

function getTransformValue(transform: FieldMapping['transform']): string {
	if (!transform) return 'none'
	if (typeof transform === 'string') return transform
	if ('split' in transform) return 'split'
	if ('regex' in transform) return 'regex'
	return 'none'
}

function buildTransform(value: string): FieldMapping['transform'] {
	if (value === 'none') return undefined
	return value
}

export function MappingEditor({
	endpointId,
	initialMapping,
	provider,
	operatorId,
}: MappingEditorProps) {
	const router = useRouter()
	const [isPending, startTransition] = useTransition()
	const [mode, setMode] = useState<'visual' | 'json'>('visual')

	// Parse initial mapping or use empty default
	const parseMapping = useCallback((): PropertyMapping => {
		if (!initialMapping) {
			return {
				static_mappings: [],
				computed_mappings: [],
				defaults: {},
			}
		}
		try {
			return JSON.parse(initialMapping)
		} catch {
			return {
				static_mappings: [],
				computed_mappings: [],
				defaults: {},
			}
		}
	}, [initialMapping])

	const [mapping, setMapping] = useState<PropertyMapping>(parseMapping)
	const [jsonValue, setJsonValue] = useState(
		initialMapping ? JSON.stringify(JSON.parse(initialMapping), null, 2) : '{}',
	)
	const [jsonError, setJsonError] = useState<string | null>(null)

	// Sync visual to JSON when switching tabs
	const handleModeChange = (newMode: string) => {
		if (newMode === 'json' && mode === 'visual') {
			setJsonValue(JSON.stringify(mapping, null, 2))
			setJsonError(null)
		} else if (newMode === 'visual' && mode === 'json') {
			try {
				const parsed = JSON.parse(jsonValue)
				setMapping(parsed)
				setJsonError(null)
			} catch (e) {
				setJsonError(
					'Invalid JSON. Fix errors before switching to visual mode.',
				)
				return
			}
		}
		setMode(newMode as 'visual' | 'json')
	}

	// Add static mapping
	const addStaticMapping = () => {
		setMapping(prev => ({
			...prev,
			static_mappings: [
				...prev.static_mappings,
				{ source_field: '', target_property: '' },
			],
		}))
	}

	// Update static mapping
	const updateStaticMapping = (
		index: number,
		field: keyof FieldMapping,
		value: string,
	) => {
		setMapping(prev => ({
			...prev,
			static_mappings: prev.static_mappings.map((m, i) =>
				i === index
					? {
							...m,
							[field]: field === 'transform' ? buildTransform(value) : value,
						}
					: m,
			),
		}))
	}

	// Remove static mapping
	const removeStaticMapping = (index: number) => {
		setMapping(prev => ({
			...prev,
			static_mappings: prev.static_mappings.filter((_, i) => i !== index),
		}))
	}

	// Add computed mapping
	const addComputedMapping = () => {
		setMapping(prev => ({
			...prev,
			computed_mappings: [
				...prev.computed_mappings,
				{ target_property: '', expression: '' },
			],
		}))
	}

	// Update computed mapping
	const updateComputedMapping = (
		index: number,
		field: keyof ComputedMapping,
		value: string,
	) => {
		setMapping(prev => ({
			...prev,
			computed_mappings: prev.computed_mappings.map((m, i) =>
				i === index ? { ...m, [field]: value } : m,
			),
		}))
	}

	// Remove computed mapping
	const removeComputedMapping = (index: number) => {
		setMapping(prev => ({
			...prev,
			computed_mappings: prev.computed_mappings.filter((_, i) => i !== index),
		}))
	}

	// Load example mapping for provider
	const loadExample = () => {
		const example = PROVIDER_EXAMPLES[provider.toLowerCase()]
		if (example) {
			setMapping(example)
			setJsonValue(JSON.stringify(example, null, 2))
			toast.success('Example mapping loaded')
		} else {
			toast.error('No example available for this provider')
		}
	}

	// Save mapping
	const handleSave = () => {
		let mappingToSave: PropertyMapping
		if (mode === 'json') {
			try {
				mappingToSave = JSON.parse(jsonValue)
			} catch {
				toast.error('Invalid JSON. Please fix errors before saving.')
				return
			}
		} else {
			mappingToSave = mapping
		}

		// Filter out empty mappings
		const cleanedMapping: PropertyMapping = {
			...mappingToSave,
			static_mappings: mappingToSave.static_mappings.filter(
				m => m.source_field && m.target_property,
			),
			computed_mappings: mappingToSave.computed_mappings.filter(
				m => m.target_property && m.expression,
			),
		}

		const mappingJson =
			cleanedMapping.static_mappings.length === 0 &&
			cleanedMapping.computed_mappings.length === 0 &&
			Object.keys(cleanedMapping.defaults || {}).length === 0
				? null
				: JSON.stringify(cleanedMapping)

		startTransition(async () => {
			const result = await updateEndpointMapping({
				endpointId,
				mapping: mappingJson,
				operatorId,
			})
			if (result.error) {
				toast.error(result.error)
			} else {
				toast.success('Mapping saved successfully')
				router.refresh()
			}
		})
	}

	// Reset to initial
	const handleReset = () => {
		setMapping(parseMapping())
		setJsonValue(
			initialMapping
				? JSON.stringify(JSON.parse(initialMapping), null, 2)
				: '{}',
		)
		setJsonError(null)
	}

	return (
		<Card>
			<CardHeader>
				<div className='flex items-center justify-between'>
					<div>
						<CardTitle className='flex items-center gap-2'>
							<Settings2 className='h-5 w-5' />
							Property Mapping
						</CardTitle>
						<CardDescription>
							Configure how external data maps to Library properties
						</CardDescription>
					</div>
					<Button variant='outline' size='sm' onClick={loadExample}>
						<Wand2 className='mr-2 h-4 w-4' />
						Load Example
					</Button>
				</div>
			</CardHeader>
			<CardContent className='space-y-4'>
				<Tabs value={mode} onValueChange={handleModeChange}>
					<TabsList>
						<TabsTrigger value='visual'>
							<Settings2 className='mr-2 h-4 w-4' />
							Visual Editor
						</TabsTrigger>
						<TabsTrigger value='json'>
							<Code2 className='mr-2 h-4 w-4' />
							JSON Editor
						</TabsTrigger>
					</TabsList>

					<TabsContent value='visual' className='space-y-6'>
						{/* Static Mappings */}
						<div className='space-y-4'>
							<div className='flex items-center justify-between'>
								<Label className='text-base font-semibold'>
									Field Mappings
								</Label>
								<Button variant='outline' size='sm' onClick={addStaticMapping}>
									<Plus className='mr-2 h-4 w-4' />
									Add Field
								</Button>
							</div>

							{mapping.static_mappings.length === 0 ? (
								<p className='text-sm text-muted-foreground py-4 text-center border border-dashed rounded-md'>
									No field mappings defined. Click &quot;Add Field&quot; to
									create one.
								</p>
							) : (
								<div className='space-y-3'>
									{mapping.static_mappings.map((m, index) => (
										<div
											key={index}
											className='flex items-center gap-2 p-3 border rounded-md bg-muted/30'
										>
											<div className='flex-1'>
												<Input
													placeholder='Source field (e.g., frontmatter.title)'
													value={m.source_field}
													onChange={e =>
														updateStaticMapping(
															index,
															'source_field',
															e.target.value,
														)
													}
												/>
											</div>
											<ArrowRight className='h-4 w-4 text-muted-foreground flex-shrink-0' />
											<div className='flex-1'>
												<Input
													placeholder='Target property'
													value={m.target_property}
													onChange={e =>
														updateStaticMapping(
															index,
															'target_property',
															e.target.value,
														)
													}
												/>
											</div>
											<Select
												value={getTransformValue(m.transform)}
												onValueChange={value =>
													updateStaticMapping(index, 'transform', value)
												}
											>
												<SelectTrigger className='w-[160px]'>
													<SelectValue placeholder='Transform' />
												</SelectTrigger>
												<SelectContent>
													{TRANSFORM_OPTIONS.map(opt => (
														<SelectItem key={opt.value} value={opt.value}>
															{opt.label}
														</SelectItem>
													))}
												</SelectContent>
											</Select>
											<Button
												variant='ghost'
												size='icon'
												onClick={() => removeStaticMapping(index)}
											>
												<Trash2 className='h-4 w-4 text-destructive' />
											</Button>
										</div>
									))}
								</div>
							)}
						</div>

						{/* Computed Mappings */}
						<div className='space-y-4'>
							<div className='flex items-center justify-between'>
								<div>
									<Label className='text-base font-semibold'>
										Computed Mappings
									</Label>
									<p className='text-xs text-muted-foreground'>
										Dynamic values computed from expressions
									</p>
								</div>
								<Button
									variant='outline'
									size='sm'
									onClick={addComputedMapping}
								>
									<Plus className='mr-2 h-4 w-4' />
									Add Computed
								</Button>
							</div>

							{mapping.computed_mappings.length === 0 ? (
								<p className='text-sm text-muted-foreground py-4 text-center border border-dashed rounded-md'>
									No computed mappings defined.
								</p>
							) : (
								<div className='space-y-3'>
									{mapping.computed_mappings.map((m, index) => (
										<div
											key={index}
											className='flex items-center gap-2 p-3 border rounded-md bg-muted/30'
										>
											<div className='flex-1'>
												<Input
													placeholder='Expression (e.g., slugify(source.title))'
													value={m.expression}
													onChange={e =>
														updateComputedMapping(
															index,
															'expression',
															e.target.value,
														)
													}
												/>
											</div>
											<ArrowRight className='h-4 w-4 text-muted-foreground flex-shrink-0' />
											<div className='w-[200px]'>
												<Input
													placeholder='Target property'
													value={m.target_property}
													onChange={e =>
														updateComputedMapping(
															index,
															'target_property',
															e.target.value,
														)
													}
												/>
											</div>
											<Button
												variant='ghost'
												size='icon'
												onClick={() => removeComputedMapping(index)}
											>
												<Trash2 className='h-4 w-4 text-destructive' />
											</Button>
										</div>
									))}
								</div>
							)}
						</div>

						{/* Defaults Preview */}
						{Object.keys(mapping.defaults || {}).length > 0 && (
							<div className='space-y-2'>
								<Label className='text-base font-semibold'>
									Default Values
								</Label>
								<div className='flex flex-wrap gap-2'>
									{Object.entries(mapping.defaults).map(([key, value]) => (
										<Badge key={key} variant='secondary'>
											{key}: {JSON.stringify(value)}
										</Badge>
									))}
								</div>
							</div>
						)}
					</TabsContent>

					<TabsContent value='json'>
						<div className='space-y-2'>
							<Textarea
								className='font-mono text-sm min-h-[400px]'
								value={jsonValue}
								onChange={e => {
									setJsonValue(e.target.value)
									try {
										JSON.parse(e.target.value)
										setJsonError(null)
									} catch {
										setJsonError('Invalid JSON syntax')
									}
								}}
								placeholder='Enter JSON mapping configuration...'
							/>
							{jsonError && (
								<p className='text-sm text-destructive'>{jsonError}</p>
							)}
							<p className='text-xs text-muted-foreground'>
								Edit the raw JSON mapping configuration. Changes will be
								validated when you save.
							</p>
						</div>
					</TabsContent>
				</Tabs>

				{/* Actions */}
				<div className='flex justify-end gap-2 pt-4 border-t'>
					<Button variant='outline' onClick={handleReset} disabled={isPending}>
						Reset
					</Button>
					<Button onClick={handleSave} disabled={isPending}>
						{isPending ? 'Saving...' : 'Save Mapping'}
					</Button>
				</div>
			</CardContent>
		</Card>
	)
}
