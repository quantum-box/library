'use client'

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from '@/components/ui/dialog'
import {
	Form,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from '@/components/ui/form'
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { useToast } from '@/components/ui/use-toast'
import type { RepoFieldOnRepoSettingsPageFragment } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import {
	AlertCircle,
	CheckCircle2,
	Github,
	Loader2,
	Pencil,
	Plus,
	Trash2,
	UserPlus,
	Users,
} from 'lucide-react'
import { useRouter } from 'next/navigation'
import { parseAsString, useQueryState } from 'nuqs'
import { useCallback, useEffect, useMemo, useState, useTransition } from 'react'
import { useForm } from 'react-hook-form'
import * as z from 'zod'
import {
	type RepoMember,
	changeRepoMemberRoleAction,
	changeRepoUsernameAction,
	deleteRepoAction,
	disableGitHubSyncAction,
	enableGitHubSyncAction,
	getRepoMembersAction,
	inviteRepoMemberAction,
	removeRepoMemberAction,
	updateRepoSettingsAction,
} from './actions'
import { LinearExtensionSettings } from './extensions/linear-extension-settings'

interface ChangeUsernameFormData {
	username: string
}

interface ChangeUsernameDialogProps {
	orgUsername: string
	currentUsername: string
	onUsernameChanged: (newUsername: string) => void
}

function ChangeUsernameDialog({
	orgUsername,
	currentUsername,
	onUsernameChanged,
}: ChangeUsernameDialogProps) {
	const { t } = useTranslation()
	const usernameSchema = useMemo(
		() =>
			z.object({
				username: z
					.string()
					.min(3, {
						message: t.v1beta.repoSettings.validation.usernameMinLength,
					})
					.max(40, {
						message: t.v1beta.repoSettings.validation.usernameMaxLength,
					})
					.regex(/^[a-zA-Z0-9-_]+$/, {
						message: t.v1beta.repoSettings.validation.usernameInvalidFormat,
					}),
			}),
		[t],
	)
	const form = useForm<ChangeUsernameFormData>({
		resolver: zodResolver(usernameSchema),
		defaultValues: {
			username: currentUsername,
		},
	})

	const { toast } = useToast()

	const onSubmit = async (data: ChangeUsernameFormData) => {
		try {
			await changeRepoUsernameAction({
				orgUsername,
				oldRepoUsername: currentUsername,
				newRepoUsername: data.username,
			})

			toast({
				title: t.v1beta.repoSettings.usernameUpdated,
				description: t.v1beta.repoSettings.usernameUpdatedDescription,
			})
			onUsernameChanged(data.username)
		} catch (error) {
			toast({
				title: t.common.error,
				description:
					error instanceof Error
						? error.message
						: 'Failed to update repository username.',
				variant: 'destructive',
			})
		}
	}

	return (
		<AlertDialog>
			<AlertDialogTrigger asChild>
				<Button variant='outline' size='sm'>
					<Pencil className='mr-2 h-4 w-4' />
					{t.v1beta.repoSettings.changeUsername}
				</Button>
			</AlertDialogTrigger>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>
						{t.v1beta.repoSettings.changeUsernameTitle}
					</AlertDialogTitle>
					<AlertDialogDescription>
						{t.v1beta.repoSettings.changeUsernameDescription}
					</AlertDialogDescription>
				</AlertDialogHeader>
				<Form {...form}>
					<form onSubmit={form.handleSubmit(onSubmit)} className='space-y-4'>
						<FormField
							control={form.control}
							name='username'
							render={({ field }) => (
								<FormItem>
									<FormLabel>{t.v1beta.repoSettings.newUsername}</FormLabel>
									<FormControl>
										<Input {...field} />
									</FormControl>
									<FormDescription>
										{t.v1beta.repoSettings.newUsernameDescription}
									</FormDescription>
									<FormMessage />
								</FormItem>
							)}
						/>
						<AlertDialogFooter>
							<AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
							<AlertDialogAction type='submit'>
								{t.v1beta.repoSettings.changeUsername}
							</AlertDialogAction>
						</AlertDialogFooter>
					</form>
				</Form>
			</AlertDialogContent>
		</AlertDialog>
	)
}

interface InviteMemberDialogProps {
	orgUsername: string
	repoUsername: string
	repoId: string
	hasOwner: boolean
	onMemberInvited: () => void
}

function InviteMemberDialog({
	orgUsername,
	repoUsername,
	repoId,
	hasOwner,
	onMemberInvited,
}: InviteMemberDialogProps) {
	const { t } = useTranslation()
	const { toast } = useToast()
	const [open, setOpen] = useState(false)
	const [loading, setLoading] = useState(false)
	const [username, setUsername] = useState('')
	const [selectedRole, setSelectedRole] = useState<
		'owner' | 'writer' | 'reader'
	>('reader')

	// Reset role to reader if owner is selected but owner already exists
	useEffect(() => {
		if (hasOwner && selectedRole === 'owner') {
			setSelectedRole('reader')
		}
	}, [hasOwner, selectedRole])

	const handleInvite = async () => {
		if (!username.trim()) return

		setLoading(true)
		try {
			await inviteRepoMemberAction({
				orgUsername,
				repoUsername,
				repoId,
				usernameOrEmail: username.trim(),
				role: selectedRole,
			})

			toast({
				title: 'Member invited',
				description: 'The user has been granted access to this repository.',
			})
			setOpen(false)
			setUsername('')
			setSelectedRole('reader')
			onMemberInvited()
		} catch (error) {
			toast({
				title: t.common.error,
				description:
					error instanceof Error ? error.message : 'Failed to invite member',
				variant: 'destructive',
			})
		} finally {
			setLoading(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={setOpen}>
			<DialogTrigger asChild>
				<Button variant='default' size='sm'>
					<UserPlus className='mr-2 h-4 w-4' />
					Invite Member
				</Button>
			</DialogTrigger>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Invite Member</DialogTitle>
					<DialogDescription>
						Invite a user by their username. They will be granted access to this
						repository only, without requiring organization membership.
					</DialogDescription>
				</DialogHeader>
				<div className='space-y-4 py-4'>
					<div className='space-y-2'>
						<Label htmlFor='username'>Username</Label>
						<Input
							id='username'
							placeholder='Enter username'
							value={username}
							onChange={e => setUsername(e.target.value)}
							onKeyDown={e => {
								if (e.key === 'Enter' && username.trim()) {
									handleInvite()
								}
							}}
						/>
						<p className='text-xs text-muted-foreground'>
							The user must have an account in the system.
						</p>
					</div>
					<div className='space-y-2'>
						<Label>{t.v1beta.repoSettings.memberRole.label}</Label>
						<Select
							value={selectedRole}
							onValueChange={value =>
								setSelectedRole(value as 'owner' | 'writer' | 'reader')
							}
						>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value='reader'>
									<div className='flex flex-col'>
										<span className='font-medium'>
											{t.v1beta.repoSettings.memberRole.roles.reader.name}
										</span>
										<span className='text-xs text-muted-foreground'>
											{
												t.v1beta.repoSettings.memberRole.roles.reader
													.description
											}
										</span>
									</div>
								</SelectItem>
								<SelectItem value='writer'>
									<div className='flex flex-col'>
										<span className='font-medium'>
											{t.v1beta.repoSettings.memberRole.roles.writer.name}
										</span>
										<span className='text-xs text-muted-foreground'>
											{
												t.v1beta.repoSettings.memberRole.roles.writer
													.description
											}
										</span>
									</div>
								</SelectItem>
								{!hasOwner && (
									<SelectItem value='owner'>
										<div className='flex flex-col'>
											<span className='font-medium'>
												{t.v1beta.repoSettings.memberRole.roles.owner.name}
											</span>
											<span className='text-xs text-muted-foreground'>
												{
													t.v1beta.repoSettings.memberRole.roles.owner
														.description
												}
											</span>
										</div>
									</SelectItem>
								)}
							</SelectContent>
						</Select>
					</div>
				</div>
				<DialogFooter>
					<Button variant='outline' onClick={() => setOpen(false)}>
						{t.common.cancel}
					</Button>
					<Button onClick={handleInvite} disabled={loading || !username.trim()}>
						{loading ? (
							<Loader2 className='mr-2 h-4 w-4 animate-spin' />
						) : (
							<Plus className='mr-2 h-4 w-4' />
						)}
						Invite
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

const generalSettingsSchema = z.object({
	name: z.string().min(1, 'Repository name is required'),
	description: z.string(),
	isPublic: z.boolean(),
	defaultBranch: z.string(),
})

type GeneralSettingsFormData = z.infer<typeof generalSettingsSchema>

interface SettingsFormProps {
	repo: RepoFieldOnRepoSettingsPageFragment
	params: {
		org: string
		repo: string
	}
	hasGitHubSync?: boolean
	currentUserId: string
	tenantId?: string | null
	repoId?: string | null
	linearConnection?: {
		id: string
		provider: string
		status: string
		externalAccountId?: string | null
		externalAccountName?: string | null
	} | null
	linearEndpoint?: {
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
	} | null
}

export function SettingsForm({
	repo,
	params,
	hasGitHubSync = false,
	currentUserId,
	tenantId,
	repoId,
	linearConnection,
	linearEndpoint,
}: SettingsFormProps) {
	const { t } = useTranslation()
	const { toast } = useToast()
	const router = useRouter()

	const form = useForm<GeneralSettingsFormData>({
		resolver: zodResolver(generalSettingsSchema),
		defaultValues: {
			name: repo.name,
			description: repo.description || '',
			isPublic: repo.isPublic,
			defaultBranch: 'main',
		},
	})

	const onSubmit = async (data: GeneralSettingsFormData) => {
		try {
			await updateRepoSettingsAction({
				orgUsername: params.org,
				repoUsername: params.repo,
				name: data.name,
				description: data.description,
				isPublic: data.isPublic,
			})

			toast({
				title: t.v1beta.repoSettings.settingsUpdated,
				description: t.v1beta.repoSettings.settingsUpdatedDescription,
			})
			router.refresh()
		} catch (error) {
			toast({
				title: t.common.error,
				description:
					error instanceof Error
						? error.message
						: 'Failed to update repository settings.',
				variant: 'destructive',
			})
		}
	}

	const handleUsernameChanged = (newUsername: string) => {
		router.push(`/v1beta/${params.org}/${newUsername}/settings`)
	}

	const handleDeleteRepository = async () => {
		try {
			await deleteRepoAction(params.org, params.repo)

			toast({
				title: t.v1beta.repoSettings.repositoryDeleted,
				description: t.v1beta.repoSettings.repositoryDeletedDescription,
			})
			router.push(`/v1beta/${params.org}`)
		} catch (error) {
			toast({
				title: t.common.error,
				description:
					error instanceof Error
						? error.message
						: 'Failed to delete repository.',
				variant: 'destructive',
			})
		}
	}

	const [isPending, startTransition] = useTransition()
	const [githubSyncEnabled, setGithubSyncEnabled] = useState(hasGitHubSync)
	const [showDisableConfirm, setShowDisableConfirm] = useState(false)

	// Tab state with URL sync using nuqs
	const [tab, setTab] = useQueryState(
		'tab',
		parseAsString.withDefault('general'),
	)

	useEffect(() => {
		if (tab === 'extensions') {
			setTab('integrations')
		}
	}, [tab, setTab])

	// Members state
	const [members, setMembers] = useState<RepoMember[]>([])
	const [membersLoading, setMembersLoading] = useState(false)
	const [membersError, setMembersError] = useState<string | null>(null)
	const [membersLoaded, setMembersLoaded] = useState(false)

	// Load members when access tab is selected
	const loadMembers = useCallback(async () => {
		setMembersLoading(true)
		setMembersError(null)
		try {
			const result = await getRepoMembersAction({
				orgUsername: params.org,
				repoUsername: params.repo,
			})
			setMembers(result)
			setMembersLoaded(true)
		} catch (error) {
			setMembersError(
				error instanceof Error ? error.message : 'Failed to load members',
			)
		} finally {
			setMembersLoading(false)
		}
	}, [params.org, params.repo])

	// Auto-load members when access tab is selected
	useEffect(() => {
		if (tab === 'access' && !membersLoaded && !membersLoading) {
			loadMembers()
		}
	}, [tab, membersLoaded, membersLoading, loadMembers])

	// Helper to get role value from policy name (lowercase for Select compatibility)
	const getRoleFromPolicy = (policyName?: string | null) => {
		if (!policyName) return 'reader'
		if (policyName.includes('Owner')) return 'owner'
		if (policyName.includes('Writer')) return 'writer'
		if (policyName.includes('Reader')) return 'reader'
		return 'reader'
	}

	// Check if there is already an owner
	const hasOwner = useMemo(() => {
		return members.some(
			member => getRoleFromPolicy(member.policyName) === 'owner',
		)
	}, [members])

	// Check if current user is owner
	const isCurrentUserOwner = useMemo(() => {
		const currentUserMember = members.find(
			member => member.userId === currentUserId,
		)
		return (
			currentUserMember &&
			getRoleFromPolicy(currentUserMember.policyName) === 'owner'
		)
	}, [members, currentUserId])

	const handleToggleGitHubSync = (enabled: boolean) => {
		if (enabled) {
			handleEnableGitHubSync()
		} else {
			setShowDisableConfirm(true)
		}
	}

	const handleEnableGitHubSync = () => {
		startTransition(async () => {
			try {
				await enableGitHubSyncAction({
					orgUsername: params.org,
					repoUsername: params.repo,
				})

				setGithubSyncEnabled(true)
				toast({
					title: t.v1beta.repoSettings.githubSyncEnabledAlert,
					description: t.v1beta.repoSettings.githubSyncEnabledAlertDescription,
				})
				router.refresh()
			} catch (error) {
				toast({
					title: t.common.error,
					description:
						error instanceof Error
							? error.message
							: 'Failed to enable GitHub sync.',
					variant: 'destructive',
				})
			}
		})
	}

	const handleDisableGitHubSync = () => {
		startTransition(async () => {
			try {
				await disableGitHubSyncAction({
					orgUsername: params.org,
					repoUsername: params.repo,
				})

				setGithubSyncEnabled(false)
				setShowDisableConfirm(false)
				toast({
					title: t.v1beta.repoSettings.githubSyncDisabledToast,
					description: t.v1beta.repoSettings.githubSyncDisabledToastDescription,
				})
				router.refresh()
			} catch (error) {
				toast({
					title: t.common.error,
					description:
						error instanceof Error
							? error.message
							: 'Failed to disable GitHub sync.',
					variant: 'destructive',
				})
			}
		})
	}

	return (
		<Tabs value={tab} onValueChange={setTab} className='space-y-6'>
			<TabsList>
				<TabsTrigger value='general'>
					{t.v1beta.repoSettings.general}
				</TabsTrigger>
				<TabsTrigger value='integrations'>
					{t.v1beta.repoSettings.integrations}
				</TabsTrigger>
				<TabsTrigger value='access'>{t.v1beta.repoSettings.access}</TabsTrigger>
				<TabsTrigger value='advanced'>
					{t.v1beta.repoSettings.advanced}
				</TabsTrigger>
			</TabsList>

			<TabsContent value='general'>
				<Card>
					<CardHeader>
						<CardTitle>{t.v1beta.repoSettings.generalSettings}</CardTitle>
					</CardHeader>
					<CardContent>
						<div className='mb-6'>
							<div className='flex items-center justify-between'>
								<div>
									<h3 className='text-lg font-medium'>
										{t.v1beta.repoSettings.repositoryUsername}
									</h3>
									<p className='text-sm text-muted-foreground'>
										{t.v1beta.repoSettings.currentUsername}: {repo.username}
									</p>
								</div>
								<ChangeUsernameDialog
									orgUsername={params.org}
									currentUsername={params.repo}
									onUsernameChanged={handleUsernameChanged}
								/>
							</div>
						</div>
						<Form {...form}>
							<form
								onSubmit={form.handleSubmit(onSubmit)}
								className='space-y-6'
							>
								<FormField
									control={form.control}
									name='name'
									render={({ field }) => (
										<FormItem>
											<FormLabel>
												{t.v1beta.repoSettings.repositoryName}
											</FormLabel>
											<FormControl>
												<Input {...field} />
											</FormControl>
											<FormDescription>
												{t.v1beta.repoSettings.repositoryNameDescription}
											</FormDescription>
											<FormMessage />
										</FormItem>
									)}
								/>

								<FormField
									control={form.control}
									name='description'
									render={({ field }) => (
										<FormItem>
											<FormLabel>{t.v1beta.repoSettings.description}</FormLabel>
											<FormControl>
												<Textarea
													{...field}
													placeholder={
														t.v1beta.repoSettings.descriptionPlaceholder
													}
												/>
											</FormControl>
											<FormMessage />
										</FormItem>
									)}
								/>

								<FormField
									control={form.control}
									name='isPublic'
									render={({ field }) => (
										<FormItem className='flex flex-row items-center justify-between rounded-lg border p-4'>
											<div className='space-y-0.5'>
												<FormLabel className='text-base'>
													{t.v1beta.repoSettings.repositoryVisibility}
												</FormLabel>
												<FormDescription>
													{field.value
														? t.v1beta.repoSettings.publicRepository
														: t.v1beta.repoSettings.privateRepository}
												</FormDescription>
											</div>
											<FormControl>
												<Switch
													checked={field.value}
													onCheckedChange={field.onChange}
												/>
											</FormControl>
										</FormItem>
									)}
								/>

								<Button type='submit'>
									{t.v1beta.repoSettings.saveChanges}
								</Button>
							</form>
						</Form>
					</CardContent>
				</Card>
			</TabsContent>

			<TabsContent value='integrations'>
				<div className='space-y-6'>
					<Card>
						<CardHeader>
							<CardTitle className='flex items-center gap-2'>
								<Github className='h-5 w-5' />
								{t.v1beta.repoSettings.githubSync}
							</CardTitle>
							<CardDescription>
								{t.v1beta.repoSettings.githubSyncDescription}
							</CardDescription>
						</CardHeader>
						<CardContent className='space-y-6'>
							<div className='flex flex-row items-center justify-between rounded-lg border p-4'>
								<div className='space-y-0.5'>
									<Label className='text-base'>
										{t.v1beta.repoSettings.enableGithubSync}
									</Label>
									<p className='text-sm text-muted-foreground'>
										{githubSyncEnabled
											? t.v1beta.repoSettings.githubSyncEnabled
											: t.v1beta.repoSettings.githubSyncDisabled}
									</p>
								</div>
								<Switch
									checked={githubSyncEnabled}
									onCheckedChange={handleToggleGitHubSync}
									disabled={isPending}
								/>
							</div>

							{githubSyncEnabled ? (
								<Alert>
									<CheckCircle2 className='h-4 w-4 text-green-500' />
									<AlertTitle>
										{t.v1beta.repoSettings.githubSyncEnabledAlert}
									</AlertTitle>
									<AlertDescription>
										<code className='rounded bg-muted px-1'>ext_github</code>{' '}
										{
											t.v1beta.repoSettings.githubSyncEnabledAlertDescriptionWithLink.split(
												'{link}',
											)[0]
										}
										<a
											href={`/v1beta/${params.org}/${params.repo}/properties`}
											className='font-medium text-primary underline'
										>
											{t.v1beta.repoSettings.propertiesPage}
										</a>
										{
											t.v1beta.repoSettings.githubSyncEnabledAlertDescriptionWithLink.split(
												'{link}',
											)[1]
										}
									</AlertDescription>
								</Alert>
							) : (
								<Alert>
									<AlertCircle className='h-4 w-4' />
									<AlertTitle>{t.v1beta.repoSettings.howItWorks}</AlertTitle>
									<AlertDescription className='space-y-2'>
										<p>{t.v1beta.repoSettings.howItWorksDescription}</p>
										<ul className='list-inside list-disc text-sm'>
											<li>
												{t.v1beta.repoSettings.howItWorksList.configureSync}
											</li>
											<li>
												{t.v1beta.repoSettings.howItWorksList.specifyTarget}
											</li>
											<li>{t.v1beta.repoSettings.howItWorksList.autoSync}</li>
											<li>
												{t.v1beta.repoSettings.howItWorksList.exportFormat}
											</li>
										</ul>
									</AlertDescription>
								</Alert>
							)}
						</CardContent>
					</Card>
					<LinearExtensionSettings
						org={params.org}
						repo={params.repo}
						tenantId={tenantId}
						repoId={repoId}
						connection={linearConnection ?? undefined}
						endpoint={linearEndpoint ?? undefined}
					/>
				</div>

				{/* Disable confirmation dialog */}
				<AlertDialog
					open={showDisableConfirm}
					onOpenChange={setShowDisableConfirm}
				>
					<AlertDialogContent>
						<AlertDialogHeader>
							<AlertDialogTitle>
								{t.v1beta.repoSettings.disableGithubSync}
							</AlertDialogTitle>
							<AlertDialogDescription>
								{t.v1beta.repoSettings.disableGithubSyncDescription}
							</AlertDialogDescription>
						</AlertDialogHeader>
						<AlertDialogFooter>
							<AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
							<AlertDialogAction
								onClick={handleDisableGitHubSync}
								disabled={isPending}
							>
								{isPending
									? t.v1beta.repoSettings.disabling
									: t.v1beta.repoSettings.disable}
							</AlertDialogAction>
						</AlertDialogFooter>
					</AlertDialogContent>
				</AlertDialog>
			</TabsContent>

			<TabsContent value='access'>
				<Card>
					<CardHeader>
						<div className='flex items-center justify-between'>
							<div>
								<CardTitle className='flex items-center gap-2'>
									<Users className='h-5 w-5' />
									{t.v1beta.repoSettings.accessControl}
								</CardTitle>
								<CardDescription>
									Manage who has access to this repository
								</CardDescription>
							</div>
							<div className='flex items-center gap-2'>
								<InviteMemberDialog
									orgUsername={params.org}
									repoUsername={params.repo}
									repoId={repo.id}
									hasOwner={hasOwner}
									onMemberInvited={() => {
										setMembersLoaded(false)
										loadMembers()
									}}
								/>
								<Button
									variant='outline'
									size='sm'
									onClick={loadMembers}
									disabled={membersLoading}
								>
									{membersLoading ? (
										<Loader2 className='mr-2 h-4 w-4 animate-spin' />
									) : null}
									Refresh
								</Button>
							</div>
						</div>
					</CardHeader>
					<CardContent>
						<div className='space-y-4'>
							{membersError && (
								<Alert variant='destructive'>
									<AlertCircle className='h-4 w-4' />
									<AlertTitle>Error</AlertTitle>
									<AlertDescription>{membersError}</AlertDescription>
								</Alert>
							)}

							{membersLoading && members.length === 0 ? (
								<div className='flex items-center justify-center py-8'>
									<Loader2 className='h-8 w-8 animate-spin text-muted-foreground' />
								</div>
							) : members.length === 0 ? (
								<Alert>
									<AlertCircle className='h-4 w-4' />
									<AlertTitle>No members</AlertTitle>
									<AlertDescription>
										No users have been granted access to this repository yet.
										Click the "Invite Member" button above to add users.
									</AlertDescription>
								</Alert>
							) : (
								<div className='rounded-md border'>
									<Table>
										<TableHeader>
											<TableRow>
												<TableHead>{t.v1beta.common.members}</TableHead>
												<TableHead>
													{t.v1beta.repoSettings.memberRole.label}
												</TableHead>
												{isCurrentUserOwner && (
													<TableHead className='text-right'>
														{t.v1beta.repoSettings.memberRole.actions}
													</TableHead>
												)}
											</TableRow>
										</TableHeader>
										<TableBody>
											{members.map(member => (
												<TableRow key={`${member.userId}-${member.policyId}`}>
													<TableCell>
														<div className='flex items-center gap-3'>
															<Avatar className='h-10 w-10'>
																<AvatarImage
																	src={member.user?.image ?? undefined}
																	alt={member.user?.name ?? 'User'}
																/>
																<AvatarFallback>
																	{(
																		member.user?.name?.[0] ??
																		member.user?.email?.[0] ??
																		'U'
																	).toUpperCase()}
																</AvatarFallback>
															</Avatar>
															<div>
																<div className='font-medium'>
																	{member.user?.name ??
																		member.user?.email ??
																		member.userId}
																</div>
																{member.user?.email && member.user.name && (
																	<div className='text-sm text-muted-foreground'>
																		{member.user.email}
																	</div>
																)}
															</div>
														</div>
													</TableCell>
													<TableCell>
														{getRoleFromPolicy(member.policyName) ===
														'owner' ? (
															<span className='text-sm font-medium'>
																{
																	t.v1beta.repoSettings.memberRole.roles.owner
																		.name
																}
																{member.permissionSource === 'ORG' && (
																	<span className='ml-1 text-xs text-muted-foreground'>
																		(org)
																	</span>
																)}
															</span>
														) : isCurrentUserOwner &&
															member.userId !== currentUserId &&
															member.permissionSource !== 'ORG' ? (
															<Select
																value={getRoleFromPolicy(member.policyName)}
																onValueChange={async newRole => {
																	try {
																		await changeRepoMemberRoleAction({
																			orgUsername: params.org,
																			repoId: repo.id,
																			userId: member.userId,
																			newRole: newRole as
																				| 'owner'
																				| 'writer'
																				| 'reader',
																		})
																		const roleName =
																			t.v1beta.repoSettings.memberRole.roles[
																				newRole as 'owner' | 'writer' | 'reader'
																			].name
																		toast({
																			title:
																				t.v1beta.repoSettings.memberRole
																					.updated,
																			description:
																				t.v1beta.repoSettings.memberRole.updatedDescription.replace(
																					'{role}',
																					roleName,
																				),
																		})
																		loadMembers()
																	} catch (error) {
																		toast({
																			title: t.common.error,
																			description:
																				error instanceof Error
																					? error.message
																					: t.v1beta.repoSettings.memberRole
																							.updateFailed,
																			variant: 'destructive',
																		})
																	}
																}}
															>
																<SelectTrigger className='w-32'>
																	<SelectValue />
																</SelectTrigger>
																<SelectContent>
																	<SelectItem value='reader'>
																		{
																			t.v1beta.repoSettings.memberRole.roles
																				.reader.name
																		}
																	</SelectItem>
																	<SelectItem value='writer'>
																		{
																			t.v1beta.repoSettings.memberRole.roles
																				.writer.name
																		}
																	</SelectItem>
																	{(!hasOwner ||
																		getRoleFromPolicy(member.policyName) ===
																			'owner') && (
																		<SelectItem value='owner'>
																			{
																				t.v1beta.repoSettings.memberRole.roles
																					.owner.name
																			}
																		</SelectItem>
																	)}
																</SelectContent>
															</Select>
														) : (
															<span className='text-sm font-medium'>
																{
																	t.v1beta.repoSettings.memberRole.roles[
																		getRoleFromPolicy(member.policyName) as
																			| 'reader'
																			| 'writer'
																			| 'owner'
																	].name
																}
																{member.permissionSource === 'ORG' && (
																	<span className='ml-1 text-xs text-muted-foreground'>
																		(org)
																	</span>
																)}
															</span>
														)}
													</TableCell>
													{isCurrentUserOwner && (
														<TableCell className='text-right'>
															{member.userId !== currentUserId &&
																member.permissionSource !== 'ORG' && (
																	<Button
																		variant='ghost'
																		size='icon'
																		className='text-destructive hover:text-destructive'
																		onClick={async () => {
																			if (
																				!confirm(
																					'Are you sure you want to remove this member?',
																				)
																			)
																				return
																			try {
																				await removeRepoMemberAction({
																					orgUsername: params.org,
																					repoId: repo.id,
																					userId: member.userId,
																				})
																				toast({
																					title: 'Member removed',
																					description:
																						'The user has been removed from this repository.',
																				})
																				loadMembers()
																			} catch (error) {
																				toast({
																					title: t.common.error,
																					description:
																						error instanceof Error
																							? error.message
																							: 'Failed to remove member',
																					variant: 'destructive',
																				})
																			}
																		}}
																	>
																		<Trash2 className='h-4 w-4' />
																	</Button>
																)}
														</TableCell>
													)}
												</TableRow>
											))}
										</TableBody>
									</Table>
								</div>
							)}

							<Alert className='mt-4'>
								<AlertCircle className='h-4 w-4' />
								<AlertTitle>
									{t.v1beta.repoSettings.accessManagement}
								</AlertTitle>
								<AlertDescription>
									{t.v1beta.repoSettings.accessManagementDescription}
								</AlertDescription>
							</Alert>
						</div>
					</CardContent>
				</Card>
			</TabsContent>

			<TabsContent value='advanced'>
				<Card>
					<CardHeader>
						<CardTitle>{t.v1beta.repoSettings.dangerZone}</CardTitle>
					</CardHeader>
					<CardContent>
						<div className='space-y-4'>
							<Alert variant='destructive'>
								<AlertCircle className='h-4 w-4' />
								<AlertTitle>{t.v1beta.repoSettings.warning}</AlertTitle>
								<AlertDescription>
									{t.v1beta.repoSettings.warningDescription}
								</AlertDescription>
							</Alert>

							<AlertDialog>
								<AlertDialogTrigger asChild>
									<Button variant='destructive'>
										<Trash2 className='mr-2 h-4 w-4' />
										{t.v1beta.repoSettings.deleteRepository}
									</Button>
								</AlertDialogTrigger>
								<AlertDialogContent>
									<AlertDialogHeader>
										<AlertDialogTitle>
											{t.v1beta.repoSettings.deleteRepository}
										</AlertDialogTitle>
										<AlertDialogDescription>
											{t.v1beta.repoSettings.deleteRepositoryDescription}
										</AlertDialogDescription>
									</AlertDialogHeader>
									<AlertDialogFooter>
										<AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
										<AlertDialogAction
											onClick={handleDeleteRepository}
											className='bg-destructive text-destructive-foreground hover:bg-destructive/90'
										>
											{t.common.delete}
										</AlertDialogAction>
									</AlertDialogFooter>
								</AlertDialogContent>
							</AlertDialog>
						</div>
					</CardContent>
				</Card>
			</TabsContent>
		</Tabs>
	)
}
