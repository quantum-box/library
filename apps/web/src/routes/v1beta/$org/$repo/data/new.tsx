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

export const Route = createFileRoute('/v1beta/$org/$repo/data/new')({
  component: NewDataPage,
})

function NewDataPage() {
  const { org, repo } = Route.useParams()
  const { session, isLoading: isAuthLoading } = useAuth()
  const [properties, setProperties] = useState<PropertyForEditorFragment[]>([])
  const [dataList, setDataList] = useState<DataListForDataListCardFragment>()
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (isAuthLoading) return
    if (session?.user && !session.user.accessToken) {
      setLoading(false)
      return
    }

    const fetchEditorData = async () => {
      setLoading(true)
      try {
        const result = await platformAction(
          (sdk) => sdk.repositoryPage({ org, repo, page: 1, pageSize: 50 }),
          {
            onError: () => {},
            allowAnonymous: true,
            accessToken: session?.user?.accessToken,
          },
        )
        setProperties((result?.repo?.properties ?? []) as PropertyForEditorFragment[])
        setDataList(result?.repo?.dataList as DataListForDataListCardFragment)
      } catch (error) {
        console.error('Failed to load data editor:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchEditorData()
  }, [org, repo, session?.user?.accessToken, isAuthLoading])

  const draftData: DataForDataDetailFragment = {
    __typename: 'Data',
    id: '',
    name: '',
    propertyData: [],
  }

  const handleSave = async ({
    properties,
    input,
  }: {
    properties: PropertyForEditorFragment[]
    input: DataForDataDetailFragment
  }) => {
    if (!session?.user) throw new Error('Sign in is required to create data.')
    const result = await platformAction(
      (sdk) =>
        sdk.addData({
          input: {
            actor: session.user.id,
            orgUsername: org,
            repoUsername: repo,
            dataName: input.name || 'Untitled',
            propertyData: convertPropertyData(properties, input.propertyData),
          },
        }),
      {
        accessToken: session.user.accessToken,
      },
    )
    return result.addData.id
  }

  if (loading) {
    return (
      <Card className='m-6'>
        <CardContent className='py-10 text-center text-sm text-muted-foreground'>
          Loading editor...
        </CardContent>
      </Card>
    )
  }

  return (
    <DataDetailUi
      data={draftData}
      properties={properties}
      dataList={dataList}
      onSave={handleSave}
      onlyEdit
      viewOnly={!session?.user}
    />
  )
}
