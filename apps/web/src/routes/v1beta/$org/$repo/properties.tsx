import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { PropertyType } from '@/gen/graphql'
import { Search, SlidersHorizontal } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

type PropertyMeta = {
	__typename?: string
	options?: Array<{
		__typename?: 'SelectItem'
		id: string
		key: string
		name: string
	}> | null
	databaseId?: string | null
	json?: string | null
}

type Property = {
	id: string
	name: string
	typ: PropertyType
	meta?: PropertyMeta | null
}

export const Route = createFileRoute('/v1beta/$org/$repo/properties')({
	component: PropertiesPage,
})

function PropertiesPage() {
  const { org, repo } = Route.useParams()
  const { session, isLoading: isAuthLoading } = useAuth()
  const [properties, setProperties] = useState<Property[]>([])
  const [loading, setLoading] = useState(true)
  const [query, setQuery] = useState('')

  useEffect(() => {
    if (isAuthLoading) return
    if (session?.user && !session.user.accessToken) {
      setLoading(false)
      return
    }

    const fetchProperties = async () => {
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
        setProperties((result?.repo?.properties ?? []) as Property[])
      } catch (error) {
        console.error('Failed to load properties:', error)
    setProperties([])
      } finally {
        setLoading(false)
      }
    }

    fetchProperties()
  }, [org, repo, session?.user?.accessToken, isAuthLoading])

  const filteredProperties = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase()
    if (!normalizedQuery) return properties
    return properties.filter((property) =>
      [property.name, property.typ, property.id]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(normalizedQuery)),
    )
  }, [properties, query])

  return (
    <div className='container py-6 space-y-6'>
      <div className='flex flex-wrap items-center justify-between gap-3'>
        <div>
          <h1 className='text-2xl font-bold'>プロパティ</h1>
          <p className='text-sm text-muted-foreground'>
            {repo} のデータ項目を管理します。
          </p>
        </div>
        <Button disabled>
          <SlidersHorizontal className='mr-2 h-4 w-4' />
          新しいプロパティ
        </Button>
      </div>

      <Card>
        <CardHeader className='space-y-4'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <CardTitle className='text-lg'>プロパティ一覧</CardTitle>
            <Badge variant='secondary'>{properties.length} 件</Badge>
          </div>
          <div className='relative max-w-sm'>
            <Search className='absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground' />
            <Input
              placeholder='Search properties'
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              className='pl-9'
            />
          </div>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className='py-10 text-center text-sm text-muted-foreground'>Loading...</div>
          ) : filteredProperties.length === 0 ? (
            <div className='py-10 text-center text-sm text-muted-foreground'>No properties</div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Options / Related</TableHead>
                  <TableHead>ID</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredProperties.map((property) => (
                  <TableRow key={property.id}>
                    <TableCell className='font-medium'>{property.name}</TableCell>
                    <TableCell>
                      <Badge variant='outline'>{formatPropertyType(property.typ)}</Badge>
                    </TableCell>
                    <TableCell className='text-sm text-muted-foreground'>
                      {formatPropertyMeta(property)}
                    </TableCell>
                    <TableCell className='font-mono text-xs text-muted-foreground'>
                      {property.id}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function formatPropertyType(type: PropertyType | string) {
  return String(type).replace(/_/g, ' ')
}

function formatPropertyMeta(property: Property): string {
  const meta = property.meta
  if (!meta) return '-'
  if (meta.__typename === 'SelectType' || meta.__typename === 'MultiSelectType') {
    return (meta.options ?? []).map((option) => option.name).join(', ') || '-'
  }
  if (meta.__typename === 'RelationType') return meta.databaseId ?? '-'
  if (meta.__typename === 'JsonType') return meta.json ?? '-'
  return property.typ === PropertyType.Select || property.typ === PropertyType.MultiSelect ? '-' : ''
}
