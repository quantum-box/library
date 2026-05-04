import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'
import { RepoSkeleton } from '@/app/v1beta/[org]/[repo]/components/repo-skeleton'
import { Eye, GitBranch, Lock, Users } from 'lucide-react'
import { useEffect, useState } from 'react'
import { type RepositoryPageQuery } from '@/gen/graphql'

export const Route = createFileRoute('/v1beta/$org/$repo/settings')({
  component: SettingsPage,
})
type RepoData = RepositoryPageQuery['repo']
type RepoPolicy = RepoData['policies'][number]

function SettingsPage() {
  const { org, repo } = Route.useParams()
  const { session } = useAuth()
  const [repoData, setRepoData] = useState<RepoData | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (session?.user && !session.user.accessToken) {
      setLoading(false)
      return
    }

    const fetchSettings = async () => {
      setLoading(true)
      try {
        const result = await platformAction(
          (sdk) => sdk.repositoryPage({ org, repo, page: 1, pageSize: 1 }),
          {
            onError: () => {},
            allowAnonymous: true,
            accessToken: session?.user?.accessToken,
          },
        )
        setRepoData(result?.repo ?? null)
      } catch (error) {
        console.error('Failed to load repo settings:', error)
        setRepoData(null)
      } finally {
        setLoading(false)
      }
    }

    fetchSettings()
  }, [org, repo, session?.user?.accessToken])

  const contributors = repoData?.policies ?? []

  return (
    <div className='container py-6 space-y-6'>
      <div>
        <h1 className='text-2xl font-bold'>設定</h1>
        <p className='text-sm text-muted-foreground'>
          {org}/{repo} の基本情報と権限を確認します。
        </p>
      </div>

      {loading ? (
        <Card>
          <CardContent className='py-10 text-center text-sm text-muted-foreground'>
            Loading settings...
          </CardContent>
        </Card>
      ) : !repoData ? (
        <Card>
          <CardContent className='py-10 text-center'>
            <h2 className='text-xl font-semibold'>Repository not found</h2>
          </CardContent>
        </Card>
      ) : (
        <>
      <div className='grid gap-4 md:grid-cols-3'>
        <Card>
          <CardHeader className='pb-3'>
            <CardTitle className='flex items-center gap-2 text-base'>
              <GitBranch className='h-4 w-4' />
              Repository
            </CardTitle>
          </CardHeader>
          <CardContent className='space-y-3'>
            <div>
              <div className='text-xs text-muted-foreground'>Name</div>
              <div className='font-medium'>{repoData.name}</div>
            </div>
            <div>
              <div className='text-xs text-muted-foreground'>Description</div>
              <div className='text-sm'>{repoData.description || '-'}</div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className='pb-3'>
            <CardTitle className='flex items-center gap-2 text-base'>
              {repoData.isPublic ? <Eye className='h-4 w-4' /> : <Lock className='h-4 w-4' />}
              Visibility
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Badge variant={repoData.isPublic ? 'secondary' : 'outline'}>
              {repoData.isPublic ? 'Public' : 'Private'}
            </Badge>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className='pb-3'>
            <CardTitle className='flex items-center gap-2 text-base'>
              <Users className='h-4 w-4' />
              Members
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className='text-2xl font-semibold'>{contributors.length}</div>
            <div className='text-xs text-muted-foreground'>repository policies</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className='text-lg'>アクセス権限</CardTitle>
        </CardHeader>
        <CardContent className='space-y-3'>
          {contributors.length === 0 ? (
            <div className='text-sm text-muted-foreground'>No policies</div>
          ) : (
            contributors.map((policy: RepoPolicy) => (
              <div key={policy.userId} className='flex flex-wrap items-center justify-between gap-3'>
                <div>
                  <div className='font-medium'>
                    {policy.user?.username ?? policy.user?.name ?? policy.userId}
                  </div>
                  <div className='text-xs text-muted-foreground'>{policy.userId}</div>
                </div>
                <Badge variant='outline'>{policy.role}</Badge>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <Separator />

      <p className='text-xs text-muted-foreground'>
        編集・削除などの変更操作は、API 互換性を確認してから有効化します。
      </p>
        </>
      )}
    </div>
  )
}
