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
import { FolderKanban, Layers, Users } from 'lucide-react'
import { useState } from 'react'

interface LinearSettingsProps {
	config: {
		team_id?: string | null
		project_id?: string | null
	}
	events: string[]
	onUpdate: (config: Record<string, unknown>, events: string[]) => Promise<void>
}

const availableEvents = [
	{
		value: 'Issue',
		label: 'Issues',
		description: 'Issue created, updated, or deleted',
	},
	{
		value: 'Project',
		label: 'Projects',
		description: 'Project created, updated, or deleted',
	},
	{
		value: 'Comment',
		label: 'Comments',
		description: 'Comment added or updated',
	},
	{ value: 'Cycle', label: 'Cycles', description: 'Cycle created or updated' },
]

export function LinearSettings({
	config,
	events,
	onUpdate,
}: LinearSettingsProps) {
	const [isEditing, setIsEditing] = useState(false)
	const [teamId, setTeamId] = useState(config.team_id || '')
	const [projectId, setProjectId] = useState(config.project_id || '')
	const [selectedEvents, setSelectedEvents] = useState<string[]>(events)

	const handleSave = async () => {
		await onUpdate(
			{
				linear: {
					team_id: teamId || null,
					project_id: projectId || null,
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
								<Layers className='h-5 w-5' />
								Linear Configuration
							</CardTitle>
							<CardDescription>
								Configure which team and project to sync
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
							<Label className='text-muted-foreground'>Team ID</Label>
							<p className='font-mono text-sm'>
								{config.team_id || (
									<span className='text-muted-foreground'>All teams</span>
								)}
							</p>
						</div>
						<div>
							<Label className='text-muted-foreground'>Project ID</Label>
							<p className='font-mono text-sm'>
								{config.project_id || (
									<span className='text-muted-foreground'>All projects</span>
								)}
							</p>
						</div>
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
					<Layers className='h-5 w-5' />
					Linear Configuration
				</CardTitle>
				<CardDescription>
					Configure which team and project to sync
				</CardDescription>
			</CardHeader>
			<CardContent className='space-y-4'>
				<div className='space-y-2'>
					<Label htmlFor='teamId'>
						<Users className='h-4 w-4 inline mr-1' />
						Team ID (optional)
					</Label>
					<Input
						id='teamId'
						value={teamId}
						onChange={e => setTeamId(e.target.value)}
						placeholder='team-uuid'
					/>
					<p className='text-xs text-muted-foreground'>
						Leave empty to receive events from all teams
					</p>
				</div>

				<div className='space-y-2'>
					<Label htmlFor='projectId'>
						<FolderKanban className='h-4 w-4 inline mr-1' />
						Project ID (optional)
					</Label>
					<Input
						id='projectId'
						value={projectId}
						onChange={e => setProjectId(e.target.value)}
						placeholder='project-uuid'
					/>
					<p className='text-xs text-muted-foreground'>
						Leave empty to receive events from all projects
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
