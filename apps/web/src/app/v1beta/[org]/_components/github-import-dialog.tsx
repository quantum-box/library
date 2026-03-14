'use client'

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
import { Skeleton } from '@/components/ui/skeleton'
import type { GitHubRepository } from '@/gen/graphql'
import { PropertyType } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import {
	AlertCircle,
	AlertTriangle,
	ChevronRight,
	File,
	Folder,
	Github,
	Loader2,
	Upload,
} from 'lucide-react'
import { useRouter } from 'next/navigation'
import { useCallback, useEffect, useState, useTransition } from 'react'
import { toast } from 'sonner'
import {
	type FrontmatterProperty,
	type GitHubFileInfo,
	analyzeFrontmatter,
	importMarkdownFromGitHub,
	listDirectoryContents,
	listGitHubRepositories,
} from './github-import-actions'

interface GitHubImportDialogProps {
	org: string
	existingRepos?: Array<{ username: string }>
	onImportComplete?: () => void
}

type ImportStep = 'select-repo' | 'select-files' | 'configure' | 'importing'

export function GitHubImportDialog({
	org,
	existingRepos = [],
	onImportComplete,
}: GitHubImportDialogProps) {
	const { t } = useTranslation()
	const router = useRouter()
	const [isPending, startTransition] = useTransition()
	const [open, setOpen] = useState(false)
	const [step, setStep] = useState<ImportStep>('select-repo')

	// Repository selection
	const [repositories, setRepositories] = useState<GitHubRepository[]>([])
	const [reposLoading, setReposLoading] = useState(false)
	const [selectedRepo, setSelectedRepo] = useState<GitHubRepository | null>(
		null,
	)

	// File selection
	const [currentPath, setCurrentPath] = useState('')
	const [files, setFiles] = useState<GitHubFileInfo[]>([])
	const [dirLoading, setDirLoading] = useState(false)
	const [selectedFiles, setSelectedFiles] = useState<string[]>([])

	// Configuration
	const [repoUsername, setRepoUsername] = useState('')
	const [contentPropertyName, setContentPropertyName] = useState('content')
	const [enableGithubSync, setEnableGithubSync] = useState(true)
	const [propertyMappings, setPropertyMappings] = useState<
		Array<{
			frontmatterKey: string
			propertyName: string
			propertyType: PropertyType
			selectOptions?: string[]
		}>
	>([])
	const [analysisLoading, setAnalysisLoading] = useState(false)
	const [analysisData, setAnalysisData] = useState<{
		properties: FrontmatterProperty[]
		totalFiles: number
		validFiles: number
	} | null>(null)

	// Load repositories when dialog opens
	useEffect(() => {
		if (open && repositories.length === 0) {
			setReposLoading(true)
			listGitHubRepositories()
				.then(result => {
					if (result.error) {
						toast.error(result.error)
					} else {
						setRepositories(result.repositories)
					}
				})
				.finally(() => setReposLoading(false))
		}
	}, [open, repositories.length])

	// Load directory contents when path or repo changes
	const loadDirectoryContents = useCallback(
		async (path: string) => {
			if (!selectedRepo) return

			setDirLoading(true)
			const result = await listDirectoryContents({
				githubRepo: selectedRepo.fullName,
				path,
			})
			setDirLoading(false)

			if (result.error) {
				toast.error(result.error)
			} else {
				setFiles(result.files)
			}
		},
		[selectedRepo],
	)

	useEffect(() => {
		if (selectedRepo && step === 'select-files') {
			loadDirectoryContents(currentPath)
		}
	}, [selectedRepo, currentPath, step, loadDirectoryContents])

	// Analyze frontmatter when entering configure step
	useEffect(() => {
		if (step === 'configure' && selectedRepo && selectedFiles.length > 0) {
			setAnalysisLoading(true)
			analyzeFrontmatter({
				githubRepo: selectedRepo.fullName,
				paths: selectedFiles,
			})
				.then(result => {
					if (result.error) {
						toast.error(result.error)
					} else {
						setAnalysisData(result)
						// Auto-populate property mappings
						const mappings = result.properties.map(prop => ({
							frontmatterKey: prop.key,
							propertyName: prop.key,
							propertyType: prop.suggestSelect
								? PropertyType.Select
								: prop.suggestedType,
							selectOptions: prop.suggestSelect ? prop.uniqueValues : undefined,
						}))
						setPropertyMappings(mappings)
					}
				})
				.finally(() => setAnalysisLoading(false))
		}
	}, [step, selectedRepo, selectedFiles])

	const handleSelectRepo = (repo: GitHubRepository) => {
		setSelectedRepo(repo)
		setRepoUsername(repo.name.toLowerCase().replace(/[^a-z0-9-]/g, '-'))
		setCurrentPath('')
		setSelectedFiles([])
		setStep('select-files')
	}

	const handleNavigateToFolder = (path: string) => {
		setCurrentPath(path)
	}

	const handleBack = () => {
		if (currentPath) {
			const parts = currentPath.split('/')
			parts.pop()
			setCurrentPath(parts.join('/'))
		}
	}

	const handleToggleFile = (path: string) => {
		setSelectedFiles(prev =>
			prev.includes(path) ? prev.filter(p => p !== path) : [...prev, path],
		)
	}

	const handleSelectAllMarkdown = () => {
		const markdownFiles = files
			.filter(f => f.fileType === 'file')
			.map(f => f.path)
		setSelectedFiles(markdownFiles)
	}

	const handleProceedToConfigure = () => {
		if (selectedFiles.length === 0) {
			toast.error(t.v1beta.githubImport.selectAtLeastOne)
			return
		}
		setStep('configure')
	}

	const handleImport = async () => {
		if (!selectedRepo) return

		setStep('importing')

		startTransition(async () => {
			const result = await importMarkdownFromGitHub({
				orgUsername: org,
				repoUsername: repoUsername,
				repoName: repoUsername,
				githubRepo: selectedRepo.fullName,
				paths: selectedFiles,
				propertyMappings: propertyMappings.map(m => ({
					frontmatterKey: m.frontmatterKey,
					propertyName: m.propertyName,
					propertyType: m.propertyType,
					selectOptions: m.selectOptions,
				})),
				contentPropertyName,
				enableGithubSync,
			})

			if (result.error) {
				toast.error(result.error)
				setStep('configure')
				return
			}

			const { importedCount, updatedCount, errors } = result

			if (errors.length > 0) {
				toast.warning(
					`Imported ${importedCount} files, updated ${updatedCount} files. ${errors.length} errors occurred.`,
				)
			} else {
				toast.success(
					`Successfully imported ${importedCount} files and updated ${updatedCount} files.`,
				)
			}

			onImportComplete?.()
			setOpen(false)
			resetState()
			router.refresh()
		})
	}

	const resetState = () => {
		setStep('select-repo')
		setSelectedRepo(null)
		setCurrentPath('')
		setSelectedFiles([])
		setRepoUsername('')
		setPropertyMappings([])
		setAnalysisData(null)
	}

	const handleOpenChange = (newOpen: boolean) => {
		setOpen(newOpen)
		if (!newOpen) {
			resetState()
		}
	}

	const getStepDescription = () => {
		switch (step) {
			case 'select-repo':
				return t.v1beta.githubImport.selectRepo
			case 'select-files':
				return t.v1beta.githubImport.selectFiles
			case 'configure':
				return t.v1beta.githubImport.configureMapping
			case 'importing':
				return t.v1beta.githubImport.importing
			default:
				return ''
		}
	}

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogTrigger asChild>
				<Button variant='outline'>
					<Github className='w-4 h-4 mr-2' />
					{t.v1beta.githubImport.importFromGitHub}
				</Button>
			</DialogTrigger>
			<DialogContent className='max-w-2xl max-h-[80vh] overflow-y-auto'>
				<DialogHeader>
					<DialogTitle>
						{t.v1beta.githubImport.importMarkdownFromGitHub}
					</DialogTitle>
					<DialogDescription>{getStepDescription()}</DialogDescription>
				</DialogHeader>

				{step === 'select-repo' && (
					<div className='space-y-4'>
						{reposLoading ? (
							<div className='space-y-2'>
								{[...Array(5)].map((_, i) => (
									<Skeleton
										key={`skeleton-repo-${i}`}
										className='h-12 w-full'
									/>
								))}
							</div>
						) : (
							<div className='space-y-2'>
								{repositories.map(repo => (
									<button
										key={repo.id}
										type='button'
										className='w-full p-3 text-left border rounded-lg hover:bg-accent transition-colors'
										onClick={() => handleSelectRepo(repo)}
									>
										<div className='font-medium'>{repo.fullName}</div>
										{repo.description && (
											<div className='text-sm text-muted-foreground truncate'>
												{repo.description}
											</div>
										)}
									</button>
								))}
								{repositories.length === 0 && (
									<div className='text-center py-8 text-muted-foreground'>
										{t.v1beta.githubImport.noRepositories}
									</div>
								)}
							</div>
						)}
					</div>
				)}

				{step === 'select-files' && (
					<div className='space-y-4'>
						{/* Breadcrumb */}
						<div className='flex items-center gap-1 text-sm'>
							<button
								type='button'
								className='hover:underline'
								onClick={() => setCurrentPath('')}
							>
								{selectedRepo?.name}
							</button>
							{currentPath
								.split('/')
								.filter(Boolean)
								.map((part, i, arr) => (
									<span key={part} className='flex items-center'>
										<ChevronRight className='w-4 h-4 mx-1' />
										<button
											type='button'
											className='hover:underline'
											onClick={() =>
												setCurrentPath(arr.slice(0, i + 1).join('/'))
											}
										>
											{part}
										</button>
									</span>
								))}
						</div>

						{/* Back button and select all */}
						<div className='flex justify-between items-center'>
							<Button
								variant='ghost'
								size='sm'
								disabled={!currentPath}
								onClick={handleBack}
							>
								← {t.v1beta.githubImport.back}
							</Button>
							<Button
								variant='outline'
								size='sm'
								onClick={handleSelectAllMarkdown}
							>
								{t.v1beta.githubImport.selectAllMarkdown}
							</Button>
						</div>

						{/* File list */}
						{dirLoading ? (
							<div className='space-y-2'>
								{[...Array(5)].map((_, i) => (
									<Skeleton
										key={`skeleton-file-${i}`}
										className='h-10 w-full'
									/>
								))}
							</div>
						) : (
							<div className='space-y-1 max-h-[300px] overflow-y-auto border rounded-lg p-2'>
								{files.map(file => (
									<div
										key={file.path}
										className='flex items-center gap-2 p-2 hover:bg-accent rounded'
									>
										{file.fileType === 'dir' ? (
											<div className='flex items-center gap-2 w-full'>
												<Checkbox
													id={`dir-${file.path}`}
													checked={selectedFiles.includes(file.path)}
													onCheckedChange={() => handleToggleFile(file.path)}
												/>
												<button
													type='button'
													className='flex items-center gap-2 flex-1 text-left'
													onClick={() => handleNavigateToFolder(file.path)}
												>
													<Folder className='w-4 h-4 text-blue-500' />
													<span>{file.name}</span>
													<ChevronRight className='w-4 h-4 ml-auto' />
												</button>
											</div>
										) : (
											<div className='flex items-center gap-2 w-full'>
												<Checkbox
													id={`file-${file.path}`}
													checked={selectedFiles.includes(file.path)}
													onCheckedChange={() => handleToggleFile(file.path)}
												/>
												<label
													htmlFor={`file-${file.path}`}
													className='flex items-center gap-2 flex-1 cursor-pointer'
												>
													<File className='w-4 h-4 text-gray-500' />
													<span>{file.name}</span>
													<span className='text-xs text-muted-foreground ml-auto'>
														{(file.size / 1024).toFixed(1)} KB
													</span>
												</label>
											</div>
										)}
									</div>
								))}
								{files.length === 0 && (
									<div className='text-center py-4 text-muted-foreground'>
										{t.v1beta.githubImport.noFiles}
									</div>
								)}
							</div>
						)}

						{/* Selected count */}
						<div className='text-sm text-muted-foreground'>
							{t.v1beta.githubImport.itemsSelected.replace(
								'{count}',
								String(selectedFiles.length),
							)}
							{selectedFiles.some(f =>
								files.find(file => file.path === f && file.fileType === 'dir'),
							) && (
								<span className='ml-1'>
									{t.v1beta.githubImport.directoriesNote}
								</span>
							)}
						</div>
					</div>
				)}

				{step === 'configure' && (
					<div className='space-y-6'>
						{/* Repository settings */}
						<div className='space-y-4'>
							<div>
								<Label>{t.v1beta.githubImport.repositoryUsername}</Label>
								<Input
									value={repoUsername}
									onChange={e => setRepoUsername(e.target.value)}
									placeholder='my-docs'
								/>
								<p className='text-xs text-muted-foreground mt-1'>
									{t.v1beta.githubImport.repositoryUsernameDescription}
								</p>
								{existingRepos.some(
									repo =>
										repo.username.toLowerCase() === repoUsername.toLowerCase(),
								) && (
									<div className='flex items-center gap-2 mt-2 p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded text-yellow-800 dark:text-yellow-200 text-sm'>
										<AlertTriangle className='w-4 h-4 shrink-0' />
										<span>
											{t.v1beta.githubImport.repoExistsWarning.replace(
												'{name}',
												repoUsername,
											)}
										</span>
									</div>
								)}
							</div>
							<div>
								<Label>{t.v1beta.githubImport.contentPropertyName}</Label>
								<Input
									value={contentPropertyName}
									onChange={e => setContentPropertyName(e.target.value)}
									placeholder='content'
								/>
								<p className='text-xs text-muted-foreground mt-1'>
									{t.v1beta.githubImport.contentPropertyNameDescription}
								</p>
							</div>
							<div className='flex items-center gap-3 pt-2'>
								<Checkbox
									id='sync-ext-github'
									checked={enableGithubSync}
									onCheckedChange={checked =>
										setEnableGithubSync(checked === true)
									}
								/>
								<div>
									<label
										htmlFor='sync-ext-github'
										className='text-sm font-medium cursor-pointer'
									>
										{t.v1beta.githubImport.syncExtGithub}
									</label>
									<p className='text-xs text-muted-foreground'>
										{t.v1beta.githubImport.syncExtGithubDescription}
									</p>
								</div>
							</div>
						</div>

						{/* Frontmatter analysis */}
						{analysisLoading ? (
							<div className='space-y-2'>
								<Skeleton className='h-8 w-48' />
								<Skeleton className='h-20 w-full' />
							</div>
						) : analysisData ? (
							<div className='space-y-4'>
								<div className='flex items-center justify-between'>
									<h3 className='font-medium'>
										{t.v1beta.githubImport.propertyMappings}
									</h3>
									<span className='text-sm text-muted-foreground'>
										{t.v1beta.githubImport.filesWithFrontmatter
											.replace('{valid}', String(analysisData.validFiles))
											.replace('{total}', String(analysisData.totalFiles))}
									</span>
								</div>

								{propertyMappings.length === 0 ? (
									<div className='text-center py-4 text-muted-foreground border rounded-lg'>
										<AlertCircle className='w-8 h-8 mx-auto mb-2 opacity-50' />
										{t.v1beta.githubImport.noFrontmatter}
									</div>
								) : (
									<div className='space-y-3 border rounded-lg p-4'>
										{propertyMappings.map((mapping, i) => (
											<div
												key={mapping.frontmatterKey}
												className='grid grid-cols-3 gap-2 items-center'
											>
												<div className='text-sm font-mono bg-muted px-2 py-1 rounded'>
													{mapping.frontmatterKey}
												</div>
												<Input
													value={mapping.propertyName}
													onChange={e => {
														const newMappings = [...propertyMappings]
														newMappings[i].propertyName = e.target.value
														setPropertyMappings(newMappings)
													}}
													placeholder={t.v1beta.githubImport.propertyName}
												/>
												<Select
													value={mapping.propertyType}
													onValueChange={(value: PropertyType) => {
														const newMappings = [...propertyMappings]
														newMappings[i].propertyType = value
														setPropertyMappings(newMappings)
													}}
												>
													<SelectTrigger>
														<SelectValue />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value={PropertyType.String}>
															String
														</SelectItem>
														<SelectItem value={PropertyType.Select}>
															Select
														</SelectItem>
														<SelectItem value={PropertyType.Markdown}>
															Markdown
														</SelectItem>
														<SelectItem value={PropertyType.Integer}>
															Integer
														</SelectItem>
													</SelectContent>
												</Select>
											</div>
										))}
									</div>
								)}
							</div>
						) : null}
					</div>
				)}

				{step === 'importing' && (
					<div className='flex flex-col items-center justify-center py-12'>
						<Loader2 className='w-12 h-12 animate-spin text-primary mb-4' />
						<p>
							{t.v1beta.githubImport.importingFiles.replace(
								'{count}',
								String(selectedFiles.length),
							)}
						</p>
						<p className='text-sm text-muted-foreground'>
							{t.v1beta.githubImport.mayTakeMoments}
						</p>
					</div>
				)}

				<DialogFooter>
					{step === 'select-files' && (
						<>
							<Button variant='outline' onClick={() => setStep('select-repo')}>
								{t.v1beta.githubImport.back}
							</Button>
							<Button
								onClick={handleProceedToConfigure}
								disabled={selectedFiles.length === 0}
							>
								{t.v1beta.githubImport.next.replace(
									'{count}',
									String(selectedFiles.length),
								)}
							</Button>
						</>
					)}
					{step === 'configure' && (
						<>
							<Button variant='outline' onClick={() => setStep('select-files')}>
								{t.v1beta.githubImport.back}
							</Button>
							<Button
								onClick={handleImport}
								disabled={
									isPending ||
									existingRepos.some(
										repo =>
											repo.username.toLowerCase() ===
											repoUsername.toLowerCase(),
									)
								}
							>
								<Upload className='w-4 h-4 mr-2' />
								{t.v1beta.githubImport.importFiles}
							</Button>
						</>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
