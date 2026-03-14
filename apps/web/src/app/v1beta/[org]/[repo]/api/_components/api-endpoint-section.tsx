'use client'

import { useTranslation } from '@/lib/i18n/useTranslation'
import { cn } from '@/lib/utils'
import { ChevronDown } from 'lucide-react'
import { useState } from 'react'
import { CodeSnippet } from './code-snippet'
import { type EndpointCategory, HTTP_METHOD_COLORS } from './endpoints-data'

interface ApiEndpointSectionProps {
	categories: EndpointCategory[]
	apiBaseUrl: string
	org: string
	repo: string
}

export function ApiEndpointSection({
	categories,
	apiBaseUrl,
	org,
	repo,
}: ApiEndpointSectionProps) {
	const { t } = useTranslation()
	const [expandedEndpoint, setExpandedEndpoint] = useState<string | null>(null)

	const getCategoryLabel = (key: string) => {
		const labels = t.v1beta.developerPortal.endpoints as Record<string, string>
		return labels[key] || key
	}

	const toggleEndpoint = (id: string) => {
		setExpandedEndpoint(prev => (prev === id ? null : id))
	}

	return (
		<div className='space-y-8'>
			<h2 className='text-xl font-semibold'>
				{t.v1beta.developerPortal.endpoints.title}
			</h2>
			{categories.map(category => (
				<div key={category.key} className='space-y-3'>
					<h3 className='text-lg font-medium text-muted-foreground'>
						{getCategoryLabel(category.key)}
					</h3>
					<div className='border rounded-lg divide-y'>
						{category.endpoints.map((endpoint, idx) => {
							const id = `${category.key}-${idx}`
							const isExpanded = expandedEndpoint === id
							return (
								<div key={id}>
									<button
										type='button'
										onClick={() => toggleEndpoint(id)}
										className='flex items-center w-full p-3 text-left hover:bg-muted/50 transition-colors min-w-0'
									>
										<span
											className={cn(
												'inline-flex items-center justify-center rounded px-2 py-0.5 text-xs font-bold min-w-[60px] shrink-0',
												HTTP_METHOD_COLORS[endpoint.method],
											)}
										>
											{endpoint.method}
										</span>
										<span className='ml-3 font-mono text-sm flex-1 min-w-0 truncate'>
											{endpoint.path}
										</span>
										<span className='ml-3 text-sm text-muted-foreground hidden sm:inline shrink-0'>
											{endpoint.description}
										</span>
										<ChevronDown
											className={cn(
												'w-4 h-4 ml-2 transition-transform text-muted-foreground shrink-0',
												isExpanded && 'rotate-180',
											)}
										/>
									</button>
									{isExpanded && (
										<div className='p-4 border-t bg-muted/20'>
											<p className='text-sm text-muted-foreground mb-3 sm:hidden'>
												{endpoint.description}
											</p>
											<CodeSnippet
												snippets={endpoint.getSnippets({
													apiBaseUrl,
													org,
													repo,
												})}
											/>
										</div>
									)}
								</div>
							)
						})}
					</div>
				</div>
			))}
		</div>
	)
}
