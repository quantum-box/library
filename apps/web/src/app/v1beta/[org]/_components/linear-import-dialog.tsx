'use client'

import { useEffect, useMemo, useState, useTransition } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
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
import { Switch } from '@/components/ui/switch'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import {
	CheckCircle2,
	Circle,
	FileText,
	Loader2,
	Plus,
	Trash2,
} from 'lucide-react'
import { useRouter } from 'next/navigation'
import { toast } from 'sonner'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import {
	createLinearRepository,
	createLinearWebhookEndpoint,
	startLinearSync,
} from './linear-import-actions'

const LinearTeamsQuery = graphql(`
  query LinearTeams {
    linearListTeams {
      id
      name
      key
    }
  }
`)

const LinearProjectsQuery = graphql(`
  query LinearProjects($teamId: String) {
    linearListProjects(teamId: $teamId) {
      id
      name
    }
  }
`)

const LinearIssuesQuery = graphql(`
  query LinearIssues($teamId: String, $projectId: String) {
    linearListIssues(teamId: $teamId, projectId: $projectId) {
      id
      identifier
      title
      stateName
      assigneeName
      url
    }
  }
`)

type ImportStep = 'select-source' | 'select-issues' | 'configure' | 'importing'
type ImportStatus =
	| 'idle'
	| 'creating-repo'
	| 'creating-webhook'
	| 'starting-sync'
	| 'completed'

interface LinearImportDialogProps {
	org: string
	tenantId: string
	hasLinearConnection: boolean
}

interface LinearTeam {
	id: string
	name: string
	key: string
}

interface LinearProject {
	id: string
	name: string
}

interface LinearIssue {
	id: string
	identifier: string
	title: string
	stateName?: string | null
	assigneeName?: string | null
	url?: string | null
}

interface LinearMapping {
	sourceField: string
	targetProperty: string
}

const DEFAULT_LINEAR_MAPPINGS: LinearMapping[] = [
	{ sourceField: 'identifier', targetProperty: 'identifier' },
	{ sourceField: 'title', targetProperty: 'title' },
	{ sourceField: 'description', targetProperty: 'description' },
	{ sourceField: 'state.name', targetProperty: 'status' },
	{ sourceField: 'assignee.name', targetProperty: 'assigned_to' },
	{ sourceField: 'priority', targetProperty: 'priority' },
	{ sourceField: 'labels', targetProperty: 'tags' },
]

const IMPORT_STATUS_ORDER: ImportStatus[] = [
	'idle',
	'creating-repo',
	'creating-webhook',
	'starting-sync',
	'completed',
]

const normalizeRepoName = (value: string) =>
	value
		.toLowerCase()
		.replace(/[^a-z0-9-]/g, '-')
		.replace(/-+/g, '-')
		.replace(/^-|-$/g, '')

export function LinearImportDialog({
	org,
	tenantId,
	hasLinearConnection,
}: LinearImportDialogProps) {
	const router = useRouter()
	const { t } = useTranslation()
	const [isPending, startTransition] = useTransition()
	const [open, setOpen] = useState(false)
	const [step, setStep] = useState<ImportStep>('select-source')
	const [repoName, setRepoName] = useState('')
	const [selectedTeamId, setSelectedTeamId] = useState<string>('all')
	const [selectedProjectId, setSelectedProjectId] = useState<string>('all')
	const [selectSpecificIssues, setSelectSpecificIssues] = useState(false)
	const [teams, setTeams] = useState<LinearTeam[]>([])
	const [projects, setProjects] = useState<LinearProject[]>([])
	const [issues, setIssues] = useState<LinearIssue[]>([])
	const [selectedIssueIds, setSelectedIssueIds] = useState<string[]>([])
	const [isLoadingTeams, setIsLoadingTeams] = useState(false)
	const [isLoadingProjects, setIsLoadingProjects] = useState(false)
	const [isLoadingIssues, setIsLoadingIssues] = useState(false)
	const [propertyMappings, setPropertyMappings] = useState<LinearMapping[]>(
		DEFAULT_LINEAR_MAPPINGS,
	)
	const [importStatus, setImportStatus] = useState<ImportStatus>('idle')

	const selectedTeam = useMemo(
		() => teams.find(team => team.id === selectedTeamId),
		[teams, selectedTeamId],
	)
	const selectedProject = useMemo(
		() => projects.find(project => project.id === selectedProjectId),
		[projects, selectedProjectId],
	)
	const linearSourceFields = useMemo(
		() => [
			{
				value: 'identifier',
				label: t.v1beta.linearImport.mapping.fields.identifier,
			},
			{ value: 'title', label: t.v1beta.linearImport.mapping.fields.title },
			{
				value: 'description',
				label: t.v1beta.linearImport.mapping.fields.description,
			},
			{
				value: 'state.name',
				label: t.v1beta.linearImport.mapping.fields.status,
			},
			{
				value: 'assignee.name',
				label: t.v1beta.linearImport.mapping.fields.assignee,
			},
			{
				value: 'priority',
				label: t.v1beta.linearImport.mapping.fields.priority,
			},
			{ value: 'labels', label: t.v1beta.linearImport.mapping.fields.labels },
		],
		[t],
	)
	const importProgressSteps = useMemo(
		() => [
			{
				id: 'creating-repo',
				label: t.v1beta.linearImport.progress.createRepository,
			},
			{
				id: 'creating-webhook',
				label: t.v1beta.linearImport.progress.createWebhook,
			},
			{
				id: 'starting-sync',
				label: t.v1beta.linearImport.progress.startInitialSync,
			},
		],
		[t],
	)

	useEffect(() => {
		if (!open) {
			setStep('select-source')
			setRepoName('')
			setSelectedTeamId('all')
			setSelectedProjectId('all')
			setSelectSpecificIssues(false)
			setIssues([])
			setSelectedIssueIds([])
			setPropertyMappings(DEFAULT_LINEAR_MAPPINGS)
			setImportStatus('idle')
		}
	}, [open])

	useEffect(() => {
		if (!open || teams.length > 0 || isLoadingTeams) return
		if (!hasLinearConnection) return
		if (!tenantId) return
		setIsLoadingTeams(true)
		executeGraphQL(LinearTeamsQuery, {}, { operatorId: tenantId })
			.then(result => {
				if (result?.linearListTeams) {
					setTeams(result.linearListTeams)
				}
			})
			.catch(error => {
				console.error('Failed to load teams:', error)
				toast.error(t.v1beta.linearImport.errors.loadTeamsFailed)
			})
			.finally(() => {
				setIsLoadingTeams(false)
			})
	}, [open, teams.length, isLoadingTeams, tenantId, hasLinearConnection])

	useEffect(() => {
		if (!open || !tenantId) return
		if (!hasLinearConnection) return
		setIsLoadingProjects(true)
		const teamId =
			selectedTeamId && selectedTeamId !== 'all' ? selectedTeamId : null
		executeGraphQL(LinearProjectsQuery, { teamId }, { operatorId: tenantId })
			.then(result => {
				if (result?.linearListProjects) {
					setProjects(result.linearListProjects)
				}
			})
			.catch(error => {
				console.error('Failed to load projects:', error)
				toast.error(t.v1beta.linearImport.errors.loadProjectsFailed)
			})
			.finally(() => {
				setIsLoadingProjects(false)
			})
	}, [open, selectedTeamId, tenantId, hasLinearConnection])

	useEffect(() => {
		if (!open) return
		setSelectedProjectId('all')
		setIssues([])
		setSelectedIssueIds([])
	}, [selectedTeamId, open])

	useEffect(() => {
		if (!open) return
		setIssues([])
		setSelectedIssueIds([])
	}, [selectedProjectId, open])

	useEffect(() => {
		if (repoName) return
		const suggested = selectedProject?.name ?? selectedTeam?.name
		if (suggested) {
			setRepoName(normalizeRepoName(suggested))
		}
	}, [repoName, selectedProject?.name, selectedTeam?.name])

	useEffect(() => {
		if (!selectSpecificIssues) {
			setSelectedIssueIds([])
		}
	}, [selectSpecificIssues])

	useEffect(() => {
		if (!open || step !== 'select-issues' || !tenantId) return
		if (!hasLinearConnection) return
		setIsLoadingIssues(true)
		const teamId =
			selectedTeamId && selectedTeamId !== 'all' ? selectedTeamId : null
		const projectId =
			selectedProjectId && selectedProjectId !== 'all'
				? selectedProjectId
				: null
		executeGraphQL(
			LinearIssuesQuery,
			{ teamId, projectId },
			{ operatorId: tenantId },
		)
			.then(result => {
				setIssues(result?.linearListIssues ?? [])
			})
			.catch(error => {
				console.error('Failed to load issues:', error)
				toast.error(t.v1beta.linearImport.errors.loadIssuesFailed)
			})
			.finally(() => {
				setIsLoadingIssues(false)
			})
	}, [
		open,
		step,
		selectedTeamId,
		selectedProjectId,
		tenantId,
		hasLinearConnection,
	])

	const handleNextFromSource = () => {
		if (selectSpecificIssues) {
			setStep('select-issues')
			return
		}
		setStep('configure')
	}

	const handleNextFromIssues = () => {
		if (selectedIssueIds.length === 0) {
			toast.error(t.v1beta.linearImport.errors.selectIssueRequired)
			return
		}
		setStep('configure')
	}

	const handleBack = () => {
		if (step === 'select-issues') {
			setStep('select-source')
			return
		}
		if (step === 'configure') {
			setStep(selectSpecificIssues ? 'select-issues' : 'select-source')
		}
	}

	const handleToggleIssue = (issueId: string) => {
		setSelectedIssueIds(prev =>
			prev.includes(issueId)
				? prev.filter(id => id !== issueId)
				: [...prev, issueId],
		)
	}

	const handleSelectAllIssues = () => {
		setSelectedIssueIds(issues.map(issue => issue.id))
	}

	const handleClearIssues = () => {
		setSelectedIssueIds([])
	}

	const handleAddMapping = () => {
		setPropertyMappings(prev => [
			...prev,
			{ sourceField: '', targetProperty: '' },
		])
	}

	const handleRemoveMapping = (index: number) => {
		setPropertyMappings(prev => prev.filter((_, idx) => idx !== index))
	}

	const handleUpdateMapping = (
		index: number,
		field: keyof LinearMapping,
		value: string,
	) => {
		setPropertyMappings(prev => {
			const next = [...prev]
			next[index] = { ...next[index], [field]: value }
			return next
		})
	}

	const buildMappingJson = () => {
		const staticMappings = propertyMappings
			.filter(mapping => mapping.sourceField && mapping.targetProperty)
			.map(mapping => ({
				source_field: mapping.sourceField,
				target_property: mapping.targetProperty,
			}))

		if (staticMappings.length === 0) return null

		return JSON.stringify({
			static_mappings: staticMappings,
			computed_mappings: [],
			defaults: {},
		})
	}

	const handleImport = () => {
		if (!repoName) {
			toast.error(t.v1beta.linearImport.errors.repoNameRequired)
			return
		}

		setStep('importing')
		setImportStatus('creating-repo')
		startTransition(async () => {
			const repoResult = await createLinearRepository({
				orgUsername: org,
				tenantId,
				repoName,
				description: 'Imported from Linear',
			})

			if (!repoResult.success || !repoResult.repoId) {
				toast.error(
					repoResult.error || t.v1beta.linearImport.errors.repoCreateFailed,
				)
				setImportStatus('idle')
				setStep('configure')
				return
			}

			setImportStatus('creating-webhook')
			const webhookResult = await createLinearWebhookEndpoint({
				tenantId,
				repoId: repoResult.repoId,
				repoName,
				teamId: selectedTeamId !== 'all' ? selectedTeamId : undefined,
				projectId: selectedProjectId !== 'all' ? selectedProjectId : undefined,
				mapping: buildMappingJson(),
			})

			if (!webhookResult.success || !webhookResult.endpointId) {
				toast.error(
					webhookResult.error ||
						t.v1beta.linearImport.errors.webhookCreateFailed,
				)
				setImportStatus('idle')
				setStep('configure')
				return
			}

			setImportStatus('starting-sync')
			const syncResult = await startLinearSync({
				orgUsername: org,
				tenantId,
				repoUsername: repoResult.repoUsername ?? repoName,
				endpointId: webhookResult.endpointId,
				issueIds: selectSpecificIssues ? selectedIssueIds : undefined,
			})

			if (!syncResult.success) {
				toast.error(syncResult.error || t.v1beta.linearImport.errors.syncFailed)
				setImportStatus('idle')
				setStep('configure')
				return
			}

			const repoUsername = repoResult.repoUsername ?? repoName
			setImportStatus('completed')
			toast.success(
				t.v1beta.linearImport.success.importCompleted.replace(
					'{repo}',
					repoUsername,
				),
			)
			setOpen(false)
			setRepoName('')
			setSelectedTeamId('all')
			setSelectedProjectId('all')
			setSelectSpecificIssues(false)
			setSelectedIssueIds([])
			setPropertyMappings(DEFAULT_LINEAR_MAPPINGS)
			router.refresh()

			if (repoUsername) {
				router.push(`/v1beta/${org}/${repoUsername}`)
			}
		})
	}

	const issueCountLabel = selectSpecificIssues
		? t.v1beta.linearImport.issues.selected.replace(
				'{count}',
				String(selectedIssueIds.length),
			)
		: t.v1beta.linearImport.issues.all

	return (
		<Dialog open={open} onOpenChange={setOpen}>
			<DialogTrigger asChild>
				<Button variant='outline'>
					<FileText className='mr-2 h-4 w-4' />
					{t.v1beta.linearImport.trigger}
				</Button>
			</DialogTrigger>
			<DialogContent className='max-w-3xl max-h-[80vh] overflow-y-auto'>
				<DialogHeader>
					<DialogTitle>{t.v1beta.linearImport.title}</DialogTitle>
					<DialogDescription>
						{step === 'select-source' &&
							t.v1beta.linearImport.description.selectSource}
						{step === 'select-issues' &&
							t.v1beta.linearImport.description.selectIssues}
						{step === 'configure' &&
							t.v1beta.linearImport.description.configure}
						{step === 'importing' &&
							t.v1beta.linearImport.description.importing}
					</DialogDescription>
				</DialogHeader>

				<div className='space-y-4 py-4'>
					{step === 'select-source' && (
						<div className='space-y-4'>
							<div className='flex items-center justify-between rounded-lg border p-3'>
								<div>
									<Label htmlFor='linear-connection'>
										{t.v1beta.linearImport.connection.label}
									</Label>
									<p className='text-sm text-muted-foreground'>
										{hasLinearConnection
											? t.v1beta.linearImport.connection.connected
											: t.v1beta.linearImport.connection.notConnected}
									</p>
								</div>
								<Badge variant={hasLinearConnection ? 'default' : 'secondary'}>
									{hasLinearConnection
										? t.v1beta.linearImport.connection.connected
										: t.v1beta.linearImport.connection.disconnected}
								</Badge>
							</div>

							<div className='space-y-2'>
								<Label htmlFor='teamSelect'>
									{t.v1beta.linearImport.team.label}
								</Label>
								<Select
									value={selectedTeamId}
									onValueChange={setSelectedTeamId}
									disabled={isLoadingTeams}
								>
									<SelectTrigger id='teamSelect'>
										<SelectValue
											placeholder={
												isLoadingTeams
													? t.v1beta.linearImport.team.placeholderLoading
													: t.v1beta.linearImport.team.placeholderSelect
											}
										/>
									</SelectTrigger>
									<SelectContent>
										<SelectItem value='all'>
											{t.v1beta.linearImport.team.all}
										</SelectItem>
										{teams.map(team => (
											<SelectItem key={team.id} value={team.id}>
												{team.name} ({team.key})
											</SelectItem>
										))}
									</SelectContent>
								</Select>
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.linearImport.team.helper}
								</p>
							</div>

							<div className='space-y-2'>
								<Label htmlFor='projectSelect'>
									{t.v1beta.linearImport.project.label}
								</Label>
								<Select
									value={selectedProjectId}
									onValueChange={setSelectedProjectId}
									disabled={isLoadingProjects}
								>
									<SelectTrigger id='projectSelect'>
										<SelectValue
											placeholder={
												isLoadingProjects
													? t.v1beta.linearImport.project.placeholderLoading
													: t.v1beta.linearImport.project.placeholderSelect
											}
										/>
									</SelectTrigger>
									<SelectContent>
										<SelectItem value='all'>
											{t.v1beta.linearImport.project.all}
										</SelectItem>
										{projects.map(project => (
											<SelectItem key={project.id} value={project.id}>
												{project.name}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.linearImport.project.helper}
								</p>
							</div>

							<div className='flex items-center justify-between rounded-lg border p-3'>
								<div>
									<Label htmlFor='issueScope'>
										{t.v1beta.linearImport.issueScope.label}
									</Label>
									<p className='text-sm text-muted-foreground'>
										{t.v1beta.linearImport.issueScope.description}
									</p>
								</div>
								<Switch
									id='issueScope'
									checked={selectSpecificIssues}
									onCheckedChange={setSelectSpecificIssues}
								/>
							</div>
						</div>
					)}

					{step === 'select-issues' && (
						<div className='space-y-4'>
							<div className='flex items-center justify-between'>
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.linearImport.issues.count.replace(
										'{count}',
										String(issues.length),
									)}
								</p>
								<div className='flex gap-2'>
									<Button
										variant='outline'
										size='sm'
										onClick={handleSelectAllIssues}
										disabled={issues.length === 0}
									>
										{t.v1beta.linearImport.issues.selectAll}
									</Button>
									<Button
										variant='outline'
										size='sm'
										onClick={handleClearIssues}
										disabled={selectedIssueIds.length === 0}
									>
										{t.v1beta.linearImport.issues.clear}
									</Button>
								</div>
							</div>

							{isLoadingIssues ? (
								<div className='flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground'>
									<Loader2 className='h-4 w-4 animate-spin' />
									{t.v1beta.linearImport.issues.loading}
								</div>
							) : issues.length === 0 ? (
								<div className='rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground'>
									{t.v1beta.linearImport.issues.empty}
								</div>
							) : (
								<div className='rounded-lg border'>
									<Table>
										<TableHeader>
											<TableRow>
												<TableHead className='w-[48px]' />
												<TableHead>{t.v1beta.linearImport.table.key}</TableHead>
												<TableHead>
													{t.v1beta.linearImport.table.title}
												</TableHead>
												<TableHead>
													{t.v1beta.linearImport.table.status}
												</TableHead>
												<TableHead>
													{t.v1beta.linearImport.table.assignee}
												</TableHead>
											</TableRow>
										</TableHeader>
										<TableBody>
											{issues.map(issue => {
												const checked = selectedIssueIds.includes(issue.id)
												return (
													<TableRow key={issue.id}>
														<TableCell>
															<Checkbox
																checked={checked}
																onCheckedChange={() =>
																	handleToggleIssue(issue.id)
																}
															/>
														</TableCell>
														<TableCell className='font-mono text-xs'>
															{issue.identifier}
														</TableCell>
														<TableCell>{issue.title}</TableCell>
														<TableCell>
															<Badge variant='secondary'>
																{issue.stateName ||
																	t.v1beta.linearImport.table.unknown}
															</Badge>
														</TableCell>
														<TableCell>{issue.assigneeName || '-'}</TableCell>
													</TableRow>
												)
											})}
										</TableBody>
									</Table>
								</div>
							)}
						</div>
					)}

					{step === 'configure' && (
						<div className='space-y-6'>
							<div className='space-y-2'>
								<Label htmlFor='repoName'>
									{t.v1beta.linearImport.repository.name}
								</Label>
								<Input
									id='repoName'
									placeholder={t.v1beta.linearImport.repository.placeholder}
									value={repoName}
									onChange={e => setRepoName(e.target.value)}
								/>
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.linearImport.repository.description}
								</p>
							</div>

							<div className='space-y-3'>
								<div className='flex items-center justify-between'>
									<div>
										<h4 className='text-sm font-medium'>
											{t.v1beta.linearImport.mapping.title}
										</h4>
										<p className='text-xs text-muted-foreground'>
											{t.v1beta.linearImport.mapping.description}
										</p>
									</div>
									<Button
										variant='outline'
										size='sm'
										onClick={handleAddMapping}
									>
										<Plus className='mr-2 h-4 w-4' />
										{t.v1beta.linearImport.mapping.add}
									</Button>
								</div>

								<div className='grid grid-cols-[1fr_1fr_auto] gap-2 text-xs text-muted-foreground'>
									<div>{t.v1beta.linearImport.mapping.linearField}</div>
									<div>{t.v1beta.linearImport.mapping.repositoryProperty}</div>
									<div className='sr-only'>
										{t.v1beta.linearImport.mapping.actions}
									</div>
								</div>

								<div className='space-y-2'>
									{propertyMappings.map((mapping, index) => (
										<div
											key={`${mapping.sourceField}-${index}`}
											className='grid grid-cols-[1fr_1fr_auto] gap-2 items-center'
										>
											<Select
												value={mapping.sourceField}
												onValueChange={value =>
													handleUpdateMapping(index, 'sourceField', value)
												}
											>
												<SelectTrigger>
													<SelectValue
														placeholder={
															t.v1beta.linearImport.mapping.selectField
														}
													/>
												</SelectTrigger>
												<SelectContent>
													{linearSourceFields.map(field => (
														<SelectItem key={field.value} value={field.value}>
															{field.label}
														</SelectItem>
													))}
												</SelectContent>
											</Select>
											<Input
												value={mapping.targetProperty}
												onChange={event =>
													handleUpdateMapping(
														index,
														'targetProperty',
														event.target.value,
													)
												}
												placeholder={
													t.v1beta.linearImport.mapping.propertyNamePlaceholder
												}
											/>
											<Button
												variant='ghost'
												size='icon'
												onClick={() => handleRemoveMapping(index)}
												disabled={propertyMappings.length <= 1}
											>
												<Trash2 className='h-4 w-4' />
											</Button>
										</div>
									))}
								</div>

								<div className='rounded-lg bg-muted p-3 text-xs text-muted-foreground'>
									<p className='font-medium text-foreground mb-2'>
										{t.v1beta.linearImport.mapping.defaultTitle}
									</p>
									<ul className='space-y-1'>
										<li>• identifier → identifier</li>
										<li>• title → title</li>
										<li>• description → description</li>
										<li>• state.name → status</li>
										<li>• assignee.name → assigned_to</li>
										<li>• priority → priority</li>
										<li>• labels → tags</li>
									</ul>
								</div>
							</div>

							<div className='rounded-lg bg-muted p-4'>
								<h4 className='text-sm font-medium mb-2'>
									{t.v1beta.linearImport.summary.title}
								</h4>
								<ul className='text-sm text-muted-foreground space-y-1'>
									<li>
										• {t.v1beta.linearImport.summary.team}:{' '}
										{selectedTeam?.name ?? t.v1beta.linearImport.team.all}
									</li>
									<li>
										• {t.v1beta.linearImport.summary.project}:{' '}
										{selectedProject?.name ?? t.v1beta.linearImport.project.all}
									</li>
									<li>
										• {t.v1beta.linearImport.summary.issues}: {issueCountLabel}
									</li>
									<li>{t.v1beta.linearImport.summary.extLinearEnabled}</li>
								</ul>
							</div>
						</div>
					)}

					{step === 'importing' && (
						<div className='space-y-4 py-4'>
							<div className='flex items-center gap-2 text-sm text-muted-foreground'>
								<Loader2 className='h-4 w-4 animate-spin' />
								{t.v1beta.linearImport.description.importing}
							</div>
							<div className='space-y-2'>
								{importProgressSteps.map((progressStep, index) => {
									const currentIndex = IMPORT_STATUS_ORDER.indexOf(importStatus)
									const stepIndex = index + 1
									const isComplete =
										importStatus === 'completed' || currentIndex > stepIndex
									const isCurrent =
										importStatus !== 'completed' && currentIndex === stepIndex

									return (
										<div
											key={progressStep.id}
											className='flex items-center gap-2 text-sm'
										>
											{isComplete ? (
												<CheckCircle2 className='h-4 w-4 text-primary' />
											) : isCurrent ? (
												<Loader2 className='h-4 w-4 animate-spin text-muted-foreground' />
											) : (
												<Circle className='h-3 w-3 text-muted-foreground' />
											)}
											<span
												className={
													isComplete
														? 'text-foreground'
														: 'text-muted-foreground'
												}
											>
												{progressStep.label}
											</span>
										</div>
									)
								})}
							</div>
							<p className='text-xs text-muted-foreground'>
								{t.v1beta.linearImport.progress.note}
							</p>
						</div>
					)}
				</div>

				<DialogFooter>
					{step === 'select-source' && (
						<>
							<Button
								variant='outline'
								onClick={() => setOpen(false)}
								disabled={isPending}
							>
								{t.common.cancel}
							</Button>
							<Button onClick={handleNextFromSource} disabled={isPending}>
								{t.common.next}
							</Button>
						</>
					)}

					{step === 'select-issues' && (
						<>
							<Button
								variant='outline'
								onClick={handleBack}
								disabled={isPending}
							>
								{t.common.back}
							</Button>
							<Button
								onClick={handleNextFromIssues}
								disabled={isPending || isLoadingIssues}
							>
								{t.common.next}
							</Button>
						</>
					)}

					{step === 'configure' && (
						<>
							<Button
								variant='outline'
								onClick={handleBack}
								disabled={isPending}
							>
								{t.common.back}
							</Button>
							<Button onClick={handleImport} disabled={isPending || !repoName}>
								{isPending ? (
									<>
										<Loader2 className='mr-2 h-4 w-4 animate-spin' />
										{t.v1beta.linearImport.actions.importing}
									</>
								) : (
									t.v1beta.linearImport.actions.import
								)}
							</Button>
						</>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
