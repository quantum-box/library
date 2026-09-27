import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { DataDetailUi } from '@/app/v1beta/_components/data-detail-ui'
import { RichTextTemplateManager } from '@/app/v1beta/_components/rich-text-template-manager'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { convertPropertyData } from '@/app/v1beta/_lib/property-data-converter'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { Card, CardContent } from '@/components/ui/card'
import {
  DataForDataDetailFragment,
  DataListForDataListCardFragment,
  PropertyForEditorFragment,
  RichTextTemplate,
} from '@/gen/graphql'
import { useEffect, useState } from 'react'

export const Route = createFileRoute('/v1beta/$org/$repo/data/new')({
  component: NewDataPage,
})

function NewDataPage() {
  const { org, repo } = Route.useParams()
  const { session, isLoading: isAuthLoading } = useAuth()
  const { t } = useTranslation()
  const [properties, setProperties] = useState<PropertyForEditorFragment[]>([])
  const [dataList, setDataList] = useState<DataListForDataListCardFragment>()
  const [canManageTemplates, setCanManageTemplates] = useState(false)
  const [richTextTemplates, setRichTextTemplates] = useState<RichTextTemplate[]>([])
  const [templatesLoading, setTemplatesLoading] = useState(false)
  const [templatesError, setTemplatesError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (isAuthLoading) return
    if (session?.user && !session.user.accessToken) {
      setLoading(false)
      return
    }

    const fetchEditorData = async () => {
      setLoading(true)
      setCanManageTemplates(false)
      setRichTextTemplates([])
      setTemplatesError(null)
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
        const canManage = Boolean(
          session?.user && result?.repo?.policies?.some(
            policy =>
              policy.userId === session.user.id &&
              (policy.role === 'writer' || policy.role === 'owner'),
          ),
        )
        setCanManageTemplates(canManage)
      } catch (error) {
        console.error('Failed to load data editor:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchEditorData()
  }, [org, repo, session?.user?.accessToken, isAuthLoading])

  useEffect(() => {
    const accessToken = session?.user?.accessToken
    if (!canManageTemplates || !accessToken) {
      setRichTextTemplates([])
      setTemplatesLoading(false)
      return
    }

    let cancelled = false
    setTemplatesLoading(true)
    setTemplatesError(null)
    void platformAction(
      sdk => sdk.richTextTemplates({ orgUsername: org, repoUsername: repo }),
      { accessToken },
    )
      .then(result => {
        if (!cancelled) setRichTextTemplates(result.richTextTemplates)
      })
      .catch(error => {
        if (!cancelled) {
          setTemplatesError(
            error instanceof Error ? error.message : t.v1beta.richTextTemplates.loadFailed,
          )
        }
      })
      .finally(() => {
        if (!cancelled) setTemplatesLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [org, repo, canManageTemplates, session?.user?.accessToken, t.v1beta.richTextTemplates.loadFailed])

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
    <>
      {canManageTemplates && session?.user?.accessToken ? (
        <RichTextTemplateManager
          org={org}
          repo={repo}
          accessToken={session.user.accessToken}
          templates={richTextTemplates}
          onTemplatesChange={setRichTextTemplates}
          loading={templatesLoading}
          loadError={templatesError}
        />
      ) : null}
      <DataDetailUi
        data={draftData}
        properties={properties}
        dataList={dataList}
        onSave={handleSave}
        onlyEdit
        viewOnly={!session?.user}
        richTextTemplates={richTextTemplates}
      />
    </>
  )
}
