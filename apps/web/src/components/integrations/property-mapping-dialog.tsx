'use client'

import { useState } from 'react'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { Trash2, Plus } from 'lucide-react'
import { toast } from 'sonner'

interface FieldMapping {
	sourceField: string
	targetProperty: string
	transform?: string
}

interface PropertyMappingDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	endpointId: string
	provider: 'GITHUB' | 'LINEAR' | 'NOTION' | 'STRIPE'
	currentMapping?: FieldMapping[]
	onSave?: (mappings: FieldMapping[]) => void
}

const LINEAR_SOURCE_FIELDS = [
	{ value: 'id', label: 'Issue ID' },
	{ value: 'identifier', label: 'Identifier (LIN-123)' },
	{ value: 'title', label: 'Title' },
	{ value: 'description', label: 'Description' },
	{ value: 'state.name', label: 'Status' },
	{ value: 'assignee.name', label: 'Assignee' },
	{ value: 'priority', label: 'Priority' },
	{ value: 'estimate', label: 'Estimate' },
	{ value: 'due_date', label: 'Due Date' },
	{ value: 'team.name', label: 'Team' },
	{ value: 'project.name', label: 'Project' },
	{ value: 'labels', label: 'Labels' },
	{ value: 'created_at', label: 'Created At' },
	{ value: 'updated_at', label: 'Updated At' },
]

const GITHUB_SOURCE_FIELDS = [
	{ value: 'frontmatter.title', label: 'Frontmatter Title' },
	{ value: 'frontmatter.description', label: 'Frontmatter Description' },
	{ value: 'frontmatter.tags', label: 'Frontmatter Tags' },
	{ value: 'body', label: 'Markdown Body' },
	{ value: 'path', label: 'File Path' },
]

// Common target properties (could be fetched from repository schema)
const COMMON_TARGET_PROPERTIES = [
	{ value: 'title', label: 'Title' },
	{ value: 'description', label: 'Description' },
	{ value: 'status', label: 'Status' },
	{ value: 'assigned_to', label: 'Assigned To' },
	{ value: 'priority', label: 'Priority' },
	{ value: 'tags', label: 'Tags' },
	{ value: 'due_date', label: 'Due Date' },
	{ value: 'team', label: 'Team' },
	{ value: 'project', label: 'Project' },
]

export function PropertyMappingDialog({
	open,
	onOpenChange,
	endpointId,
	provider,
	currentMapping,
	onSave,
}: PropertyMappingDialogProps) {
	const [mappings, setMappings] = useState<FieldMapping[]>(
		currentMapping || [{ sourceField: '', targetProperty: '' }],
	)

	const sourceFields =
		provider === 'LINEAR' ? LINEAR_SOURCE_FIELDS : GITHUB_SOURCE_FIELDS

	const handleAddMapping = () => {
		setMappings([...mappings, { sourceField: '', targetProperty: '' }])
	}

	const handleRemoveMapping = (index: number) => {
		setMappings(mappings.filter((_, i) => i !== index))
	}

	const handleUpdateMapping = (
		index: number,
		field: 'sourceField' | 'targetProperty',
		value: string,
	) => {
		const updated = [...mappings]
		updated[index][field] = value
		setMappings(updated)
	}

	const handleSave = () => {
		// Validate
		const validMappings = mappings.filter(
			m => m.sourceField && m.targetProperty,
		)

		if (validMappings.length === 0) {
			toast.error('Please add at least one mapping')
			return
		}

		// TODO: Call GraphQL mutation to update webhook endpoint mapping
		onSave?.(validMappings)
		toast.success('Property mapping saved')
		onOpenChange(false)
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className='max-w-2xl max-h-[80vh] overflow-y-auto'>
				<DialogHeader>
					<DialogTitle>Property Mapping Configuration</DialogTitle>
					<DialogDescription>
						Map {provider} fields to repository properties. Data will be
						automatically transformed during sync.
					</DialogDescription>
				</DialogHeader>

				<div className='space-y-4 py-4'>
					<div className='grid grid-cols-[1fr_auto_1fr_auto] gap-2 items-center text-sm font-medium text-muted-foreground mb-2'>
						<div>{provider} Field</div>
						<div />
						<div>Repository Property</div>
						<div />
					</div>

					{mappings.map((mapping, idx) => (
						<div
							key={idx}
							className='grid grid-cols-[1fr_auto_1fr_auto] gap-2 items-center'
						>
							<Select
								value={mapping.sourceField}
								onValueChange={value =>
									handleUpdateMapping(idx, 'sourceField', value)
								}
							>
								<SelectTrigger>
									<SelectValue placeholder='Select source field' />
								</SelectTrigger>
								<SelectContent>
									{sourceFields.map(field => (
										<SelectItem key={field.value} value={field.value}>
											{field.label}
										</SelectItem>
									))}
								</SelectContent>
							</Select>

							<div className='text-muted-foreground'>→</div>

							<Select
								value={mapping.targetProperty}
								onValueChange={value =>
									handleUpdateMapping(idx, 'targetProperty', value)
								}
							>
								<SelectTrigger>
									<SelectValue placeholder='Select target property' />
								</SelectTrigger>
								<SelectContent>
									{COMMON_TARGET_PROPERTIES.map(prop => (
										<SelectItem key={prop.value} value={prop.value}>
											{prop.label}
										</SelectItem>
									))}
								</SelectContent>
							</Select>

							<Button
								variant='ghost'
								size='sm'
								onClick={() => handleRemoveMapping(idx)}
								disabled={mappings.length === 1}
							>
								<Trash2 className='h-4 w-4' />
							</Button>
						</div>
					))}

					<Button
						variant='outline'
						size='sm'
						onClick={handleAddMapping}
						className='w-full'
					>
						<Plus className='mr-2 h-4 w-4' />
						Add Mapping
					</Button>

					<div className='mt-4 p-4 bg-muted rounded-lg text-sm'>
						<p className='font-medium mb-2'>
							Default Mappings (if none configured):
						</p>
						<ul className='space-y-1 text-muted-foreground'>
							<li>
								•{' '}
								{provider === 'LINEAR'
									? 'title → title'
									: 'frontmatter.title → title'}
							</li>
							<li>
								•{' '}
								{provider === 'LINEAR'
									? 'description → description'
									: 'body → description'}
							</li>
							<li>
								•{' '}
								{provider === 'LINEAR'
									? 'state.name → status'
									: 'frontmatter.status → status'}
							</li>
							{provider === 'LINEAR' && (
								<>
									<li>• assignee.name → assigned_to</li>
									<li>• priority → priority</li>
								</>
							)}
						</ul>
					</div>
				</div>

				<DialogFooter>
					<Button variant='outline' onClick={() => onOpenChange(false)}>
						Cancel
					</Button>
					<Button onClick={handleSave}>Save Mapping</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
