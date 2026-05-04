import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { DataDetailUi } from '@/app/v1beta/_components/data-detail-ui'
import { convertPropertyData } from '@/app/v1beta/_lib/property-data-converter'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { Card, CardContent } from '@/components/ui/card'
import {
  DataForDataDetailFragment,
  DataListForDataListCardFragment,
  PropertyForEditorFragment,
} from '@/gen/graphql'
import { useEffect, useState } from 'react'

export const Route = createFileRoute('/v1beta/$org/$repo/data/$dataId')({
  component: DataDetailPage,
})

function DataDetailPage() {
  const { org, repo, dataId } = Route.useParams()
  const { session, isLoading: isAuthLoading } = useAuth()
  const [data, setData] = useState<DataForDataDetailFragment>()
  const [properties, setProperties] = useState<PropertyForEditorFragment[]>([])
  const [dataList, setDataList] = useState<DataListForDataListCardFragment>()
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (isAuthLoading) return
    if (session?.user && !session.user.accessToken) {
      setLoading(false)
      return
    }

    const fetchDetail = async () => {
      setLoading(true)
      try {
        const result = await platformAction(
          (sdk) => sdk.repositoryPage({ org, repo, page: 1, pageSize: 100 }),
          {
            onError: () => {},
            allowAnonymous: true,
            accessToken: session?.user?.accessToken,
          },
        )
        const repoData = result?.repo
        const items = (repoData?.dataList?.items ?? []) as Array<{ id: string }>
        setData(items.find((item) => item.id === dataId) as DataForDataDetailFragment | undefined)
        setProperties((repoData?.properties ?? []) as PropertyForEditorFragment[])
        setDataList(repoData?.dataList as DataListForDataListCardFragment)
      } catch (error) {
        console.error('Failed to load data detail:', error)
        setData(undefined)
      } finally {
        setLoading(false)
      }
    }

    fetchDetail()
  }, [org, repo, dataId, session?.user?.accessToken, isAuthLoading])

  const handleSave = async ({
    properties,
    input,
  }: {
    properties: PropertyForEditorFragment[]
    input: DataForDataDetailFragment
  }) => {
    if (!session?.user) throw new Error('Sign in is required to update data.')
    const result = await platformAction(
      (sdk) =>
        sdk.updateData({
          input: {
            actor: session.user.id,
            orgUsername: org,
            repoUsername: repo,
            dataId,
            dataName: input.name || 'Untitled',
            propertyData: convertPropertyData(properties, input.propertyData),
          },
        }),
      {
        accessToken: session.user.accessToken,
      },
    )
    setData(result.updateData)
    return result.updateData.id
  }

  if (loading) {
    return (
      <Card className='m-6'>
        <CardContent className='py-10 text-center text-sm text-muted-foreground'>
          Loading data...
        </CardContent>
      </Card>
    )
  }

  if (!data) {
    return (
      <Card className='m-6'>
        <CardContent className='py-10 text-center'>
          <h1 className='text-xl font-semibold'>Data not found</h1>
        </CardContent>
      </Card>
    )
  }

  return (
    <DataDetailUi
      data={data}
      properties={properties}
      dataList={dataList}
      onSave={handleSave}
      viewOnly={!session?.user}
    />
  )
}
