import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { useEffect, useState } from 'react'
import { platformAction, PlatformActionError } from '@/app/v1beta/_lib/platform-action'
import { canEdit } from '@/app/v1beta/_lib/repo-permissions'
import {
  RepositoryUi,
} from '@/app/v1beta/[org]/[repo]/components/repository-ui'
import { RepoSkeleton } from '@/app/v1beta/[org]/[repo]/components/repo-skeleton'

export const Route = createFileRoute('/v1beta/$org/$repo/')({
  component: RepositoryPage,
})

function RepositoryPage() {
  const { org, repo } = Route.useParams()
  const search = Route.useSearch() as { page?: string; pageSize?: string }
  const { session } = useAuth()
  const [repoData, setRepoData] = useState<any>(null)
  const [tags, setTags] = useState<string[]>([])
  const [hasEditPermission, setHasEditPermission] = useState(false)
  const [loading, setLoading] = useState(true)

  const page = search.page ? parseInt(search.page, 10) : 1
  const pageSize = search.pageSize ? parseInt(search.pageSize, 10) : 20

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true)
      try {
        // Try with tags first
        let data: any = null
        try {
          const result = await platformAction(
            (sdk) => sdk.repositoryPageWithTags({ org, repo, page, pageSize }),
            {
              onError: () => {},
              allowAnonymous: true,
              accessToken: session?.user?.accessToken,
            },
          )
          if (result?.repo) {
            data = result.repo
            setTags(result.repo.tags ?? [])
          }
        } catch (e) {
          if (e instanceof PlatformActionError && e.message.includes('Unknown field "tags"')) {
            const result = await platformAction(
              (sdk) => sdk.repositoryPage({ org, repo, page, pageSize }),
              {
                onError: () => {},
                allowAnonymous: true,
                accessToken: session?.user?.accessToken,
              },
            )
            if (result?.repo) {
              data = result.repo
            }
          } else {
            throw e
          }
        }

        if (data) {
          setRepoData(data)
        }

        // Check edit permission
        if (session?.user) {
          const editResult = await canEdit(org, repo, session.user.id, session.user.accessToken)
          if (editResult.isOk()) {
            setHasEditPermission(editResult.value)
          }
        }
      } catch (e) {
        console.error('Failed to load repo:', e)
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [org, repo, page, pageSize, session?.user?.accessToken])

  if (loading) return <RepoSkeleton />

  if (!repoData) {
    return (
      <div className='flex flex-col items-center justify-center min-h-[50vh]'>
        <h1 className='text-2xl font-bold mb-2'>Repository not found</h1>
      </div>
    )
  }

  const sources = repoData.sources?.map((source: any, index: number) => ({
    id: source.id,
    name: source.name,
    url: source.url ?? '',
    isPrimary: index === 0,
  }))

  const contributors = repoData.policies.map((policy: any) => ({
    userId: policy.userId,
    role: policy.role,
    name: policy.user?.username ?? policy.user?.name ?? undefined,
    avatarUrl: policy.user?.image ?? undefined,
  }))

  const hasGitHubSync = repoData.properties.some((p: any) => p.name === 'ext_github')

  return (
    <RepositoryUi
      repoName={repoData.name}
      dataList={{
        items: repoData.dataList.items,
        paginator: repoData.dataList.paginator,
      }}
      properties={repoData.properties}
      about={repoData.description ?? ''}
      labels={tags}
      sources={sources}
      contributors={contributors}
      org={org}
      repo={repo}
      isPublic={repoData.isPublic}
      hasGitHubSync={hasGitHubSync}
    />
  )
}
