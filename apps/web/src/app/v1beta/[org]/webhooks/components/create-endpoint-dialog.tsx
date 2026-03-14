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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { useRouter } from 'next/navigation'
import { useState, useTransition } from 'react'
import { createWebhookEndpoint } from '../actions'

interface CreateEndpointDialogProps {
	tenantId: string
	open: boolean
	onOpenChange: (open: boolean) => void
}

const providers = [
	{ value: 'GITHUB', label: 'GitHub' },
	{ value: 'LINEAR', label: 'Linear' },
	{ value: 'HUBSPOT', label: 'HubSpot' },
	{ value: 'STRIPE', label: 'Stripe' },
	{ value: 'NOTION', label: 'Notion' },
	{ value: 'AIRTABLE', label: 'Airtable' },
	{ value: 'GENERIC', label: 'Generic' },
]

const providerConfigs: Record<
	string,
	{
		fields: {
			name: string
			label: string
			required: boolean
			placeholder?: string
		}[]
	}
> = {
	GITHUB: {
		fields: [
			{
				name: 'repository',
				label: 'Repository (owner/repo)',
				required: true,
				placeholder: 'owner/repo',
			},
			{ name: 'branch', label: 'Branch', required: true, placeholder: 'main' },
			{
				name: 'path_pattern',
				label: 'Path Pattern (optional)',
				required: false,
				placeholder: 'docs/**/*.md',
			},
		],
	},
	LINEAR: {
		fields: [
			{ name: 'team_id', label: 'Team ID (optional)', required: false },
			{ name: 'project_id', label: 'Project ID (optional)', required: false },
		],
	},
	HUBSPOT: {
		fields: [
			{ name: 'portal_id', label: 'Portal ID', required: true },
			{
				name: 'object_types',
				label: 'Object Types (comma-separated)',
				required: false,
				placeholder: 'contact,deal,company',
			},
		],
	},
	STRIPE: {
		fields: [
			{
				name: 'sync_objects',
				label: 'Sync Objects (comma-separated)',
				required: false,
				placeholder: 'product,price',
			},
			{
				name: 'fetch_latest',
				label: 'Fetch Latest on Update',
				required: false,
			},
		],
	},
	NOTION: {
		fields: [{ name: 'database_id', label: 'Database ID', required: true }],
	},
	AIRTABLE: {
		fields: [
			{ name: 'base_id', label: 'Base ID', required: true },
			{ name: 'table_id', label: 'Table ID', required: true },
		],
	},
	GENERIC: {
		fields: [
			{
				name: 'content_type',
				label: 'Content Type',
				required: false,
				placeholder: 'application/json',
			},
		],
	},
}

const providerEvents: Record<string, string[]> = {
	GITHUB: ['push', 'pull_request', 'release', 'issues'],
	LINEAR: ['Issue', 'Project', 'Comment', 'Cycle'],
	HUBSPOT: [
		'contact.creation',
		'contact.propertyChange',
		'contact.deletion',
		'deal.creation',
		'deal.propertyChange',
		'company.creation',
	],
	STRIPE: [
		'product.created',
		'product.updated',
		'product.deleted',
		'price.created',
		'price.updated',
	],
	NOTION: ['page.created', 'page.updated', 'page.deleted'],
	AIRTABLE: ['record.created', 'record.changed', 'record.deleted'],
	GENERIC: [],
}

export function CreateEndpointDialog({
	tenantId,
	open,
	onOpenChange,
}: CreateEndpointDialogProps) {
	const router = useRouter()
	const [isPending, startTransition] = useTransition()
	const [name, setName] = useState('')
	const [provider, setProvider] = useState<string>('')
	const [config, setConfig] = useState<Record<string, string>>({})
	const [selectedEvents, setSelectedEvents] = useState<string[]>([])
	const [error, setError] = useState<string | null>(null)
	const [createdEndpoint, setCreatedEndpoint] = useState<{
		webhookUrl: string
		secret: string
	} | null>(null)

	const handleProviderChange = (value: string) => {
		setProvider(value)
		setConfig({})
		setSelectedEvents([])
	}

	const handleConfigChange = (fieldName: string, value: string) => {
		setConfig(prev => ({ ...prev, [fieldName]: value }))
	}

	const handleEventToggle = (event: string) => {
		setSelectedEvents(prev =>
			prev.includes(event) ? prev.filter(e => e !== event) : [...prev, event],
		)
	}

	const handleSubmit = () => {
		if (!name || !provider) {
			setError('Name and provider are required')
			return
		}

		const configFields = providerConfigs[provider]?.fields || []
		for (const field of configFields) {
			if (field.required && !config[field.name]) {
				setError(`${field.label} is required`)
				return
			}
		}

		setError(null)

		startTransition(async () => {
			try {
				const result = await createWebhookEndpoint({
					tenantId,
					name,
					provider,
					config: JSON.stringify(buildProviderConfig(provider, config)),
					events: selectedEvents,
				})

				if (result.error) {
					setError(result.error)
				} else if (result.data) {
					setCreatedEndpoint({
						webhookUrl: result.data.webhookUrl,
						secret: result.data.secret,
					})
				}
			} catch (e) {
				setError('Failed to create endpoint')
			}
		})
	}

	const buildProviderConfig = (
		provider: string,
		config: Record<string, string>,
	) => {
		switch (provider) {
			case 'GITHUB':
				return {
					github: {
						repository: config.repository,
						branch: config.branch || 'main',
						path_pattern: config.path_pattern || null,
					},
				}
			case 'LINEAR':
				return {
					linear: {
						team_id: config.team_id || null,
						project_id: config.project_id || null,
					},
				}
			case 'HUBSPOT':
				return {
					hubspot: {
						portal_id: config.portal_id,
						object_types: config.object_types
							? config.object_types.split(',').map(s => s.trim())
							: [],
					},
				}
			case 'STRIPE':
				return {
					stripe: {
						sync_objects: config.sync_objects
							? config.sync_objects.split(',').map(s => s.trim())
							: [],
						fetch_latest: config.fetch_latest === 'true',
					},
				}
			case 'NOTION':
				return {
					notion: {
						database_id: config.database_id,
					},
				}
			case 'AIRTABLE':
				return {
					airtable: {
						base_id: config.base_id,
						table_id: config.table_id,
					},
				}
			default:
				return {
					generic: {
						content_type: config.content_type || 'application/json',
					},
				}
		}
	}

	const handleClose = () => {
		if (createdEndpoint) {
			router.refresh()
		}
		setName('')
		setProvider('')
		setConfig({})
		setSelectedEvents([])
		setError(null)
		setCreatedEndpoint(null)
		onOpenChange(false)
	}

	if (createdEndpoint) {
		return (
			<Dialog open={open} onOpenChange={handleClose}>
				<DialogContent className='sm:max-w-[500px]'>
					<DialogHeader>
						<DialogTitle>Endpoint Created Successfully</DialogTitle>
						<DialogDescription>
							Copy the webhook URL and secret below. The secret will only be
							shown once.
						</DialogDescription>
					</DialogHeader>
					<div className='space-y-4 py-4'>
						<div className='space-y-2'>
							<Label>Webhook URL</Label>
							<div className='flex gap-2'>
								<Input
									value={createdEndpoint.webhookUrl}
									readOnly
									className='font-mono text-sm'
								/>
								<Button
									variant='outline'
									onClick={() =>
										navigator.clipboard.writeText(createdEndpoint.webhookUrl)
									}
								>
									Copy
								</Button>
							</div>
						</div>
						<div className='space-y-2'>
							<Label>Webhook Secret</Label>
							<div className='flex gap-2'>
								<Input
									value={createdEndpoint.secret}
									readOnly
									className='font-mono text-sm'
								/>
								<Button
									variant='outline'
									onClick={() =>
										navigator.clipboard.writeText(createdEndpoint.secret)
									}
								>
									Copy
								</Button>
							</div>
							<p className='text-sm text-destructive'>
								⚠️ Save this secret now. It won&apos;t be shown again.
							</p>
						</div>
					</div>
					<DialogFooter>
						<Button onClick={handleClose}>Done</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		)
	}

	return (
		<Dialog open={open} onOpenChange={handleClose}>
			<DialogContent className='sm:max-w-[500px] max-h-[90vh] overflow-y-auto'>
				<DialogHeader>
					<DialogTitle>Create Webhook Endpoint</DialogTitle>
					<DialogDescription>
						Set up a new webhook endpoint to receive data from external
						services.
					</DialogDescription>
				</DialogHeader>
				<div className='space-y-4 py-4'>
					<div className='space-y-2'>
						<Label htmlFor='name'>Name</Label>
						<Input
							id='name'
							value={name}
							onChange={e => setName(e.target.value)}
							placeholder='My GitHub Docs Sync'
						/>
					</div>

					<div className='space-y-2'>
						<Label htmlFor='provider'>Provider</Label>
						<Select value={provider} onValueChange={handleProviderChange}>
							<SelectTrigger>
								<SelectValue placeholder='Select a provider' />
							</SelectTrigger>
							<SelectContent>
								{providers.map(p => (
									<SelectItem key={p.value} value={p.value}>
										{p.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>

					{provider && providerConfigs[provider] && (
						<>
							<div className='border-t pt-4'>
								<h4 className='text-sm font-medium mb-3'>
									{providerConfigs[provider].fields.length > 0
										? 'Provider Configuration'
										: ''}
								</h4>
								{providerConfigs[provider].fields.map(field => (
									<div key={field.name} className='space-y-2 mb-3'>
										<Label htmlFor={field.name}>
											{field.label}
											{field.required && (
												<span className='text-destructive'>*</span>
											)}
										</Label>
										<Input
											id={field.name}
											value={config[field.name] || ''}
											onChange={e =>
												handleConfigChange(field.name, e.target.value)
											}
											placeholder={field.placeholder}
										/>
									</div>
								))}
							</div>

							{providerEvents[provider] &&
								providerEvents[provider].length > 0 && (
									<div className='border-t pt-4'>
										<h4 className='text-sm font-medium mb-3'>
											Events (optional)
										</h4>
										<p className='text-sm text-muted-foreground mb-2'>
											Leave empty to receive all events
										</p>
										<div className='flex flex-wrap gap-2'>
											{providerEvents[provider].map(event => (
												<Button
													key={event}
													type='button'
													variant={
														selectedEvents.includes(event)
															? 'default'
															: 'outline'
													}
													size='sm'
													onClick={() => handleEventToggle(event)}
												>
													{event}
												</Button>
											))}
										</div>
									</div>
								)}
						</>
					)}

					{error && <p className='text-sm text-destructive'>{error}</p>}
				</div>
				<DialogFooter>
					<Button variant='outline' onClick={handleClose}>
						Cancel
					</Button>
					<Button onClick={handleSubmit} disabled={isPending}>
						{isPending ? 'Creating...' : 'Create Endpoint'}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
