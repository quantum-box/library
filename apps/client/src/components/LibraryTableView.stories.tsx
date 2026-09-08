import type { Meta, StoryObj } from '@storybook/react-vite'
import { expect, within } from 'storybook/test'
import { en } from '../i18n/messages/en'
import { LibraryTableView } from './LibraryTableView'

/**
 * The repository data view reads its rows from the Library API rather than
 * taking them as props, so the story answers that one GraphQL call instead of
 * passing a fixture in. Types are written the way the API serializes them --
 * SCREAMING_SNAKE -- so the story exercises the same normalization the app does.
 */
const properties: Array<{
  id: string
  name: string
  typ: string
  meta?: { options: Array<{ id: string; name: string }> }
}> = [
  { id: 'prop-id', name: 'id', typ: 'ID' },
  { id: 'prop-content', name: 'content', typ: 'MARKDOWN' },
  {
    id: 'prop-status',
    name: 'status',
    typ: 'SELECT',
    meta: {
      options: [
        { id: 'option-published', name: 'Published' },
        { id: 'option-draft', name: 'Draft' },
      ],
    },
  },
  { id: 'prop-owner', name: 'owner', typ: 'STRING' },
  { id: 'prop-public', name: 'public', typ: 'BOOLEAN' },
]

const items = [
  {
    id: 'data_01m1xy86q9c8wwnk6mtpgcd6rn',
    name: 'Tachyon Feature Flag 機能ガイド',
    updatedAt: '2026-09-01T10:00:00.000Z',
    propertyData: [
      { propertyId: 'prop-id', value: { id: 'data_01m1xy86q9c8wwnk6mtpgcd6rn' } },
      {
        propertyId: 'prop-content',
        value: {
          markdown:
            '定義・条件・継承・評価経路・manifest・分析を整理したガイドです。全体像は定義からアプリ配信まで。',
        },
      },
      { propertyId: 'prop-status', value: { optionId: 'option-published' } },
      { propertyId: 'prop-owner', value: { string: 'takanori' } },
      { propertyId: 'prop-public', value: { boolean: true } },
    ],
  },
  {
    id: 'data_01m1xy86stfq7bw3zfyyck3ph0',
    name: 'Release checklist',
    updatedAt: '2026-08-20T10:00:00.000Z',
    propertyData: [
      { propertyId: 'prop-id', value: { id: 'data_01m1xy86stfq7bw3zfyyck3ph0' } },
      { propertyId: 'prop-content', value: { markdown: 'Steps to cut a desktop release.' } },
      { propertyId: 'prop-status', value: { optionId: 'option-draft' } },
      { propertyId: 'prop-public', value: { boolean: false } },
    ],
  },
  ...Array.from({ length: 8 }, (_, index) => ({
    id: `data_sample_${index}`,
    name: `サンプルレコード ${index + 1}`,
    updatedAt: '2026-07-14T10:00:00.000Z',
    propertyData: [
      { propertyId: 'prop-id', value: { id: `data_01m1xy86stfq7bw3zfyyck3p0${index}` } },
      {
        propertyId: 'prop-content',
        value: { markdown: '本文のプレビューはセルの幅で切り詰められ、行の高さを崩しません。' },
      },
      {
        propertyId: 'prop-status',
        value: { optionId: index % 2 === 0 ? 'option-draft' : 'option-published' },
      },
      { propertyId: 'prop-owner', value: { string: 'quantumbox' } },
      { propertyId: 'prop-public', value: { boolean: index % 3 === 0 } },
    ],
  })),
]

function repoPayload(rows: typeof items, repoProperties: typeof properties = properties) {
  return {
    data: {
      repo: {
        id: 'repo_01m1xy86q9c8wwnk6mtpgcd6rn',
        name: 'tachyonuserguide',
        properties: repoProperties,
        dataList: {
          items: rows,
          paginator: { totalPages: 1, totalItems: rows.length },
        },
      },
    },
  }
}

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

/**
 * A repository the story can actually edit.
 *
 * The listing, and the Property mutations the header runs, are answered from
 * one mutable copy of the fixture, so adding, renaming, and deleting a column
 * behave here the way they do against a real repository -- the reader of the
 * story can try them rather than read about them.
 *
 * Everything that is not the Library GraphQL endpoint goes to the real
 * `fetch`: a blanket stub would swallow the dev server's own module requests.
 */
function stubRepository(rows: typeof items) {
  return () => {
    const original = window.fetch
    const liveProperties = properties.map((property) => ({ ...property }))
    const liveRows = rows.map((row) => ({ ...row }))
    let created = 0

    const stub: typeof window.fetch = async (input, init) => {
      const url =
        typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
      if (!url.includes('/v1/graphql')) return original(input, init)

      const body = JSON.parse(String(init?.body ?? '{}')) as {
        query?: string
        variables?: {
          id?: string
          dataId?: string
          // The Property mutations name their fields this way on the wire.
          input?: {
            propertyName?: string
            propertyType?: string
            dataId?: string
            name?: string
            propertyData?: Array<{ propertyId: string; value: Record<string, unknown> }>
          }
        }
      }
      const query = body.query ?? ''
      const variables = body.variables ?? {}

      if (query.includes('addProperty')) {
        created += 1
        const property = {
          id: `prop-new-${created}`,
          name: variables.input?.propertyName ?? 'New property',
          typ: variables.input?.propertyType ?? 'STRING',
        }
        liveProperties.push(property)
        return jsonResponse({ data: { addProperty: property } })
      }
      if (query.includes('updateProperty')) {
        const property = liveProperties.find((entry) => entry.id === variables.id)
        if (property && variables.input?.propertyName) {
          property.name = variables.input.propertyName
        }
        return jsonResponse({ data: { updateProperty: property ?? null } })
      }
      if (query.includes('deleteProperty')) {
        const index = liveProperties.findIndex((entry) => entry.id === variables.id)
        if (index >= 0) liveProperties.splice(index, 1)
        return jsonResponse({ data: { deleteProperty: true } })
      }

      if (query.includes('updateData')) {
        const row = liveRows.find((entry) => entry.id === variables.input?.dataId)
        if (row) {
          if (variables.input?.name) row.name = variables.input.name
          for (const entry of variables.input?.propertyData ?? []) {
            const existing = row.propertyData.find(
              (candidate) => candidate.propertyId === entry.propertyId,
            )
            if (existing) existing.value = entry.value as typeof existing.value
            else row.propertyData.push(entry as (typeof row.propertyData)[number])
          }
        }
        return jsonResponse({ data: { updateData: row ?? null } })
      }
      if (query.includes('addData')) {
        const row = {
          id: `data_added_${liveRows.length}`,
          name: variables.input?.name ?? 'Untitled',
          updatedAt: new Date().toISOString(),
          propertyData: [],
        }
        liveRows.unshift(row as (typeof liveRows)[number])
        return jsonResponse({ data: { addData: row } })
      }
      if (query.includes('deleteData')) {
        const index = liveRows.findIndex((entry) => entry.id === variables.dataId)
        if (index >= 0) liveRows.splice(index, 1)
        return jsonResponse({ data: { deleteData: variables.dataId ?? '' } })
      }

      return jsonResponse(repoPayload(liveRows, liveProperties))
    }

    window.fetch = stub
    return () => {
      window.fetch = original
    }
  }
}

const meta = {
  title: 'Library/LibraryTableView',
  component: LibraryTableView,
  tags: ['autodocs'],
  parameters: {
    layout: 'fullscreen',
  },
  decorators: [
    (Story) => (
      <div className="h-[640px] bg-background">
        <Story />
      </div>
    ),
  ],
  args: {
    org: 'quantumbox',
    repo: 'tachyonuserguide',
    repoLabel: 'quantumbox/tachyonuserguide',
    selectedDataId: 'data_01m1xy86q9c8wwnk6mtpgcd6rn',
    onSelectData: () => undefined,
  },
  beforeEach: stubRepository(items),
} satisfies Meta<typeof LibraryTableView>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByTestId('library-table-view')).toBeVisible()
    await expect(
      await canvas.findByText('Tachyon Feature Flag 機能ガイド')
    ).toBeVisible()
    // Every Select value renders as a badge, so several rows carry this one.
    await expect(canvas.getAllByText('Published')[0]).toBeVisible()
  },
}

/**
 * The table as a reader left it: two columns hidden, the rest reordered, and
 * one of them dragged wider. The arrangement lives on the device, so the story
 * seeds it the same way the table would have.
 */
export const Arranged: Story = {
  beforeEach: () => {
    window.localStorage.setItem(
      'library-client-table-layout-quantumbox/tachyonuserguide',
      JSON.stringify({
        order: ['prop-status', 'prop-content', 'prop-owner', 'prop-id', 'prop-public'],
        hidden: ['prop-id', 'prop-public'],
        widths: { 'prop-content': 340 },
      }),
    )
    const restore = stubRepository(items)()
    return () => {
      restore()
      window.localStorage.removeItem('library-client-table-layout-quantumbox/tachyonuserguide')
    }
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByTestId('library-table-header-prop-status')).toBeVisible()
    // A hidden Property keeps its row values but leaves the header row.
    await expect(canvas.queryByTestId('library-table-header-prop-public')).toBeNull()
  },
}

export const Empty: Story = {
  args: {
    selectedDataId: null,
  },
  beforeEach: stubRepository([]),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByTestId('library-table-empty')).toHaveTextContent(
      en['libraryTable.empty']
    )
  },
}
