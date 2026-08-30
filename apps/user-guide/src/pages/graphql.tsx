import {
  Callout,
  Code,
  CodeBlock,
  LiveDataStatus,
  Prose,
  Section,
} from '@/components/primitives'
import { SchemaFieldList } from '@/components/schema-list'
import { apiBaseUrl, sampleOrg, sampleRepo } from '@/lib/config'
import { fetchSchemaSummary } from '@/lib/graphql-schema'
import { useAsync } from '@/lib/use-async'

export function GraphQL() {
  const schema = useAsync(fetchSchemaSummary)

  return (
    <>
      <div className='space-y-3'>
        <h1 className='text-3xl font-bold tracking-tight'>GraphQL API</h1>
        <p className='text-[15px] leading-7 text-slate-600 dark:text-slate-400'>
          REST が 1 エンドポイント 1 リソースなのに対し、GraphQL
          は必要なフィールドだけを 1 回の往復でまとめて取得できます。
          エンドポイントは 1 つです。
        </p>
        <CodeBlock code={`POST ${apiBaseUrl}/v1/graphql`} />
      </div>

      <Section title='API キーで呼ぶときは組織 ID が必要'>
        <Prose>
          <p>
            REST のパスには <Code>/repos/{sampleOrg}/{sampleRepo}</Code>{' '}
            のように組織名が含まれるため、サーバーはどの組織のキーとして検証すればよいか
            分かります。GraphQL のパスには組織名が含まれません。そのため、
            <strong>
              API キーで GraphQL を呼ぶときは <Code>x-operator-id</Code>{' '}
              ヘッダで組織 ID を指定してください。
            </strong>
          </p>
        </Prose>
        <CodeBlock
          language='bash'
          code={`curl -X POST "${apiBaseUrl}/v1/graphql" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY" \\
  -H "x-operator-id: $LIBRARY_ORG_ID" \\
  -H "Content-Type: application/json" \\
  -d '{"query": "{ apiKeys(orgUsername: \\"${sampleOrg}\\") { id name createdAt } }"}'`}
        />
        <Callout tone='warning' title='ヘッダを付け忘れると匿名になります'>
          <Code>x-operator-id</Code>{' '}
          がないと、キーは検証されずリクエストは匿名として扱われます。
          認証エラーではなく「権限がありません」や空の結果として返るため、
          気づきにくい挙動です。組織 ID は{' '}
          <Code>GET /v1beta/orgs/{sampleOrg}</Code> のレスポンスの{' '}
          <Code>id</Code> です。
        </Callout>
      </Section>

      <Section title='クエリの例'>
        <Prose>
          <p>REST なら 2 回に分かれる問い合わせが、1 回で済みます。</p>
        </Prose>
        <CodeBlock
          language='graphql'
          code={`query {
  repo(orgUsername: "${sampleOrg}", repoUsername: "${sampleRepo}") {
    id
    name
    description
    dataList(pageSize: 20, page: 1) {
      items {
        id
        name
        updatedAt
      }
      paginator {
        currentPage
        totalPages
        totalItems
      }
    }
  }
}`}
        />
      </Section>

      <Section title='エラーの読み方'>
        <Prose>
          <p>
            GraphQL は HTTP としては 200 を返しつつ、レスポンスの{' '}
            <Code>errors</Code> 配列に失敗を入れて返します。REST
            のようにステータスコードだけで判定すると、失敗を成功として扱ってしまいます。
          </p>
        </Prose>
        <CodeBlock
          language='json'
          code={`{
  "data": null,
  "errors": [
    {
      "message": "PermissionDenied: action: library:RevokeApiKey",
      "extensions": { "code": "FORBIDDEN" }
    }
  ]
}`}
        />
        <Prose>
          <p>
            <Code>errors</Code> が空でないかを必ず確認してください。
          </p>
        </Prose>
      </Section>

      <Section title='スキーマ'>
        <Prose>
          <p>
            以下はこの API が現在公開しているスキーマから生成しています。
            SDL 全体は{' '}
            <a
              href={`${apiBaseUrl}/v1/graphql/introspection`}
              target='_blank'
              rel='noopener noreferrer'
              className='font-medium text-sky-700 hover:underline dark:text-sky-400'
            >
              introspection エンドポイント
            </a>
            から、対話的な試行は{' '}
            <a
              href={`${apiBaseUrl}/v1/graphql`}
              target='_blank'
              rel='noopener noreferrer'
              className='font-medium text-sky-700 hover:underline dark:text-sky-400'
            >
              Playground
            </a>
            からできます。
          </p>
        </Prose>
        <LiveDataStatus
          loading={schema.loading}
          error={schema.error}
          loadingLabel='スキーマを取得しています...'
        />
        {schema.data && (
          <div className='space-y-6'>
            <div className='space-y-2'>
              <h3 className='text-sm font-semibold text-slate-500 dark:text-slate-400'>
                Query（{schema.data.queries.length} 件）
              </h3>
              <SchemaFieldList fields={schema.data.queries} />
            </div>
            <div className='space-y-2'>
              <h3 className='text-sm font-semibold text-slate-500 dark:text-slate-400'>
                Mutation（{schema.data.mutations.length} 件）
              </h3>
              <SchemaFieldList fields={schema.data.mutations} />
            </div>
          </div>
        )}
      </Section>
    </>
  )
}
