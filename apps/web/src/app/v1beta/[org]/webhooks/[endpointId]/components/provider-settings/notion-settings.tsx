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
import { BookOpen, Database, FileText } from 'lucide-react'
import { useState } from 'react'

interface NotionSettingsProps {
	config: {
		database_id?: string | null
	}
	events: string[]
	onUpdate: (config: Record<string, unknown>, events: string[]) => Promise<void>
}

const availableEvents = [
	{
		value: 'page.created',
		label: 'Page Created',
		description: 'A new page is created',
	},
	{
		value: 'page.updated',
		label: 'Page Updated',
		description: 'An existing page is updated',
	},
	{
		value: 'page.deleted',
		label: 'Page Deleted',
		description: 'A page is deleted',
	},
	{
		value: 'database.created',
		label: 'Database Created',
		description: 'A new database is created',
	},
	{
		value: 'database.updated',
		label: 'Database Updated',
		description: 'An existing database is updated',
	},
]

export function NotionSettings({
	config,
	events,
	onUpdate,
}: NotionSettingsProps) {
	const [isEditing, setIsEditing] = useState(false)
	const [databaseId, setDatabaseId] = useState(config.database_id || '')
	const [selectedEvents, setSelectedEvents] = useState<string[]>(events)

	const handleSave = async () => {
		await onUpdate(
			{
				notion: {
					database_id: databaseId || null,
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
								<BookOpen className='h-5 w-5' />
								Notion Configuration
							</CardTitle>
							<CardDescription>
								Configure which database to sync
							</CardDescription>
						</div>
						<Button variant='outline' onClick={() => setIsEditing(true)}>
							Edit
						</Button>
					</div>
				</CardHeader>
				<CardContent className='space-y-4'>
					<div>
						<Label className='text-muted-foreground'>Database ID</Label>
						<p className='font-mono text-sm'>
							{config.database_id || (
								<span className='text-muted-foreground'>
									All databases (workspace-wide)
								</span>
							)}
						</p>
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
					<BookOpen className='h-5 w-5' />
					Notion Configuration
				</CardTitle>
				<CardDescription>Configure which database to sync</CardDescription>
			</CardHeader>
			<CardContent className='space-y-4'>
				<div className='space-y-2'>
					<Label htmlFor='databaseId'>
						<Database className='h-4 w-4 inline mr-1' />
						Database ID (optional)
					</Label>
					<Input
						id='databaseId'
						value={databaseId}
						onChange={e => setDatabaseId(e.target.value)}
						placeholder='xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx'
					/>
					<p className='text-xs text-muted-foreground'>
						Leave empty to receive events from all databases in the workspace.
						You can find the database ID in the URL when viewing a database.
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
								<div className='flex items-center gap-2'>
									<FileText className='h-4 w-4 text-muted-foreground' />
									<div>
										<p className='font-medium'>{event.label}</p>
										<p className='text-sm text-muted-foreground'>
											{event.description}
										</p>
									</div>
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
