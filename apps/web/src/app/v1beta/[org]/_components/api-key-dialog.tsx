'use client'

import { Button } from '@/components/ui/button'
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
import { useToast } from '@/components/ui/use-toast'
import { CreateApiKeyMutation } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Copy, Plus } from 'lucide-react'
import { useState } from 'react'

export function ApiKeyDialog({
	orgUsername,
	onCreate,
}: {
	orgUsername: string
	onCreate: (orgUsername: string, name: string) => Promise<CreateApiKeyMutation>
}) {
	const { t } = useTranslation()
	const [open, setOpen] = useState(false)
	const [name, setName] = useState('')
	const [loading, setLoading] = useState(false)
	const [apiKey, setApiKey] = useState<string | null>(null)
	const { toast } = useToast()

	const handleCreateApiKey = async () => {
		if (!name) return

		setLoading(true)
		try {
			const result = await onCreate(orgUsername, name)

			setApiKey(result.createApiKey.apiKey.value)
			toast({
				title: t.v1beta.apiKeyDialog.successCreated,
				description: t.v1beta.apiKeyDialog.apiKeyCreatedDescription,
			})
		} catch (error) {
			console.error('API key creation error:', error)
			toast({
				title: t.v1beta.apiKeyDialog.error,
				description: t.v1beta.apiKeyDialog.failedCreate,
				variant: 'destructive',
			})
		} finally {
			setLoading(false)
		}
	}

	const handleCopyApiKey = () => {
		if (apiKey) {
			navigator.clipboard.writeText(apiKey)
			toast({
				title: t.v1beta.apiKeyDialog.copiedToClipboard,
				description: t.v1beta.apiKeyDialog.apiKeyCopied,
			})
		}
	}

	const handleClose = () => {
		setOpen(false)
		setName('')
		setApiKey(null)
	}

	return (
		<Dialog open={open} onOpenChange={setOpen}>
			<DialogTrigger asChild>
				<Button variant='outline' size='sm'>
					<Plus className='w-4 h-4 mr-2' />
					{t.v1beta.apiKeyDialog.createApiKey}
				</Button>
			</DialogTrigger>
			<DialogContent className='sm:max-w-md'>
				<DialogHeader>
					<DialogTitle>{t.v1beta.apiKeyDialog.createApiKey}</DialogTitle>
					<DialogDescription>
						{apiKey
							? t.v1beta.apiKeyDialog.apiKeyCreatedDescription
							: t.v1beta.apiKeyDialog.createApiKeyDescription}
					</DialogDescription>
				</DialogHeader>
				{!apiKey ? (
					<div className='grid gap-4 py-4'>
						<div className='grid grid-cols-4 items-center gap-4'>
							<Label htmlFor='name' className='text-right'>
								{t.v1beta.apiKeyDialog.name}
							</Label>
							<Input
								id='name'
								value={name}
								onChange={e => setName(e.target.value)}
								className='col-span-3'
								placeholder={t.v1beta.apiKeyDialog.namePlaceholder}
							/>
						</div>
					</div>
				) : (
					<div className='grid gap-4 py-4'>
						<div className='grid grid-cols-4 items-center gap-4'>
							<Label htmlFor='apiKey' className='text-right'>
								{t.v1beta.apiKeyDialog.apiKey}
							</Label>
							<div className='col-span-3 flex items-center gap-2'>
								<Input
									id='apiKey'
									value={apiKey}
									readOnly
									className='font-mono text-xs'
								/>
								<Button
									type='button'
									variant='outline'
									size='icon'
									onClick={handleCopyApiKey}
								>
									<Copy className='h-4 w-4' />
								</Button>
							</div>
						</div>
					</div>
				)}
				<DialogFooter>
					{!apiKey ? (
						<Button
							type='button'
							onClick={handleCreateApiKey}
							disabled={!name || loading}
						>
							{loading
								? t.v1beta.apiKeyDialog.creating
								: t.v1beta.apiKeyDialog.create}
						</Button>
					) : (
						<Button type='button' onClick={handleClose}>
							{t.v1beta.apiKeyDialog.close}
						</Button>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
