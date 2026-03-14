'use client'

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { toast } from '@/components/ui/use-toast'
import {
	MultiSelectTypeMetaForPropertiesUiFragment,
	PropertyForPropertiesUiFragment,
	PropertyType,
	RelationTypeMetaForPropertiesUiFragment,
	SelectTypeMetaForPropertiesUiFragment,
} from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Edit2, ExternalLink, Github, Info, Plus, X } from 'lucide-react'
import Link from 'next/link'
import { useState } from 'react'
import {
	GitHubRepoConfig,
	GitHubReposDisplay,
	GitHubReposEditorDialog,
} from './github-repos-editor'
import { DatabaseConfig, PropertyDialog } from './property-dialog'

/** Check if property is ext_github (GitHub sync configuration) */
function isExtGithubProperty(
	property: PropertyForPropertiesUiFragment,
): boolean {
	return property.name === 'ext_github'
}

/** Check if property is a system extension (starts with ext_) */
function isSystemExtension(property: PropertyForPropertiesUiFragment): boolean {
	return property.name.startsWith('ext_')
}

export interface PropertiesUiProps {
	databases?: DatabaseConfig[]
	properties: PropertyForPropertiesUiFragment[]
	onAddProperty?: (property: PropertyForPropertiesUiFragment) => void
	onUpdateProperty?: (property: PropertyForPropertiesUiFragment) => void
	onRemoveProperty?: (propertyId: string) => void
	/** Whether GitHub is connected for repository search */
	isGitHubConnected?: boolean
	/** Called when bulk sync is requested */
	onBulkSyncGitHub?: (
		repoConfigs: GitHubRepoConfig[],
		extGithubPropertyId: string,
	) => Promise<void>
	/** Total number of data items in this repository */
	totalDataCount?: number
	/** URL to the settings page for managing integrations */
	settingsUrl?: string
}

export function PropertiesUi({
	databases,
	properties,
	onAddProperty,
	onUpdateProperty,
	onRemoveProperty,
	isGitHubConnected = false,
	onBulkSyncGitHub,
	totalDataCount = 0,
	settingsUrl,
}: PropertiesUiProps) {
	const { t } = useTranslation()
	const [selectedProperties, setSelectedProperties] =
		useState<PropertyForPropertiesUiFragment[]>(properties)
	const [isAddEditPropertyDialogOpen, setIsAddEditPropertyDialogOpen] =
		useState(false)
	const [editingProperty, setEditingProperty] =
		useState<PropertyForPropertiesUiFragment | null>(null)
	const [searchTerm, setSearchTerm] = useState('')
	// State for ext_github_repos editor dialog
	const [isGitHubReposDialogOpen, setIsGitHubReposDialogOpen] = useState(false)
	const [editingGitHubReposProperty, setEditingGitHubReposProperty] =
		useState<PropertyForPropertiesUiFragment | null>(null)

	// Separate properties into user properties and system extensions
	const userProperties = selectedProperties.filter(p => !isSystemExtension(p))
	const systemExtensions = selectedProperties.filter(p => isSystemExtension(p))

	const handleSaveProperty = (property: PropertyForPropertiesUiFragment) => {
		if (editingProperty) {
			onUpdateProperty?.(property)
			setSelectedProperties(prev =>
				prev.map(p => (p.id === property.id ? property : p)),
			)
			toast({
				variant: 'success',
				title: t.v1beta.properties.propertyUpdated,
				description: t.v1beta.properties.propertyUpdatedDescription.replace(
					'{name}',
					property.name,
				),
			})
		} else {
			onAddProperty?.(property)
			setSelectedProperties(prev => [...prev, property])
			toast({
				variant: 'success',
				title: t.v1beta.properties.propertyAdded,
				description: t.v1beta.properties.propertyAddedDescription.replace(
					'{name}',
					property.name,
				),
			})
		}
	}

	const removeCustomProperty = (propertyId: string) => {
		const propertyToRemove = selectedProperties.find(p => p.id === propertyId)
		if (propertyToRemove && !isEssentialProperty(propertyToRemove)) {
			onRemoveProperty?.(propertyId)
			setSelectedProperties(prev => prev.filter(p => p.id !== propertyId))
			toast({
				variant: 'success',
				title: t.v1beta.properties.propertyRemoved,
				description: t.v1beta.properties.propertyRemovedDescription.replace(
					'{name}',
					propertyToRemove.name,
				),
			})
		} else {
			toast({
				title: t.v1beta.properties.cannotRemoveEssential,
				description: t.v1beta.properties.cannotRemoveEssentialDescription,
				variant: 'destructive',
			})
		}
	}

	const isEssentialProperty = (property: PropertyForPropertiesUiFragment) => {
		return [
			'id',
			'name',
			'createdAt',
			'updatedAt',
			'content',
			'ext_github',
		].some(essentialName => essentialName === property.name)
	}

	// Handle saving ext_github repos configuration
	const handleSaveGitHubRepos = (jsonValue: string) => {
		if (!editingGitHubReposProperty) return
		// Create an updated property with the JSON value stored in meta.json
		const updatedProperty: PropertyForPropertiesUiFragment = {
			...editingGitHubReposProperty,
			meta: {
				__typename: 'JsonType' as const,
				json: jsonValue,
			},
		}
		onUpdateProperty?.(updatedProperty)
		// Update local state to reflect the change
		setSelectedProperties(prev =>
			prev.map(p => (p.id === updatedProperty.id ? updatedProperty : p)),
		)
		toast({
			variant: 'success',
			title: t.v1beta.properties.githubReposUpdated,
			description: t.v1beta.properties.githubReposUpdatedDescription,
		})
	}

	const filteredUserProperties = userProperties.filter(prop =>
		prop.name?.toLowerCase().includes(searchTerm?.toLowerCase()),
	)

	return (
		<main className='flex-1 overflow-auto p-6'>
			<div className='max-w-4xl mx-auto space-y-8'>
				<h1 className='text-2xl font-bold'>
					{t.v1beta.properties.manageProperties}
				</h1>

				{/* User Properties Section */}
				<section>
					<div className='mb-4 flex justify-between items-center'>
						<div>
							<h2 className='text-lg font-semibold'>
								{t.v1beta.properties.userProperties}
							</h2>
							<p className='text-sm text-muted-foreground'>
								{t.v1beta.properties.userPropertiesDescription}
							</p>
						</div>
						<div className='flex gap-2'>
							<Input
								className='w-64'
								placeholder={t.v1beta.properties.searchProperties}
								value={searchTerm}
								onChange={e => setSearchTerm(e.target.value)}
							/>
							<Button
								onClick={() => {
									setIsAddEditPropertyDialogOpen(true)
								}}
							>
								<Plus className='w-4 h-4 mr-2' />
								{t.v1beta.properties.addNewProperty}
							</Button>
						</div>
					</div>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>{t.v1beta.properties.name}</TableHead>
								<TableHead>{t.v1beta.properties.type}</TableHead>
								<TableHead>{t.v1beta.properties.optionsOrRelated}</TableHead>
								<TableHead>{t.v1beta.properties.actions}</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{filteredUserProperties.map(property => (
								<TableRow key={property.id}>
									<TableCell>
										<div className='flex items-center gap-2'>
											{property.name}
										</div>
									</TableCell>
									<TableCell>{property.typ}</TableCell>
									<TableCell>
										{(property.typ === PropertyType.Select ||
											property.typ === PropertyType.MultiSelect) && (
											<div className='flex flex-wrap gap-1'>
												{(
													property.meta as
														| SelectTypeMetaForPropertiesUiFragment
														| MultiSelectTypeMetaForPropertiesUiFragment
												)?.options?.map(o => (
													<Badge
														key={o.id}
														variant='secondary'
														className='whitespace-nowrap'
													>
														{o.name}
													</Badge>
												))}
											</div>
										)}
										{property.typ === PropertyType.Relation &&
											(property.meta as RelationTypeMetaForPropertiesUiFragment)
												?.databaseId}
									</TableCell>
									<TableCell>
										<div className='flex space-x-2'>
											<Button
												variant='outline'
												size='sm'
												onClick={() => {
													setEditingProperty(property)
													setIsAddEditPropertyDialogOpen(true)
												}}
												disabled={isEssentialProperty(property)}
											>
												<Edit2 className='mr-2 h-4 w-4' />
												{t.v1beta.properties.edit}
											</Button>
											<Button
												variant='outline'
												size='sm'
												onClick={() => removeCustomProperty(property.id)}
												disabled={isEssentialProperty(property)}
											>
												<X className='mr-2 h-4 w-4' />
												{t.v1beta.properties.remove}
											</Button>
										</div>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</section>

				{/* System Extensions Section */}
				{systemExtensions.length > 0 && (
					<section>
						<div className='mb-4'>
							<h2 className='text-lg font-semibold'>
								{t.v1beta.properties.systemExtensions}
							</h2>
							<p className='text-sm text-muted-foreground'>
								{t.v1beta.properties.systemExtensionsDescription}
							</p>
						</div>
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>{t.v1beta.properties.name}</TableHead>
									<TableHead>{t.v1beta.properties.type}</TableHead>
									<TableHead>{t.v1beta.properties.status}</TableHead>
									<TableHead>{t.v1beta.properties.actions}</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{systemExtensions.map(property => (
									<TableRow key={property.id}>
										<TableCell>
											<div className='flex items-center gap-2'>
												{isExtGithubProperty(property) && (
													<Github className='h-4 w-4 text-muted-foreground' />
												)}
												{property.name}
												<Badge variant='outline' className='text-xs'>
													{isExtGithubProperty(property)
														? t.v1beta.properties.githubSync
														: t.v1beta.properties.extension}
												</Badge>
											</div>
										</TableCell>
										<TableCell>
											{isExtGithubProperty(property)
												? t.v1beta.properties.githubSync
												: property.typ}
										</TableCell>
										<TableCell>
											{isExtGithubProperty(property) && (
												<GitHubReposDisplay
													value={
														(property.meta as { json?: string } | null)?.json ??
														undefined
													}
												/>
											)}
										</TableCell>
										<TableCell>
											<div className='flex space-x-2'>
												{isExtGithubProperty(property) ? (
													<Button
														variant='outline'
														size='sm'
														onClick={() => {
															setEditingGitHubReposProperty(property)
															setIsGitHubReposDialogOpen(true)
														}}
													>
														<Github className='mr-2 h-4 w-4' />
														{t.v1beta.properties.configure}
													</Button>
												) : settingsUrl ? (
													<Button variant='outline' size='sm' asChild>
														<Link href={settingsUrl}>
															<ExternalLink className='mr-2 h-4 w-4' />
															{t.v1beta.properties.settings}
														</Link>
													</Button>
												) : null}
											</div>
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
						<Alert className='mt-4'>
							<Info className='h-4 w-4' />
							<AlertTitle>
								{t.v1beta.properties.systemExtensionsInfo}
							</AlertTitle>
							<AlertDescription>
								{settingsUrl ? (
									<>
										{t.v1beta.properties.systemExtensionsInfoDescription
											.split('Settings > Integrations')
											.map((part, index, array) =>
												index < array.length - 1 ? (
													<span key={part}>
														{part}
														<Link
															href={settingsUrl}
															className='font-medium text-primary underline'
														>
															Settings &gt; Integrations
														</Link>
													</span>
												) : (
													<span key={part}>{part}</span>
												),
											)}
									</>
								) : (
									t.v1beta.properties.systemExtensionsInfoDescription
								)}
							</AlertDescription>
						</Alert>
					</section>
				)}
			</div>
			<PropertyDialog
				isOpen={isAddEditPropertyDialogOpen}
				onClose={() => {
					setIsAddEditPropertyDialogOpen(false)
					setEditingProperty(null)
				}}
				editingProperty={editingProperty}
				databases={databases}
				onSave={handleSaveProperty}
			/>
			<GitHubReposEditorDialog
				isOpen={isGitHubReposDialogOpen}
				onClose={() => {
					setIsGitHubReposDialogOpen(false)
					setEditingGitHubReposProperty(null)
				}}
				value={
					(editingGitHubReposProperty?.meta as { json?: string } | null)
						?.json ?? undefined
				}
				onSave={handleSaveGitHubRepos}
				isGitHubConnected={isGitHubConnected}
				onBulkSync={
					onBulkSyncGitHub && editingGitHubReposProperty
						? repoConfigs =>
								onBulkSyncGitHub(repoConfigs, editingGitHubReposProperty.id)
						: undefined
				}
				totalDataCount={totalDataCount}
			/>
		</main>
	)
}
