import { createFileRoute } from '@tanstack/react-router'
import { ApiPageUi } from '@/app/v1beta/[org]/[repo]/api/_components/api-page-ui'
import { baseURL } from '@/lib/apiClient'

export const Route = createFileRoute('/v1beta/$org/$repo/api')({
  component: ApiPage,
})

function ApiPage() {
  const { org, repo } = Route.useParams()

  return (
    <ApiPageUi
      org={org}
      repo={repo}
      apiBaseUrl={baseURL}
    />
  )
}
