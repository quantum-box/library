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
import { FileCode, FolderGit, GitBranch } from 'lucide-react'
import { useState } from 'react'

interface GitHubSettingsProps {
	config: {
		repository?: string
		branch?: string
		path_pattern?: string | null
	}
	events: string[]
	onUpdate: (config: Record<string, unknown>, events: string[]) => Promise<void>
}

const availableEvents = [
	{ value: 'push', label: 'Push', description: 'Branch pushed to repository' },
	{
		value: 'pull_request',
		label: 'Pull Request',
		description: 'PR opened, closed, or merged',
	},
	{
		value: 'release',
		label: 'Release',
		description: 'Release created or published',
	},
	{ value: 'issues', label: 'Issues', description: 'Issue opened or updated' },
]

export function GitHubSettings({
	config,
	events,
	onUpdate,
}: GitHubSettingsProps) {
	const [isEditing, setIsEditing] = useState(false)
	const [repository, setRepository] = useState(config.repository || '')
	const [branch, setBranch] = useState(config.branch || 'main')
	const [pathPattern, setPathPattern] = useState(config.path_pattern || '')
	const [selectedEvents, setSelectedEvents] = useState<string[]>(events)

	const handleSave = async () => {
		await onUpdate(
			{
				github: {
					repository,
					branch,
					path_pattern: pathPattern || null,
				},
			},
			selectedEvents,
		)
		setIsEditing(false)
	}

	const toggleEvent = (event: string) => {
		setSelectedEvents(prev =>
			prev.includes(event) ? prev.filter(e => e !== event) : [...prev, event],
		)
	}

	if (!isEditing) {
		return (
			<Card>
				<CardHeader>
					<div className='flex justify-between items-start'>
						<div>
							<CardTitle className='flex items-center gap-2'>
								<GitBranch className='h-5 w-5' />
								GitHub Configuration
							</CardTitle>
							<CardDescription>
								Configure which repository, branch, and files to sync
							</CardDescription>
						</div>
						<Button variant='outline' onClick={() => setIsEditing(true)}>
							Edit
						</Button>
					</div>
				</CardHeader>
				<CardContent className='space-y-4'>
					<div className='grid gap-4 sm:grid-cols-2'>
						<div>
							<Label className='text-muted-foreground'>Repository</Label>
							<p className='font-mono text-sm'>{config.repository}</p>
						</div>
						<div>
							<Label className='text-muted-foreground'>Branch</Label>
							<p className='font-mono text-sm'>{config.branch}</p>
						</div>
						{config.path_pattern && (
							<div className='sm:col-span-2'>
								<Label className='text-muted-foreground'>Path Pattern</Label>
								<p className='font-mono text-sm'>{config.path_pattern}</p>
							</div>
						)}
					</div>
					<div>
						<Label className='text-muted-foreground'>Events</Label>
						<div className='flex flex-wrap gap-2 mt-1'>
							{events.length === 0 ? (
								<Badge variant='secondary'>All events</Badge>
							) : (
								events.map(event => (
									<Badge key={event} variant='outline'>
										{event}
									</Badge>
								))
							)}
						</div>
					</div>
				</CardContent>
			</Card>
		)
	}

	return (
		<Card>
			<CardHeader>
				<CardTitle className='flex items-center gap-2'>
					<GitBranch className='h-5 w-5' />
					GitHub Configuration
				</CardTitle>
				<CardDescription>
					Configure which repository, branch, and files to sync
				</CardDescription>
			</CardHeader>
			<CardContent className='space-y-4'>
				<div className='space-y-2'>
					<Label htmlFor='repository'>
						<FolderGit className='h-4 w-4 inline mr-1' />
						Repository (owner/repo)
					</Label>
					<Input
						id='repository'
						value={repository}
						onChange={e => setRepository(e.target.value)}
						placeholder='owner/repo'
					/>
				</div>

				<div className='space-y-2'>
					<Label htmlFor='branch'>
						<GitBranch className='h-4 w-4 inline mr-1' />
						Branch
					</Label>
					<Input
						id='branch'
						value={branch}
						onChange={e => setBranch(e.target.value)}
						placeholder='main'
					/>
				</div>

				<div className='space-y-2'>
					<Label htmlFor='pathPattern'>
						<FileCode className='h-4 w-4 inline mr-1' />
						Path Pattern (optional)
					</Label>
					<Input
						id='pathPattern'
						value={pathPattern}
						onChange={e => setPathPattern(e.target.value)}
						placeholder='docs/**/*.md'
					/>
					<p className='text-xs text-muted-foreground'>
						Glob pattern to filter files. Leave empty to sync all files.
					</p>
				</div>

				<div className='space-y-2'>
					<Label>Events</Label>
					<div className='grid gap-2'>
						{availableEvents.map(event => (
							<label
								key={event.value}
								className={`flex items-center justify-between p-3 rounded-md border cursor-pointer transition-colors ${
									selectedEvents.includes(event.value)
										? 'border-primary bg-primary/5'
										: 'hover:bg-muted'
								}`}
							>
								<div>
									<p className='font-medium'>{event.label}</p>
									<p className='text-sm text-muted-foreground'>
										{event.description}
									</p>
								</div>
								<input
									type='checkbox'
									checked={selectedEvents.includes(event.value)}
									onChange={() => toggleEvent(event.value)}
									className='h-4 w-4'
								/>
							</label>
						))}
					</div>
					<p className='text-xs text-muted-foreground'>
						Leave all unchecked to receive all events
					</p>
				</div>

				<div className='flex gap-2 justify-end'>
					<Button variant='outline' onClick={() => setIsEditing(false)}>
						Cancel
					</Button>
					<Button onClick={handleSave}>Save Changes</Button>
				</div>
			</CardContent>
		</Card>
	)
}
