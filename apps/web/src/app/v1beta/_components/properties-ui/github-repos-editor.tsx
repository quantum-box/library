'use client'

import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from '@/components/ui/command'
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
	Popover,
	PopoverContent,
	PopoverTrigger,
} from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import {
	Check,
	ChevronsUpDown,
	Github,
	Loader2,
	Plus,
	Trash2,
} from 'lucide-react'
import { useCallback, useEffect, useState, useTransition } from 'react'
import { listGitHubRepositories } from '../../[org]/_components/github-settings-actions'

/**
 * GitHub repository configuration
 */
export interface GitHubRepoConfig {
	id: string
	repo: string
	label?: string
	/** Default file path for this repository */
	defaultPath?: string
}

interface GitHubRepoFromApi {
	id: string
	name: string
	fullName: string
	description?: string | null
	private: boolean
	htmlUrl: string
	defaultBranch?: string | null
}

interface GitHubReposEditorDialogProps {
	isOpen: boolean
	onClose: () => void
	/** Current value as JSON string */
	value?: string
	/** Called when saved with new JSON string value */
	onSave: (jsonValue: string) => void
	/** Whether GitHub is connected */
	isGitHubConnected?: boolean
	/** Called when bulk sync is requested with the repo configs */
	onBulkSync?: (repoConfigs: GitHubRepoConfig[]) => Promise<void>
	/** Total number of data items to sync */
	totalDataCount?: number
}

/**
 * Combobox for selecting GitHub repository
 */
function RepoCombobox({
	value,
	onSelect,
	disabled,
}: {
	value: string
	onSelect: (repo: string) => void
	disabled?: boolean
}) {
	const [open, setOpen] = useState(false)
	const [searchQuery, setSearchQuery] = useState('')
	const [repos, setRepos] = useState<GitHubRepoFromApi[]>([])
	const [isPending, startTransition] = useTransition()
	const [hasSearched, setHasSearched] = useState(false)

	const searchRepos = useCallback((query: string) => {
		startTransition(async () => {
			try {
				const result = await listGitHubRepositories(query, 30, 1)
				setRepos(result)
				setHasSearched(true)
			} catch (error) {
				console.error('Failed to search repositories:', error)
				setRepos([])
			}
		})
	}, [])

	// Initial load when opening
	useEffect(() => {
		if (open && !hasSearched) {
			searchRepos('')
		}
	}, [open, hasSearched, searchRepos])

	// Debounced search
	useEffect(() => {
		if (!open) return

		const timer = setTimeout(() => {
			searchRepos(searchQuery)
		}, 300)

		return () => clearTimeout(timer)
	}, [searchQuery, open, searchRepos])

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant='outline'
					// biome-ignore lint/a11y/useSemanticElements: Custom combobox pattern requires ARIA role
					role='combobox'
					aria-expanded={open}
					className='w-full justify-between font-mono text-sm'
					disabled={disabled}
				>
					{value || 'Select repository...'}
					<ChevronsUpDown className='ml-2 h-4 w-4 shrink-0 opacity-50' />
				</Button>
			</PopoverTrigger>
			<PopoverContent className='w-[400px] p-0' align='start'>
				<Command shouldFilter={false}>
					<CommandInput
						placeholder='Search repositories...'
						value={searchQuery}
						onValueChange={setSearchQuery}
					/>
					<CommandList>
						{isPending ? (
							<div className='flex items-center justify-center py-6'>
								<Loader2 className='h-4 w-4 animate-spin' />
								<span className='ml-2 text-sm text-muted-foreground'>
									Searching...
								</span>
							</div>
						) : repos.length === 0 ? (
							<CommandEmpty>
								{hasSearched
									? 'No repositories found.'
									: 'Type to search repositories...'}
							</CommandEmpty>
						) : (
							<CommandGroup>
								{repos.map(repo => (
									<CommandItem
										key={repo.id}
										value={repo.fullName}
										onSelect={() => {
											onSelect(repo.fullName)
											setOpen(false)
										}}
									>
										<Check
											className={cn(
												'mr-2 h-4 w-4',
												value === repo.fullName ? 'opacity-100' : 'opacity-0',
											)}
										/>
										<div className='flex flex-col'>
											<div className='flex items-center gap-2'>
												<Github className='h-3 w-3' />
												<span className='font-mono text-sm'>
													{repo.fullName}
												</span>
												{repo.private && (
													<span className='rounded bg-muted px-1 py-0.5 text-[10px] text-muted-foreground'>
														Private
													</span>
												)}
											</div>
											{repo.description && (
												<span className='text-xs text-muted-foreground line-clamp-1'>
													{repo.description}
												</span>
											)}
										</div>
									</CommandItem>
								))}
							</CommandGroup>
						)}
					</CommandList>
				</Command>
			</PopoverContent>
		</Popover>
	)
}

/**
 * Dialog for editing ext_github property
 * Allows adding/editing/removing GitHub repository configurations
 */
export function GitHubReposEditorDialog({
	isOpen,
	onClose,
	value,
	onSave,
	isGitHubConnected = false,
	onBulkSync,
	totalDataCount = 0,
}: GitHubReposEditorDialogProps) {
	const [repos, setRepos] = useState<GitHubRepoConfig[]>([])
	const [showBulkSyncConfirm, setShowBulkSyncConfirm] = useState(false)
	const [isBulkSyncing, setIsBulkSyncing] = useState(false)
	const [savedJson, setSavedJson] = useState<string>('')

	// Parse initial value
	useEffect(() => {
		if (value) {
			try {
				const parsed = JSON.parse(value)
				if (Array.isArray(parsed)) {
					setRepos(
						parsed.map((item, idx) => ({
							id: item.id || `repo-${idx}`,
							repo: item.repo || '',
							label: item.label || '',
							defaultPath: item.defaultPath || '',
						})),
					)
				}
			} catch {
				setRepos([])
			}
		} else {
			setRepos([])
		}
	}, [value, isOpen])

	const addRepo = () => {
		setRepos(prev => [
			...prev,
			{ id: `repo-${Date.now()}`, repo: '', label: '', defaultPath: '' },
		])
	}

	const removeRepo = (id: string) => {
		setRepos(prev => prev.filter(r => r.id !== id))
	}

	const updateRepo = (
		id: string,
		field: 'repo' | 'label' | 'defaultPath',
		fieldValue: string,
	) => {
		setRepos(prev =>
			prev.map(r => (r.id === id ? { ...r, [field]: fieldValue } : r)),
		)
	}

	const handleSave = () => {
		// Filter out empty repos and serialize
		const validRepos = repos.filter(r => r.repo.trim())
		const json = JSON.stringify(validRepos)
		onSave(json)

		// If bulk sync is available and there are data items, show confirmation
		if (onBulkSync && totalDataCount > 0 && validRepos.length > 0) {
			setSavedJson(json)
			setShowBulkSyncConfirm(true)
		} else {
			onClose()
		}
	}

	const handleBulkSync = async () => {
		if (!onBulkSync) return
		setIsBulkSyncing(true)
		try {
			const validRepos = repos.filter(r => r.repo.trim())
			await onBulkSync(validRepos)
			setShowBulkSyncConfirm(false)
			onClose()
		} catch (error) {
			console.error('Bulk sync failed:', error)
		} finally {
			setIsBulkSyncing(false)
		}
	}

	const skipBulkSync = () => {
		setShowBulkSyncConfirm(false)
		onClose()
	}

	return (
		<Dialog open={isOpen} onOpenChange={onClose}>
			<DialogContent className='max-w-lg'>
				<DialogHeader>
					<DialogTitle className='flex items-center gap-2'>
						<Github className='h-5 w-5' />
						Configure GitHub Repositories
					</DialogTitle>
					<DialogDescription>
						Add GitHub repositories that can be used as sync destinations for
						data in this repository.
					</DialogDescription>
				</DialogHeader>

				<div className='space-y-4 py-4'>
					{!isGitHubConnected && (
						<div className='rounded-lg border border-amber-200 bg-amber-50 p-3 dark:border-amber-800 dark:bg-amber-950/30'>
							<p className='text-sm text-amber-800 dark:text-amber-200'>
								GitHub is not connected. Connect your GitHub account in
								Organization Settings to search repositories.
							</p>
						</div>
					)}

					<div className='space-y-3'>
						{repos.map(config => (
							<div
								key={config.id}
								className='flex items-start gap-2 rounded-lg border p-3'
							>
								<div className='flex-1 space-y-2'>
									<div>
										<Label className='text-xs text-muted-foreground'>
											Repository (owner/repo)
										</Label>
										{isGitHubConnected ? (
											<div className='mt-1'>
												<RepoCombobox
													value={config.repo}
													onSelect={repo => updateRepo(config.id, 'repo', repo)}
												/>
											</div>
										) : (
											<Input
												value={config.repo}
												onChange={e =>
													updateRepo(config.id, 'repo', e.target.value)
												}
												placeholder='owner/repository'
												className='mt-1 font-mono text-sm'
											/>
										)}
									</div>
									<div>
										<Label className='text-xs text-muted-foreground'>
											Label (optional)
										</Label>
										<Input
											value={config.label || ''}
											onChange={e =>
												updateRepo(config.id, 'label', e.target.value)
											}
											placeholder='Display name'
											className='mt-1 text-sm'
										/>
									</div>
									<div>
										<Label className='text-xs text-muted-foreground'>
											Default path (optional)
										</Label>
										<Input
											value={config.defaultPath || ''}
											onChange={e =>
												updateRepo(config.id, 'defaultPath', e.target.value)
											}
											placeholder='docs/{{name}}.md'
											className='mt-1 font-mono text-sm'
										/>
										<p className='mt-1 text-[10px] text-muted-foreground'>
											Use {'{{name}}'} as placeholder for data name
										</p>
									</div>
								</div>
								<Button
									type='button'
									variant='ghost'
									size='icon'
									onClick={() => removeRepo(config.id)}
									className='mt-6 text-destructive hover:bg-destructive/10'
								>
									<Trash2 className='h-4 w-4' />
								</Button>
							</div>
						))}
					</div>

					<Button
						type='button'
						variant='outline'
						onClick={addRepo}
						className='w-full'
					>
						<Plus className='mr-2 h-4 w-4' />
						Add Repository
					</Button>
				</div>

				<DialogFooter>
					<Button type='button' variant='outline' onClick={onClose}>
						Cancel
					</Button>
					<Button type='button' onClick={handleSave}>
						Save
					</Button>
				</DialogFooter>
			</DialogContent>

			{/* Bulk Sync Confirmation Dialog */}
			<AlertDialog
				open={showBulkSyncConfirm}
				onOpenChange={setShowBulkSyncConfirm}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle className='flex items-center gap-2'>
							<Github className='h-5 w-5' />
							Sync all existing data?
						</AlertDialogTitle>
						<AlertDialogDescription>
							Would you like to configure GitHub sync for all{' '}
							<strong>{totalDataCount}</strong> existing data items? This will
							set up the default repository and path for each data item.
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel onClick={skipBulkSync} disabled={isBulkSyncing}>
							Skip
						</AlertDialogCancel>
						<AlertDialogAction
							onClick={handleBulkSync}
							disabled={isBulkSyncing}
						>
							{isBulkSyncing ? (
								<>
									<Loader2 className='mr-2 h-4 w-4 animate-spin' />
									Syncing...
								</>
							) : (
								'Sync All'
							)}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</Dialog>
	)
}

/**
 * Inline display for ext_github value in the properties table
 */
export function GitHubReposDisplay({ value }: { value?: string }) {
	if (!value) {
		return (
			<span className='text-xs text-muted-foreground'>No repositories</span>
		)
	}

	try {
		const repos = JSON.parse(value) as GitHubRepoConfig[]
		if (!repos.length) {
			return (
				<span className='text-xs text-muted-foreground'>No repositories</span>
			)
		}

		return (
			<div className='flex flex-wrap gap-1'>
				{repos.map(r => (
					<span
						key={r.id}
						className='inline-flex items-center gap-1 rounded bg-muted px-2 py-0.5 text-xs'
					>
						<Github className='h-3 w-3' />
						{r.label || r.repo}
					</span>
				))}
			</div>
		)
	} catch {
		return <span className='text-xs text-destructive'>Invalid JSON</span>
	}
}
