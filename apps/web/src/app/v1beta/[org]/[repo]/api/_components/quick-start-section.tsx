'use client'

import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Check, Copy } from 'lucide-react'
import { useCallback, useState } from 'react'
import { CodeSnippet } from './code-snippet'

interface QuickStartSectionProps {
	apiBaseUrl: string
	org: string
	repo: string
	apiKeySlot?: React.ReactNode
}

export function QuickStartSection({
	apiBaseUrl,
	org,
	repo,
	apiKeySlot,
}: QuickStartSectionProps) {
	const { t } = useTranslation()
	const [copied, setCopied] = useState(false)

	const handleCopyBaseUrl = useCallback(async () => {
		await navigator.clipboard.writeText(apiBaseUrl)
		setCopied(true)
		setTimeout(() => setCopied(false), 2000)
	}, [apiBaseUrl])

	const quickStartSnippets = {
		curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
		python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
		javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
	}

	return (
		<div className='space-y-4'>
			<h2 className='text-xl font-semibold'>
				{t.v1beta.developerPortal.quickStart.title}
			</h2>
			<div className='grid gap-4 md:grid-cols-3'>
				{/* Step 1: Create API Key */}
				<Card>
					<CardContent className='pt-6'>
						<div className='flex items-center gap-2 mb-3'>
							<span className='flex items-center justify-center w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs font-bold'>
								1
							</span>
							<h3 className='font-medium'>
								{t.v1beta.developerPortal.quickStart.step1Title}
							</h3>
						</div>
						<p className='text-sm text-muted-foreground mb-3'>
							{t.v1beta.developerPortal.quickStart.step1Description}
						</p>
						{apiKeySlot}
					</CardContent>
				</Card>

				{/* Step 2: Base URL */}
				<Card>
					<CardContent className='pt-6'>
						<div className='flex items-center gap-2 mb-3'>
							<span className='flex items-center justify-center w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs font-bold'>
								2
							</span>
							<h3 className='font-medium'>
								{t.v1beta.developerPortal.quickStart.step2Title}
							</h3>
						</div>
						<p className='text-sm text-muted-foreground mb-3'>
							{t.v1beta.developerPortal.quickStart.step2Description}
						</p>
						<div className='flex items-center gap-2'>
							<code className='flex-1 rounded bg-muted px-3 py-2 text-xs font-mono break-all'>
								{apiBaseUrl}
							</code>
							<Button
								variant='outline'
								size='sm'
								onClick={handleCopyBaseUrl}
								className='shrink-0'
							>
								{copied ? (
									<Check className='w-3 h-3' />
								) : (
									<Copy className='w-3 h-3' />
								)}
							</Button>
						</div>
					</CardContent>
				</Card>

				{/* Step 3: First Request */}
				<Card>
					<CardContent className='pt-6'>
						<div className='flex items-center gap-2 mb-3'>
							<span className='flex items-center justify-center w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs font-bold'>
								3
							</span>
							<h3 className='font-medium'>
								{t.v1beta.developerPortal.quickStart.step3Title}
							</h3>
						</div>
						<p className='text-sm text-muted-foreground'>
							{t.v1beta.developerPortal.quickStart.step3Description}
						</p>
					</CardContent>
				</Card>
			</div>

			{/* First request code snippet */}
			<CodeSnippet snippets={quickStartSnippets} />
		</div>
	)
}
