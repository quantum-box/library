import { EndpointGroups } from '@/components/endpoint-table'
import {
  Code,
  CodeBlock,
  LiveDataStatus,
  Prose,
  Section,
} from '@/components/primitives'
import { apiBaseUrl, sampleOrg, sampleRepo } from '@/lib/config'
import { fetchApiDocument } from '@/lib/openapi'
import { useAsync } from '@/lib/use-async'

const ERRORS: [string, string, string][] = [
  ['400', 'リクエストが不正', '必須項目の欠落、値の形式違い'],
  ['401', '認証されていない', 'キーの指定漏れ、失効済みのキー'],
  ['403', '権限がない', 'その組織・リポジトリへの権限不足'],
  ['404', '対象が見つからない', '組織名・リポジトリ名・ID の誤り'],
  ['409', '競合', '既に存在する名前での作成など'],
  ['500', 'サーバー側エラー', '時間をおいて再試行'],
]

export function Rest() {
  const api = useAsync(fetchApiDocument)

  return (
    <>
      <div className='space-y-3'>
        <h1 className='text-3xl font-bold tracking-tight'>REST API</h1>
        <p className='text-[15px] leading-7 text-slate-600 dark:text-slate-400'>
          認証は全エンドポイント共通で{' '}
          <Code>Authorization: Bearer $LIBRARY_API_KEY</Code> です。
          パスはすべてベース URL <Code>{apiBaseUrl}</Code> からの相対です。
        </p>
      </div>

      <Section title='よく使う操作'>
        <Prose>
          <p>
            組織 <Code>{sampleOrg}</Code>、リポジトリ <Code>{sampleRepo}</Code>{' '}
            を対象にした例です。
          </p>
        </Prose>
        <CodeBlock
          language='bash'
          code={`# データ一覧
curl "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data-list" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY"

# 1 件取得
curl "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data/DATA_ID" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY"

# 作成
curl -X POST "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"name": "My Data", "properties": {}}'

# 削除（成功時は 204 No Content）
curl -X DELETE "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data/DATA_ID" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY"`}
        />
      </Section>

      <Section title='組織 ID を調べる'>
        <Prose>
          <p>
            GraphQL を API キーで呼ぶときに必要になる組織 ID (<Code>tn_</Code>{' '}
            で始まる値) は、組織情報のレスポンスに含まれています。
          </p>
        </Prose>
        <CodeBlock
          language='bash'
          code={`curl "${apiBaseUrl}/v1beta/orgs/${sampleOrg}" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY"`}
        />
      </Section>

      <Section title='エンドポイント一覧'>
        <Prose>
          <p>
            この一覧は API が公開している OpenAPI ドキュメントから生成しています。
            手で書いた表ではないため、API に追加されたエンドポイントはここにも現れます。
            {api.data && (
              <>
                {' '}
                現在 {api.data.operationCount} 件（{api.data.title} v
                {api.data.version}）。
              </>
            )}
          </p>
        </Prose>
        <LiveDataStatus
          loading={api.loading}
          error={api.error}
          loadingLabel='エンドポイント一覧を取得しています...'
        />
        {api.data && <EndpointGroups groups={api.data.groups} />}
        <Prose>
          <p>
            リクエストボディやレスポンスの詳細は、同じ OpenAPI から生成されている{' '}
            <a
              href={`${apiBaseUrl}/v1beta/swagger-ui`}
              target='_blank'
              rel='noopener noreferrer'
              className='font-medium text-sky-700 hover:underline dark:text-sky-400'
            >
              Swagger UI
            </a>{' '}
            で確認できます。
          </p>
        </Prose>
      </Section>

      <Section title='エラー'>
        <div className='overflow-hidden rounded-lg border border-slate-200 dark:border-slate-800'>
          <table className='w-full text-sm'>
            <thead className='bg-slate-50 text-left dark:bg-slate-900'>
              <tr>
                <th className='px-4 py-2 font-medium'>ステータス</th>
                <th className='px-4 py-2 font-medium'>意味</th>
                <th className='px-4 py-2 font-medium'>主な原因</th>
              </tr>
            </thead>
            <tbody className='divide-y divide-slate-200 dark:divide-slate-800'>
              {ERRORS.map(([status, meaning, cause]) => (
                <tr key={status}>
                  <td className='px-4 py-2 font-mono'>{status}</td>
                  <td className='px-4 py-2'>{meaning}</td>
                  <td className='px-4 py-2 text-slate-600 dark:text-slate-400'>
                    {cause}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <Prose>
          <p>
            401 が返るときは、まず開発者ポータルの一覧でそのキーがまだ有効かを
            確認してください。失効させたキーは即座に認証されなくなります。
          </p>
        </Prose>
      </Section>
    </>
  )
}
