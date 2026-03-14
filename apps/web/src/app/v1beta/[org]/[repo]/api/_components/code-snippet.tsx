'use client'

import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Check, Copy } from 'lucide-react'
import { useCallback, useState } from 'react'

interface CodeSnippetProps {
	snippets: {
		curl: string
		python: string
		javascript: string
	}
}

export function CodeSnippet({ snippets }: CodeSnippetProps) {
	const { t } = useTranslation()
	const [copied, setCopied] = useState(false)
	const [activeTab, setActiveTab] = useState('curl')

	const handleCopy = useCallback(async () => {
		const text =
			activeTab === 'curl'
				? snippets.curl
				: activeTab === 'python'
					? snippets.python
					: snippets.javascript
		await navigator.clipboard.writeText(text)
		setCopied(true)
		setTimeout(() => setCopied(false), 2000)
	}, [activeTab, snippets])

	return (
		<Tabs value={activeTab} onValueChange={setActiveTab}>
			<div className='flex items-center justify-between gap-2'>
				<TabsList className='shrink-0'>
					<TabsTrigger value='curl'>curl</TabsTrigger>
					<TabsTrigger value='python'>Python</TabsTrigger>
					<TabsTrigger value='javascript'>JavaScript</TabsTrigger>
				</TabsList>
				<Button
					variant='ghost'
					size='sm'
					onClick={handleCopy}
					className='h-7 px-2 text-xs shrink-0'
				>
					{copied ? (
						<>
							<Check className='w-3 h-3 mr-1' />
							{t.v1beta.developerPortal.codeSnippet.copied}
						</>
					) : (
						<>
							<Copy className='w-3 h-3 mr-1' />
							{t.v1beta.developerPortal.codeSnippet.copy}
						</>
					)}
				</Button>
			</div>
			{(['curl', 'python', 'javascript'] as const).map(lang => (
				<TabsContent key={lang} value={lang} className='mt-2'>
					<pre className='rounded-lg bg-zinc-950 p-4 overflow-x-auto'>
						<code className='text-sm text-zinc-100 font-mono whitespace-pre'>
							{snippets[lang]}
						</code>
					</pre>
				</TabsContent>
			))}
		</Tabs>
	)
}
