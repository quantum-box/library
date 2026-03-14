'use client'

import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { BookOpen, ExternalLink, FileJson, Lock } from 'lucide-react'
import { ApiEndpointSection } from './api-endpoint-section'
import { getEndpointCategories } from './endpoints-data'
import { QuickStartSection } from './quick-start-section'

interface ApiPageUiProps {
	org: string
	repo: string
	apiBaseUrl: string
	apiKeySlot?: React.ReactNode
	apiKeyListSlot?: React.ReactNode
}

export function ApiPageUi({
	org,
	repo,
	apiBaseUrl,
	apiKeySlot,
	apiKeyListSlot,
}: ApiPageUiProps) {
	const { t } = useTranslation()
	const categories = getEndpointCategories()

	return (
		<div className='container py-6 space-y-10 overflow-hidden'>
			{/* Header */}
			<div className='flex items-center gap-3 flex-wrap'>
				<h1 className='text-2xl font-bold'>{t.v1beta.developerPortal.title}</h1>
				<Badge variant='secondary'>{t.v1beta.developerPortal.subtitle}</Badge>
			</div>

			{/* Quick Start */}
			<QuickStartSection
				apiBaseUrl={apiBaseUrl}
				org={org}
				repo={repo}
				apiKeySlot={apiKeySlot}
			/>

			{/* Authentication */}
			<Card>
				<CardHeader>
					<CardTitle className='flex items-center gap-2'>
						<Lock className='w-5 h-5' />
						{t.v1beta.developerPortal.authentication.title}
					</CardTitle>
				</CardHeader>
				<CardContent className='space-y-3'>
					<p className='text-sm text-muted-foreground'>
						{t.v1beta.developerPortal.authentication.description}
					</p>
					<pre className='rounded-lg bg-zinc-950 p-4 overflow-x-auto'>
						<code className='text-sm text-zinc-100 font-mono'>
							{t.v1beta.developerPortal.authentication.headerFormat}
						</code>
					</pre>
					<p className='text-xs text-muted-foreground'>
						{t.v1beta.developerPortal.authentication.note}
					</p>
				</CardContent>
			</Card>

			{/* API Key Management */}
			{apiKeyListSlot && (
				<div className='space-y-4'>
					<h2 className='text-xl font-semibold'>
						{t.v1beta.developerPortal.apiKeyManagement.title}
					</h2>
					{apiKeyListSlot}
				</div>
			)}

			{/* Endpoint Reference */}
			<ApiEndpointSection
				categories={categories}
				apiBaseUrl={apiBaseUrl}
				org={org}
				repo={repo}
			/>

			{/* Documentation Links */}
			<Card>
				<CardHeader>
					<CardTitle className='flex items-center gap-2'>
						<BookOpen className='w-5 h-5' />
						{t.v1beta.developerPortal.documentation.title}
					</CardTitle>
				</CardHeader>
				<CardContent className='space-y-3'>
					<p className='text-sm text-muted-foreground'>
						{t.v1beta.developerPortal.documentation.description}
					</p>
					<div className='flex flex-wrap gap-3'>
						<a
							href={`${apiBaseUrl}/v1beta/swagger-ui`}
							target='_blank'
							rel='noopener noreferrer'
							className='inline-flex items-center gap-1.5 rounded-md border px-3 py-2 text-sm font-medium hover:bg-muted transition-colors'
						>
							<BookOpen className='w-4 h-4' />
							{t.v1beta.developerPortal.documentation.swaggerUi}
							<ExternalLink className='w-3 h-3 text-muted-foreground' />
						</a>
						<a
							href={`${apiBaseUrl}/v1beta/redoc`}
							target='_blank'
							rel='noopener noreferrer'
							className='inline-flex items-center gap-1.5 rounded-md border px-3 py-2 text-sm font-medium hover:bg-muted transition-colors'
						>
							<BookOpen className='w-4 h-4' />
							{t.v1beta.developerPortal.documentation.redoc}
							<ExternalLink className='w-3 h-3 text-muted-foreground' />
						</a>
						<a
							href={`${apiBaseUrl}/v1beta/api-docs/openapi.json`}
							target='_blank'
							rel='noopener noreferrer'
							className='inline-flex items-center gap-1.5 rounded-md border px-3 py-2 text-sm font-medium hover:bg-muted transition-colors'
						>
							<FileJson className='w-4 h-4' />
							{t.v1beta.developerPortal.documentation.openApiSpec}
							<ExternalLink className='w-3 h-3 text-muted-foreground' />
						</a>
					</div>
				</CardContent>
			</Card>
		</div>
	)
}
