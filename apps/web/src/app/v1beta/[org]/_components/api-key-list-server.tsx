import { auth } from '@/app/(auth)/auth'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { getSdkPlatform } from '@/lib/apiClient'
import { Key } from 'lucide-react'
import { createApiKeyAction } from './action'
import { ApiKeyDialog } from './api-key-dialog'

export async function ApiKeyListServer({
	orgUsername,
}: {
	orgUsername: string
}) {
	// サーバーサイドでAPIキーを取得
	const session = await auth()
	if (!session) {
		return null
	}

	const locale = detectLocale()
	const dictionary = getDictionary(locale)
	const t = dictionary

	const sdk = await getSdkPlatform(session.user.accessToken)

	// APIキーを取得
	const result = await sdk.getApiKeys({
		orgUsername: orgUsername,
	})

	return (
		<Card>
			<CardHeader className='flex flex-row items-center justify-between'>
				<CardTitle className='text-xl flex items-center'>
					<Key className='w-5 h-5 mr-2' />
					{t.v1beta.apiKeyList.apiKey}
				</CardTitle>
				<ApiKeyDialog orgUsername={orgUsername} onCreate={createApiKeyAction} />
			</CardHeader>
			<CardContent>
				{result.apiKeys.length === 0 ? (
					<div className='py-6 text-center text-muted-foreground'>
						{t.v1beta.apiKeyList.noApiKeys}
					</div>
				) : (
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>{t.v1beta.apiKeyDialog.name}</TableHead>
								<TableHead>{t.v1beta.apiKeyList.id}</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{result.apiKeys.map(key => (
								<TableRow key={key.id}>
									<TableCell className='font-medium'>{key.name}</TableCell>
									<TableCell className='font-mono text-xs'>{key.id}</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				)}
			</CardContent>
		</Card>
	)
}
