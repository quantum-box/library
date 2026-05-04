import { createFileRoute, Outlet, useRouterState } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { useEffect, useState } from 'react'
import { platformAction, PlatformActionError } from '@/app/v1beta/_lib/platform-action'
import { canEdit } from '@/app/v1beta/_lib/repo-permissions'
import { type RepositoryPageQuery } from '@/gen/graphql'
import { DataViewComponent } from '@/app/v1beta/[org]/[repo]/data/components/data-view'
import { RepoSkeleton } from '@/app/v1beta/[org]/[repo]/components/repo-skeleton'

export const Route = createFileRoute('/v1beta/$org/$repo/data')({
  component: DataRoute,
})

function DataRoute() {
  const { org, repo } = Route.useParams()
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  if (pathname !== `/v1beta/${org}/${repo}/data`) {
    return <Outlet />
  }
  return <DataPageContent />
}

function DataPageContent() {
  const { org, repo } = Route.useParams()
  const search = Route.useSearch() as { page?: string; pageSize?: string }
  const { session, isLoading: isAuthLoading } = useAuth()
  const [repoData, setRepoData] = useState<RepositoryPageQuery['repo'] | null>(null)
  const [hasEditPermission, setHasEditPermission] = useState(false)
  const [loading, setLoading] = useState(true)

  const page = search.page ? Number.parseInt(search.page, 10) : 1
  const pageSize = search.pageSize ? Number.parseInt(search.pageSize, 10) : 20

  useEffect(() => {
    if (isAuthLoading) return

    const fetchData = async () => {
      setLoading(true)
      setHasEditPermission(false)
      try {
        let data: RepositoryPageQuery['repo'] | null = null
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

        setRepoData(data)

        if (data && session?.user) {
          const editResult = await canEdit(org, repo, session.user.id, session.user.accessToken)
          if (editResult.isOk()) {
            setHasEditPermission(editResult.value)
          }
        }
      } catch (e) {
        console.error('Failed to load repo data:', e)
        setRepoData(null)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [
    org,
    repo,
    page,
    pageSize,
    session?.user?.id,
    session?.user?.accessToken,
    isAuthLoading,
  ])

  if (loading) return <RepoSkeleton />

  if (!repoData) {
    return (
      <div className='flex flex-col items-center justify-center min-h-[50vh]'>
        <h1 className='text-2xl font-bold mb-2'>Repository not found</h1>
      </div>
    )
  }

  return (
    <DataViewComponent
      org={org}
      repo={repo}
      dataList={{
        items: repoData.dataList.items,
        paginator: repoData.dataList.paginator,
      }}
      properties={repoData.properties}
      canEdit={hasEditPermission}
    />
  )
}
