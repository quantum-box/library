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
import { AlertTriangle, FolderGit2, Github, Trash2 } from 'lucide-react'
import Link from 'next/link'
import { useParams } from 'next/navigation'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

/**
 * プロパティ画面で設定されたリポジトリの型
 */
export interface AvailableRepo {
	repo: string
	label?: string
	/** Default file path for this repository */
	defaultPath?: string
}

/**
 * ext_github プロパティの値を表す型
 */
export interface ExtGithubValue {
	repo: string
	path: string
	enabled?: boolean
	/** パス変更時に古いファイルを削除するかどうか */
	deleteOldPath?: boolean
	/** 変更前のパス（古いファイル削除用） */
	oldPath?: string
}

interface ExtGithubEditorProps {
	/** 現在の値 (JSON文字列またはパース済みオブジェクト) */
	value?: string | ExtGithubValue | null
	/** 編集モードかどうか */
	isEditing: boolean
	/** 値が変更されたときのコールバック */
	onChange: (jsonValue: string) => void
	/** プロパティ画面で設定されたリポジトリリスト */
	availableRepos?: AvailableRepo[]
}

/**
 * ext_github プロパティ専用エディタ
 *
 * - 編集時: repo選択（プロパティで設定したリポジトリからSelect）、パス入力、有効/無効切り替え
 * - 表示時: リポジトリ名とパスをアイコン付きで表示
 */
export function ExtGithubEditor({
	value,
	isEditing,
	onChange,
	availableRepos = [],
}: ExtGithubEditorProps) {
	const params = useParams()
	const org = params.org as string
	const parsedValue = useMemo<ExtGithubValue | null>(() => {
		if (!value) return null
		if (typeof value === 'object') return value
		try {
			return JSON.parse(value) as ExtGithubValue
		} catch {
			return null
		}
	}, [value])

	const [repo, setRepo] = useState(parsedValue?.repo ?? '')
	const [path, setPath] = useState(parsedValue?.path ?? '')
	const [enabled, setEnabled] = useState(parsedValue?.enabled ?? true)
	const [deleteOldPath, setDeleteOldPath] = useState(true)

	// 元のパスを保存（編集開始時点のパス）
	const originalPathRef = useRef<string | null>(null)
	const originalRepoRef = useRef<string | null>(null)

	// パス変更確認ダイアログ
	const [showPathChangeDialog, setShowPathChangeDialog] = useState(false)
	const [pendingPath, setPendingPath] = useState('')

	// 外部から値が変更された場合に状態を同期
	useEffect(() => {
		if (parsedValue) {
			setRepo(parsedValue.repo ?? '')
			setPath(parsedValue.path ?? '')
			setEnabled(parsedValue.enabled ?? true)
		}
	}, [parsedValue])

	// 編集モード開始時に元のパスを保存、終了時にリセット
	useEffect(() => {
		if (isEditing && parsedValue?.path) {
			// 編集開始時に現在のパスを保存
			originalPathRef.current = parsedValue.path
			originalRepoRef.current = parsedValue.repo
		} else if (!isEditing) {
			// 編集終了時にリセット
			originalPathRef.current = null
			originalRepoRef.current = null
		}
	}, [isEditing, parsedValue?.path, parsedValue?.repo])

	const emitChange = useCallback(
		(
			newRepo: string,
			newPath: string,
			newEnabled: boolean,
			shouldDeleteOld: boolean,
			oldPathValue?: string,
		) => {
			if (!newRepo || !newPath) {
				onChange('')
				return
			}

			const hasPathChanged =
				originalPathRef.current &&
				originalPathRef.current !== newPath &&
				originalRepoRef.current === newRepo

			const json = JSON.stringify({
				repo: newRepo,
				path: newPath,
				enabled: newEnabled,
				// パスが変更された場合のみ削除オプションを含める
				...(hasPathChanged && {
					deleteOldPath: shouldDeleteOld,
					oldPath: oldPathValue ?? originalPathRef.current ?? undefined,
				}),
			} satisfies ExtGithubValue)
			onChange(json)
		},
		[onChange],
	)

	const handleRepoChange = (newRepo: string) => {
		setRepo(newRepo)
		// If path is empty and the selected repo has a default path, use it
		const selectedRepoConfig = availableRepos.find(r => r.repo === newRepo)
		let newPath = path
		if (!path && selectedRepoConfig?.defaultPath) {
			newPath = selectedRepoConfig.defaultPath
			setPath(newPath)
		}
		emitChange(newRepo, newPath, enabled, deleteOldPath)
	}

	const handlePathInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
		const newPath = e.target.value
		setPath(newPath)

		// 元のパスがあり、パスが異なる場合は確認ダイアログを表示
		if (
			originalPathRef.current &&
			newPath !== originalPathRef.current &&
			newPath.trim() !== '' &&
			originalRepoRef.current === repo
		) {
			setPendingPath(newPath)
			setShowPathChangeDialog(true)
		} else {
			emitChange(repo, newPath, enabled, deleteOldPath)
		}
	}

	const handlePathChangeConfirm = (shouldDelete: boolean) => {
		setDeleteOldPath(shouldDelete)
		emitChange(
			repo,
			pendingPath,
			enabled,
			shouldDelete,
			originalPathRef.current || undefined,
		)
		setShowPathChangeDialog(false)
	}

	const handlePathChangeCancel = () => {
		// パスを元に戻す
		setPath(originalPathRef.current ?? '')
		setShowPathChangeDialog(false)
	}

	const handleEnabledChange = (checked: boolean) => {
		setEnabled(checked)
		emitChange(repo, path, checked, deleteOldPath)
	}

	// 表示モード
	if (!isEditing) {
		if (!parsedValue || !parsedValue.repo || !parsedValue.path) {
			return (
				<span className='text-xs text-muted-foreground'>Not configured</span>
			)
		}

		return (
			<div className='space-y-2'>
				<div className='flex items-center gap-2 text-sm'>
					<Github className='h-4 w-4 text-muted-foreground' />
					<a
						href={`https://github.com/${parsedValue.repo}`}
						target='_blank'
						rel='noopener noreferrer'
						className='font-medium text-blue-600 hover:underline dark:text-blue-400'
					>
						{parsedValue.repo}
					</a>
				</div>
				<div className='flex items-center gap-2 text-sm'>
					<FolderGit2 className='h-4 w-4 text-muted-foreground' />
					<a
						href={`https://github.com/${parsedValue.repo}/blob/main/${parsedValue.path}`}
						target='_blank'
						rel='noopener noreferrer'
						className='font-mono text-xs text-blue-600 hover:underline dark:text-blue-400'
					>
						{parsedValue.path}
					</a>
				</div>
				{parsedValue.enabled === false && (
					<span className='rounded bg-yellow-100 px-2 py-0.5 text-xs text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300'>
						Sync disabled
					</span>
				)}
			</div>
		)
	}

	// 編集モード
	return (
		<>
			<div className='space-y-4'>
				{/* Repository selector */}
				<div className='space-y-2'>
					<Label className='text-xs font-medium text-muted-foreground'>
						Repository
					</Label>
					{availableRepos.length > 0 ? (
						<Select value={repo} onValueChange={handleRepoChange}>
							<SelectTrigger className='h-10 w-full rounded-xl border-border/60 bg-background/80'>
								<SelectValue placeholder='Select repository...'>
									{repo && (
										<span className='flex items-center gap-2'>
											<Github className='h-4 w-4' />
											{availableRepos.find(r => r.repo === repo)?.label || repo}
										</span>
									)}
								</SelectValue>
							</SelectTrigger>
							<SelectContent>
								{availableRepos.map(r => (
									<SelectItem key={r.repo} value={r.repo}>
										<span className='flex items-center gap-2'>
											<Github className='h-4 w-4' />
											{r.label || r.repo}
										</span>
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					) : (
						<div className='rounded-lg border border-dashed border-amber-300 bg-amber-50 p-3 dark:border-amber-700 dark:bg-amber-950/30'>
							<p className='text-sm text-amber-800 dark:text-amber-200'>
								No repositories configured.
							</p>
							<p className='mt-1 text-xs text-amber-600 dark:text-amber-400'>
								Go to{' '}
								<Link
									href={`/v1beta/${org}/properties`}
									className='font-medium text-blue-600 hover:underline dark:text-blue-400'
								>
									Properties
								</Link>{' '}
								and configure the ext_github property to add repositories.
							</p>
						</div>
					)}
				</div>

				{/* Path input */}
				<div className='space-y-2'>
					<Label className='text-xs font-medium text-muted-foreground'>
						File path
					</Label>
					<Input
						value={path}
						onChange={handlePathInputChange}
						placeholder='docs/example.md'
						className='h-10 w-full rounded-xl border-border/60 bg-background/80 font-mono text-sm'
					/>
					{originalPathRef.current && path !== originalPathRef.current && (
						<p className='flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400'>
							<AlertTriangle className='h-3 w-3' />
							Path changed from: {originalPathRef.current}
						</p>
					)}
				</div>

				{/* Enabled toggle */}
				<div className='flex items-center justify-between rounded-lg border border-border/60 p-3'>
					<div>
						<Label className='text-sm font-medium'>Auto-sync enabled</Label>
						<p className='text-xs text-muted-foreground'>
							Automatically sync changes to GitHub on save
						</p>
					</div>
					<Switch checked={enabled} onCheckedChange={handleEnabledChange} />
				</div>
			</div>

			{/* パス変更確認ダイアログ */}
			<AlertDialog
				open={showPathChangeDialog}
				onOpenChange={setShowPathChangeDialog}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle className='flex items-center gap-2'>
							<AlertTriangle className='h-5 w-5 text-amber-500' />
							Sync path is changing
						</AlertDialogTitle>
						<AlertDialogDescription className='space-y-3'>
							<p>
								You are changing the GitHub sync path. The old file at the
								previous location will remain on GitHub unless you delete it.
							</p>
							<div className='rounded-lg border bg-muted/50 p-3 space-y-2 text-sm'>
								<div className='flex items-center gap-2'>
									<span className='text-muted-foreground'>Old path:</span>
									<code className='rounded bg-muted px-1.5 py-0.5 font-mono text-xs'>
										{originalPathRef.current}
									</code>
								</div>
								<div className='flex items-center gap-2'>
									<span className='text-muted-foreground'>New path:</span>
									<code className='rounded bg-muted px-1.5 py-0.5 font-mono text-xs'>
										{pendingPath}
									</code>
								</div>
							</div>
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter className='flex-col gap-2 sm:flex-row'>
						<AlertDialogCancel onClick={handlePathChangeCancel}>
							Cancel
						</AlertDialogCancel>
						<AlertDialogAction
							onClick={() => handlePathChangeConfirm(false)}
							className='bg-secondary text-secondary-foreground hover:bg-secondary/80'
						>
							Keep old file
						</AlertDialogAction>
						<AlertDialogAction
							onClick={() => handlePathChangeConfirm(true)}
							className='bg-destructive text-destructive-foreground hover:bg-destructive/90'
						>
							<Trash2 className='mr-2 h-4 w-4' />
							Delete old file
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</>
	)
}
