'use client'

import { GitHubSettings } from './github-settings'
import { LinearSettings } from './linear-settings'
import { NotionSettings } from './notion-settings'

interface ProviderSettingsProps {
	provider: string
	config: string
	events: string[]
	onUpdate: (config: Record<string, unknown>, events: string[]) => Promise<void>
}

export function ProviderSettings({
	provider,
	config,
	events,
	onUpdate,
}: ProviderSettingsProps) {
	const parsedConfig = (() => {
		try {
			return JSON.parse(config)
		} catch {
			return {}
		}
	})()

	switch (provider) {
		case 'GITHUB':
			return (
				<GitHubSettings
					config={parsedConfig.github || parsedConfig}
					events={events}
					onUpdate={onUpdate}
				/>
			)
		case 'LINEAR':
			return (
				<LinearSettings
					config={parsedConfig.linear || parsedConfig}
					events={events}
					onUpdate={onUpdate}
				/>
			)
		case 'NOTION':
			return (
				<NotionSettings
					config={parsedConfig.notion || parsedConfig}
					events={events}
					onUpdate={onUpdate}
				/>
			)
		default:
			return (
				<div className='p-4 bg-muted rounded-md'>
					<p className='text-sm text-muted-foreground'>
						Provider-specific settings are not available for {provider}.
					</p>
					<pre className='mt-2 text-xs overflow-auto'>
						{JSON.stringify(parsedConfig, null, 2)}
					</pre>
				</div>
			)
	}
}
