'use client'

import { useEffect, useMemo, useState, useTransition } from 'react'
import { useRouter } from 'next/navigation'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { ExternalLink, Settings } from 'lucide-react'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { SyncHistory } from '@/components/sync/sync-history'
import { toast } from 'sonner'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import {
	createWebhookEndpoint,
	startInitialSync,
	updateEndpointConfig,
	updateEndpointEvents,
	updateEndpointStatus,
} from '@/app/v1beta/[org]/webhooks/actions'
import { enableLinearSyncAction } from '../actions'
import { MappingEditor } from '../../../webhooks/[endpointId]/components/mapping-editor'

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

interface LinearTeam {
	id: string
	name: string
	key: string
}

interface LinearProject {
	id: string
	name: string
}

interface LinearEndpoint {
	id: string
	tenantId: string
	repositoryId?: string | null
	name: string
	provider: string
	config: string
	events: string[]
	mapping?: string | null
	status: 'ACTIVE' | 'PAUSED' | 'DISABLED'
	webhookUrl: string
}

interface LinearExtensionSettingsProps {
	org: string
	repo: string
	tenantId?: string | null
	repoId?: string | null
	connection?: {
		id: string
		provider: string
		status: string
		externalAccountName?: string | null
	} | null
	endpoint?: LinearEndpoint | null
}

function parseLinearConfig(config?: string | null) {
	if (!config) return {}
	try {
		const parsed = JSON.parse(config)
		return parsed.linear || parsed
	} catch {
		return {}
	}
}

export function LinearExtensionSettings({
	org,
	repo,
	tenantId,
	repoId,
	connection,
	endpoint,
}: LinearExtensionSettingsProps) {
	const router = useRouter()
	const { t } = useTranslation()
	const isConnected = connection?.status?.toLowerCase?.() === 'active'
	const [isPending, startTransition] = useTransition()
	const [teams, setTeams] = useState<LinearTeam[]>([])
	const [projects, setProjects] = useState<LinearProject[]>([])
	const [isLoadingTeams, setIsLoadingTeams] = useState(false)
	const [isLoadingProjects, setIsLoadingProjects] = useState(false)
	const [selectedTeamId, setSelectedTeamId] = useState('all')
	const [selectedProjectId, setSelectedProjectId] = useState('all')
	const [selectedEvents, setSelectedEvents] = useState<string[]>([
		'Issue',
		'Project',
	])
	const [webhookSecret, setWebhookSecret] = useState('')
	const [mappingOpen, setMappingOpen] = useState(false)
	const [createdWebhook, setCreatedWebhook] = useState<{
		webhookUrl: string
		secret: string
	} | null>(null)

	const linearEventOptions = useMemo(
		() => [
			{
				value: 'Issue',
				label: t.v1beta.repoSettings.integrationsTab.eventLabels.issue,
			},
			{
				value: 'Project',
				label: t.v1beta.repoSettings.integrationsTab.eventLabels.project,
			},
			{
				value: 'Comment',
				label: t.v1beta.repoSettings.integrationsTab.eventLabels.comment,
			},
			{
				value: 'Cycle',
				label: t.v1beta.repoSettings.integrationsTab.eventLabels.cycle,
			},
		],
		[t],
	)

	const linearConfig = useMemo(
		() => parseLinearConfig(endpoint?.config),
		[endpoint],
	)

	useEffect(() => {
		if (!endpoint) {
			setSelectedTeamId('all')
			setSelectedProjectId('all')
			setSelectedEvents(['Issue', 'Project'])
			setWebhookSecret('')
			return
		}
		const teamId =
			typeof linearConfig.team_id === 'string' &&
			linearConfig.team_id.length > 0
				? linearConfig.team_id
				: 'all'
		const projectId =
			typeof linearConfig.project_id === 'string' &&
			linearConfig.project_id.length > 0
				? linearConfig.project_id
				: 'all'
		setSelectedTeamId(teamId)
		setSelectedProjectId(projectId)
		setSelectedEvents(endpoint.events ?? [])
		setWebhookSecret(
			typeof linearConfig.webhook_secret === 'string'
				? linearConfig.webhook_secret
				: '',
		)
	}, [
		endpoint,
		linearConfig.team_id,
		linearConfig.project_id,
		linearConfig.webhook_secret,
	])

	useEffect(() => {
		if (!isConnected || !tenantId || isLoadingTeams || teams.length > 0) return
		setIsLoadingTeams(true)
		executeGraphQL(LinearTeamsQuery, {}, { operatorId: tenantId })
			.then(result => {
				if (result?.linearListTeams) {
					setTeams(result.linearListTeams)
				}
			})
			.catch(error => {
				console.error('Failed to load Linear teams:', error)
				toast.error(t.v1beta.repoSettings.integrationsTab.loadTeamsFailed)
			})
			.finally(() => setIsLoadingTeams(false))
	}, [isConnected, isLoadingTeams, teams.length, tenantId])

	useEffect(() => {
		if (!isConnected || !tenantId) return
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
				console.error('Failed to load Linear projects:', error)
				toast.error(t.v1beta.repoSettings.integrationsTab.loadProjectsFailed)
			})
			.finally(() => setIsLoadingProjects(false))
	}, [isConnected, selectedTeamId, tenantId])

	const handleConnect = async () => {
		const integrationId = 'int_linear'
		const oauthTenantId = tenantId ?? org
		const oauthUrl = `/oauth/linear/authorize?tenant_id=${oauthTenantId}&integration_id=${integrationId}&org_username=${org}`
		window.location.href = oauthUrl
	}

	const handleCreateEndpoint = () => {
		if (!tenantId || !repoId) {
			toast.error(t.v1beta.repoSettings.integrationsTab.tenantOrRepoMissing)
			return
		}

		startTransition(async () => {
			const result = await createWebhookEndpoint({
				tenantId,
				name: `${repo} Linear Sync`,
				provider: 'LINEAR',
				config: JSON.stringify({
					provider: 'linear',
					team_id:
						selectedTeamId && selectedTeamId !== 'all' ? selectedTeamId : null,
					project_id:
						selectedProjectId && selectedProjectId !== 'all'
							? selectedProjectId
							: null,
					webhook_secret: webhookSecret.trim() ? webhookSecret.trim() : null,
				}),
				events: selectedEvents,
				repositoryId: repoId,
			})

			if (result.error) {
				toast.error(result.error)
				return
			}

			if (result.data) {
				try {
					await enableLinearSyncAction({
						orgUsername: org,
						repoUsername: repo,
					})
				} catch (error) {
					console.error('Failed to enable Linear sync property:', error)
					toast.error(
						t.v1beta.repoSettings.integrationsTab
							.extLinearPropertyCreationFailed,
					)
				}
				setCreatedWebhook(result.data)
				toast.success(
					t.v1beta.repoSettings.integrationsTab.webhookEndpointCreated,
				)
				router.refresh()
			}
		})
	}

	const handleUpdateConfig = () => {
		if (!endpoint) return
		startTransition(async () => {
			const configResult = await updateEndpointConfig({
				endpointId: endpoint.id,
				config: JSON.stringify({
					provider: 'linear',
					team_id:
						selectedTeamId && selectedTeamId !== 'all' ? selectedTeamId : null,
					project_id:
						selectedProjectId && selectedProjectId !== 'all'
							? selectedProjectId
							: null,
					webhook_secret: webhookSecret.trim() ? webhookSecret.trim() : null,
				}),
				operatorId: endpoint.tenantId,
			})

			if (configResult.error) {
				toast.error(configResult.error)
				return
			}

			const eventsResult = await updateEndpointEvents({
				endpointId: endpoint.id,
				events: selectedEvents,
				operatorId: endpoint.tenantId,
			})

			if (eventsResult.error) {
				toast.error(eventsResult.error)
				return
			}

			toast.success(
				t.v1beta.repoSettings.integrationsTab.syncConfigurationUpdated,
			)
			router.refresh()
		})
	}

	const handleToggleStatus = (checked: boolean) => {
		if (!endpoint) return
		startTransition(async () => {
			const result = await updateEndpointStatus({
				tenantId: endpoint.tenantId,
				endpointId: endpoint.id,
				status: checked ? 'ACTIVE' : 'DISABLED',
			})

			if (result.error) {
				toast.error(result.error)
				return
			}

			toast.success(
				checked
					? t.v1beta.repoSettings.integrationsTab.syncEnabledToast
					: t.v1beta.repoSettings.integrationsTab.syncDisabledToast,
			)
			router.refresh()
		})
	}

	const handleInitialSync = () => {
		if (!endpoint) return
		startTransition(async () => {
			const result = await startInitialSync({
				endpointId: endpoint.id,
				operatorId: endpoint.tenantId,
			})
			if (result.error) {
				toast.error(result.error)
			} else {
				toast.success(t.v1beta.repoSettings.integrationsTab.initialSyncStarted)
				router.refresh()
			}
		})
	}

	const handleCopy = async (value: string, label: string) => {
		try {
			await navigator.clipboard.writeText(value)
			toast.success(
				t.v1beta.repoSettings.integrationsTab.copySuccess.replace(
					'{label}',
					label,
				),
			)
		} catch {
			toast.error(
				t.v1beta.repoSettings.integrationsTab.copyFailed.replace(
					'{label}',
					label,
				),
			)
		}
	}

	return (
		<Card>
			<CardHeader>
				<div className='flex items-center justify-between'>
					<div className='flex items-center gap-3'>
						<div className='text-2xl'>📐</div>
						<div>
							<CardTitle>
								{t.v1beta.repoSettings.integrationsTab.linearTitle}
							</CardTitle>
							<CardDescription>
								{t.v1beta.repoSettings.integrationsTab.linearDescription}
							</CardDescription>
						</div>
					</div>
					<Badge variant={isConnected ? 'default' : 'secondary'}>
						{isConnected
							? t.v1beta.repoSettings.integrationsTab.connected
							: t.v1beta.repoSettings.integrationsTab.notConnected}
					</Badge>
				</div>
			</CardHeader>
			<CardContent className='space-y-6'>
				{!isConnected ? (
					<div className='space-y-4'>
						<p className='text-sm text-muted-foreground'>
							{t.v1beta.repoSettings.integrationsTab.connectDescription}
						</p>
						<div className='flex gap-2'>
							<Button onClick={handleConnect}>
								<ExternalLink className='mr-2 h-4 w-4' />
								{t.v1beta.repoSettings.integrationsTab.connectAction}
							</Button>
						</div>
					</div>
				) : (
					<div className='space-y-6'>
						<div className='space-y-2'>
							<h3 className='text-sm font-medium'>
								{t.v1beta.repoSettings.integrationsTab.connectionStatus}
							</h3>
							<div className='flex items-center gap-2 text-sm'>
								<div className='h-2 w-2 rounded-full bg-green-500' />
								<span>
									{t.v1beta.repoSettings.integrationsTab.connectedToWorkspace}
									{connection?.externalAccountName
										? ` (${connection.externalAccountName})`
										: ''}
								</span>
							</div>
						</div>

						{!tenantId || !repoId ? (
							<div className='rounded-lg border border-dashed border-muted p-4 text-sm text-muted-foreground'>
								{t.v1beta.repoSettings.integrationsTab.missingInfo}
							</div>
						) : (
							<>
								<div className='space-y-4'>
									<div className='flex items-center justify-between'>
										<div>
											<h3 className='text-sm font-medium'>
												{
													t.v1beta.repoSettings.integrationsTab
														.syncConfiguration
												}
											</h3>
											<p className='text-sm text-muted-foreground'>
												{
													t.v1beta.repoSettings.integrationsTab
														.syncConfigurationDescription
												}
											</p>
										</div>
										{endpoint ? (
											<div className='flex items-center gap-2 text-sm'>
												<span className='text-muted-foreground'>
													{t.v1beta.repoSettings.integrationsTab.enabled}
												</span>
												<Switch
													checked={endpoint.status === 'ACTIVE'}
													onCheckedChange={handleToggleStatus}
													disabled={isPending}
												/>
											</div>
										) : null}
									</div>

									<div className='grid gap-4 md:grid-cols-2'>
										<div className='space-y-2'>
											<Label htmlFor='linear-team'>
												{t.v1beta.repoSettings.integrationsTab.team}
											</Label>
											<Select
												value={selectedTeamId}
												onValueChange={value => {
													setSelectedTeamId(value)
													setSelectedProjectId('all')
												}}
												disabled={isLoadingTeams}
											>
												<SelectTrigger id='linear-team'>
													<SelectValue
														placeholder={
															isLoadingTeams
																? t.v1beta.repoSettings.integrationsTab
																		.loadingTeams
																: t.v1beta.repoSettings.integrationsTab.allTeams
														}
													/>
												</SelectTrigger>
												<SelectContent>
													<SelectItem value='all'>
														{t.v1beta.repoSettings.integrationsTab.allTeams}
													</SelectItem>
													{teams.map(team => (
														<SelectItem key={team.id} value={team.id}>
															{team.name} ({team.key})
														</SelectItem>
													))}
												</SelectContent>
											</Select>
										</div>

										<div className='space-y-2'>
											<Label htmlFor='linear-project'>
												{t.v1beta.repoSettings.integrationsTab.project}
											</Label>
											<Select
												value={selectedProjectId}
												onValueChange={setSelectedProjectId}
												disabled={isLoadingProjects}
											>
												<SelectTrigger id='linear-project'>
													<SelectValue
														placeholder={
															isLoadingProjects
																? t.v1beta.repoSettings.integrationsTab
																		.loadingProjects
																: t.v1beta.repoSettings.integrationsTab
																		.allProjects
														}
													/>
												</SelectTrigger>
												<SelectContent>
													<SelectItem value='all'>
														{t.v1beta.repoSettings.integrationsTab.allProjects}
													</SelectItem>
													{projects.map(project => (
														<SelectItem key={project.id} value={project.id}>
															{project.name}
														</SelectItem>
													))}
												</SelectContent>
											</Select>
										</div>
									</div>

									<div className='space-y-2'>
										<Label>
											{t.v1beta.repoSettings.integrationsTab.events}
										</Label>
										<div className='flex flex-wrap gap-2'>
											{linearEventOptions.map(option => (
												<Button
													key={option.value}
													type='button'
													variant={
														selectedEvents.includes(option.value)
															? 'default'
															: 'outline'
													}
													size='sm'
													onClick={() => {
														setSelectedEvents(prev =>
															prev.includes(option.value)
																? prev.filter(v => v !== option.value)
																: [...prev, option.value],
														)
													}}
												>
													{option.label}
												</Button>
											))}
										</div>
									</div>

									<div className='space-y-2'>
										<Label htmlFor='linear-webhook-secret'>
											{t.v1beta.repoSettings.integrationsTab.webhookSecretLabel}
										</Label>
										<Input
											id='linear-webhook-secret'
											type='password'
											placeholder={
												t.v1beta.repoSettings.integrationsTab
													.webhookSecretPlaceholder
											}
											value={webhookSecret}
											onChange={event => setWebhookSecret(event.target.value)}
										/>
										<p className='text-xs text-muted-foreground'>
											{
												t.v1beta.repoSettings.integrationsTab
													.webhookSecretDescription
											}
										</p>
									</div>

									<div className='flex gap-2'>
										{endpoint ? (
											<Button
												variant='outline'
												onClick={handleUpdateConfig}
												disabled={isPending}
											>
												<Settings className='mr-2 h-4 w-4' />
												{
													t.v1beta.repoSettings.integrationsTab
														.updateConfiguration
												}
											</Button>
										) : (
											<Button
												onClick={handleCreateEndpoint}
												disabled={isPending}
											>
												<Settings className='mr-2 h-4 w-4' />
												{t.v1beta.repoSettings.integrationsTab.enableSync}
											</Button>
										)}
									</div>
								</div>

								{createdWebhook ? (
									<div className='rounded-lg border border-dashed border-muted p-4 space-y-2 text-sm'>
										<p className='font-medium'>
											{
												t.v1beta.repoSettings.integrationsTab
													.webhookEndpointCreated
											}
										</p>
										<div className='flex items-center justify-between gap-2'>
											<div className='truncate text-muted-foreground'>
												{t.v1beta.repoSettings.integrationsTab.webhookUrlLabel}:{' '}
												{createdWebhook.webhookUrl}
											</div>
											<Button
												variant='ghost'
												size='sm'
												onClick={() =>
													handleCopy(
														createdWebhook.webhookUrl,
														t.v1beta.repoSettings.integrationsTab
															.webhookUrlLabel,
													)
												}
											>
												{t.v1beta.repoSettings.integrationsTab.copy}
											</Button>
										</div>
										{createdWebhook.secret ? (
											<>
												<div className='flex items-center justify-between gap-2'>
													<div className='truncate text-muted-foreground'>
														{t.v1beta.repoSettings.integrationsTab.secretLabel}:{' '}
														{createdWebhook.secret}
													</div>
													<Button
														variant='ghost'
														size='sm'
														onClick={() =>
															handleCopy(
																createdWebhook.secret,
																t.v1beta.repoSettings.integrationsTab
																	.secretLabel,
															)
														}
													>
														{t.v1beta.repoSettings.integrationsTab.copy}
													</Button>
												</div>
												<p className='text-xs text-muted-foreground'>
													{t.v1beta.repoSettings.integrationsTab.secretNotice}
												</p>
											</>
										) : (
											<p className='text-xs text-muted-foreground'>
												{
													t.v1beta.repoSettings.integrationsTab
														.secretReuseNotice
												}
											</p>
										)}
									</div>
								) : null}

								{endpoint ? (
									<div className='space-y-4'>
										<div className='space-y-2'>
											<h3 className='text-sm font-medium'>
												{t.v1beta.repoSettings.integrationsTab.syncControls}
											</h3>
											<div className='flex flex-wrap gap-2'>
												<Button
													onClick={handleInitialSync}
													disabled={isPending}
												>
													{
														t.v1beta.repoSettings.integrationsTab
															.startInitialSync
													}
												</Button>
												<Button
													variant='outline'
													onClick={() => setMappingOpen(true)}
												>
													{
														t.v1beta.repoSettings.integrationsTab
															.configureMapping
													}
												</Button>
											</div>
										</div>

										<div className='space-y-2'>
											<h3 className='text-sm font-medium'>
												{t.v1beta.repoSettings.integrationsTab.syncHistory}
											</h3>
											<SyncHistory endpointId={endpoint.id} />
										</div>
									</div>
								) : null}

								{endpoint ? (
									<div className='space-y-2 text-sm'>
										<h3 className='text-sm font-medium'>
											{t.v1beta.repoSettings.integrationsTab.webhookUrlHeading}
										</h3>
										<div className='flex items-center justify-between gap-2 rounded-lg border border-muted px-3 py-2'>
											<span className='truncate text-muted-foreground'>
												{endpoint.webhookUrl}
											</span>
											<Button
												variant='ghost'
												size='sm'
												onClick={() =>
													handleCopy(
														endpoint.webhookUrl,
														t.v1beta.repoSettings.integrationsTab
															.webhookUrlLabel,
													)
												}
											>
												{t.v1beta.repoSettings.integrationsTab.copy}
											</Button>
										</div>
									</div>
								) : null}
							</>
						)}

						<div className='space-y-2'>
							<h3 className='text-sm font-medium'>
								{t.v1beta.repoSettings.integrationsTab.extLinearTitle}
							</h3>
							<p className='text-sm text-muted-foreground'>
								{
									t.v1beta.repoSettings.integrationsTab
										.extLinearDescriptionPrefix
								}{' '}
								<code className='text-xs bg-muted px-1 py-0.5 rounded'>
									ext_linear
								</code>{' '}
								{
									t.v1beta.repoSettings.integrationsTab
										.extLinearDescriptionSuffix
								}
							</p>
							<ul className='text-sm text-muted-foreground list-disc list-inside space-y-1 ml-2'>
								<li>
									{
										t.v1beta.repoSettings.integrationsTab.extLinearFields
											.issueId
									}
								</li>
								<li>
									{
										t.v1beta.repoSettings.integrationsTab.extLinearFields
											.issueUrl
									}
								</li>
								<li>
									{
										t.v1beta.repoSettings.integrationsTab.extLinearFields
											.identifier
									}
								</li>
								<li>
									{
										t.v1beta.repoSettings.integrationsTab.extLinearFields
											.syncEnabled
									}
								</li>
								<li>
									{
										t.v1beta.repoSettings.integrationsTab.extLinearFields
											.lastSyncedAt
									}
								</li>
							</ul>
						</div>
					</div>
				)}
			</CardContent>

			<Dialog open={mappingOpen} onOpenChange={setMappingOpen}>
				<DialogContent className='max-w-4xl'>
					<DialogHeader>
						<DialogTitle>
							{t.v1beta.repoSettings.integrationsTab.propertyMappingTitle}
						</DialogTitle>
						<DialogDescription>
							{t.v1beta.repoSettings.integrationsTab.propertyMappingDescription}
						</DialogDescription>
					</DialogHeader>
					{endpoint ? (
						<MappingEditor
							endpointId={endpoint.id}
							initialMapping={endpoint.mapping ?? null}
							provider='linear'
							operatorId={endpoint.tenantId}
						/>
					) : null}
					<DialogFooter>
						<Button variant='outline' onClick={() => setMappingOpen(false)}>
							{t.v1beta.repoSettings.integrationsTab.close}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</Card>
	)
}
