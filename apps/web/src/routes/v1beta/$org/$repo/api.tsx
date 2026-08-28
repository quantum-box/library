import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { useCallback, useEffect, useState } from 'react'
import { ApiPageUi } from '@/app/v1beta/[org]/[repo]/api/_components/api-page-ui'
import { ApiKeyDialog } from '@/app/v1beta/[org]/_components/api-key-dialog'
import { ApiKeyList } from '@/app/v1beta/[org]/_components/api-key-list'
import {
  createApiKey,
  fetchApiKeys,
  revokeApiKey,
} from '@/app/v1beta/[org]/_components/api-key-actions'
import { useToast } from '@/components/ui/use-toast'
import type { ApiKeyItemFragment } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { baseURL, docsURL } from '@/lib/apiClient'

export const Route = createFileRoute('/v1beta/$org/$repo/api')({
  component: ApiPage,
})

function ApiPage() {
  const { org, repo } = Route.useParams()
  const { session } = useAuth()
  const { t } = useTranslation()
  const { toast } = useToast()
  const accessToken = session?.user?.accessToken

  const [apiKeys, setApiKeys] = useState<ApiKeyItemFragment[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | undefined>(undefined)

  const loadApiKeys = useCallback(async () => {
    if (!accessToken) {
      setApiKeys([])
      setLoading(false)
      return
    }
    setLoading(true)
    setError(undefined)
    try {
      const result = await fetchApiKeys(org, accessToken)
      setApiKeys(result.apiKeys)
    } catch (e) {
      console.error('Failed to load API keys:', e)
      setError(t.v1beta.apiKeyList.failedLoad)
    } finally {
      setLoading(false)
    }
  }, [org, accessToken, t.v1beta.apiKeyList.failedLoad])

  useEffect(() => {
    loadApiKeys()
  }, [loadApiKeys])

  const handleCreate = useCallback(
    async (orgUsername: string, name: string) => {
      const result = await createApiKey(orgUsername, name, accessToken)
      // The new key only reaches the list once it comes back from the
      // server, so the row and the dialog agree on what exists.
      await loadApiKeys()
      return result
    },
    [accessToken, loadApiKeys],
  )

  const handleRevoke = useCallback(
    async (apiKeyId: string) => {
      try {
        await revokeApiKey(org, apiKeyId, accessToken)
        toast({ title: t.v1beta.apiKeyList.revoked })
        await loadApiKeys()
      } catch (e) {
        console.error('Failed to revoke API key:', e)
        toast({
          title: t.v1beta.apiKeyDialog.error,
          description: t.v1beta.apiKeyList.failedRevoke,
          variant: 'destructive',
        })
      }
    },
    [org, accessToken, loadApiKeys, toast, t],
  )

  // Without a session there is no one to issue a key to, so the page
  // says so instead of offering controls that would only fail.
  const signedIn = Boolean(accessToken)

  return (
    <ApiPageUi
      org={org}
      repo={repo}
      apiBaseUrl={baseURL}
      docsUrl={docsURL}
      apiKeySlot={
        signedIn ? (
          <ApiKeyDialog orgUsername={org} onCreate={handleCreate} />
        ) : (
          <p className='text-sm text-muted-foreground'>
            {t.v1beta.apiKeyList.signInRequired}
          </p>
        )
      }
      apiKeyListSlot={
        signedIn ? (
          <ApiKeyList
            apiKeys={apiKeys}
            loading={loading}
            error={error}
            onRevoke={handleRevoke}
          />
        ) : (
          <p className='text-sm text-muted-foreground'>
            {t.v1beta.apiKeyList.signInRequired}
          </p>
        )
      }
    />
  )
}
