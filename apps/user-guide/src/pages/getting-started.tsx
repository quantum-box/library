import {
  Callout,
  Code,
  CodeBlock,
  LiveDataStatus,
  Prose,
  Section,
} from '@/components/primitives'
import { apiBaseUrl, clientUrl, sampleOrg, sampleRepo } from '@/lib/config'
import { fetchApiDocument } from '@/lib/openapi'
import { useAsync } from '@/lib/use-async'
import { Link } from 'react-router-dom'

export function GettingStarted() {
  const api = useAsync(fetchApiDocument)

  const snippets = {
    curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data-list" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY"`,
    python: `import os
import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data-list",
    headers={"Authorization": f"Bearer {os.environ['LIBRARY_API_KEY']}"},
)
print(response.json())`,
    javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${sampleOrg}/${sampleRepo}/data-list",
  { headers: { Authorization: \`Bearer \${process.env.LIBRARY_API_KEY}\` } },
)
console.log(await response.json())`,
  }

  return (
    <>
      <div className='space-y-3'>
        <h1 className='text-3xl font-bold tracking-tight'>はじめに</h1>
        <p className='text-[15px] leading-7 text-slate-600 dark:text-slate-400'>
          Library Client の画面でできることの多くは、API からも実行できます。
          このガイドは、API キーを発行してリポジトリのデータを読み書きするまでを扱います。
        </p>
        <p className='text-sm text-slate-500 dark:text-slate-500'>
          このページの API 情報は{' '}
          <code className='font-mono'>{apiBaseUrl}</code>{' '}
          から取得しています
          {api.data && (
            <>
              （{api.data.title} v{api.data.version}、
              {api.data.operationCount} エンドポイント）
            </>
          )}
          。
        </p>
        <LiveDataStatus
          loading={api.loading}
          error={api.error}
          loadingLabel='API 情報を取得しています...'
        />
      </div>

      <Section title='1. API キーを発行する'>
        <Prose>
          <p>
            <a
              href={clientUrl}
              target='_blank'
              rel='noopener noreferrer'
              className='font-medium text-sky-700 hover:underline dark:text-sky-400'
            >
              Library Client
            </a>{' '}
            でリポジトリを開き、上部のタブから <strong>API</strong>{' '}
            を選びます。開発者ポータルのクイックスタート「① API キーを作成」から発行できます。
          </p>
          <p>
            キーは発行時に一度だけ表示されます。手順の詳細と取り扱いは{' '}
            <Link
              to='/api-keys'
              className='font-medium text-sky-700 hover:underline dark:text-sky-400'
            >
              API キーの発行と失効
            </Link>{' '}
            を参照してください。
          </p>
        </Prose>
      </Section>

      <Section title='2. ベース URL を確認する'>
        <Prose>
          <p>
            このガイドが説明している API のベース URL は次のとおりです。
            以降の例ではこの値を使っています。
          </p>
        </Prose>
        <CodeBlock code={apiBaseUrl} />
        <Prose>
          <p>
            別の環境に対して使う場合は、その環境の開発者ポータルに表示されている
            ベース URL に読み替えてください。
          </p>
        </Prose>
      </Section>

      <Section title='3. 最初のリクエストを送る'>
        <Prose>
          <p>
            組織 <Code>{sampleOrg}</Code>、リポジトリ <Code>{sampleRepo}</Code>{' '}
            のデータ一覧を取得する例です。自分の組織名・リポジトリ名に置き換えてください。
          </p>
        </Prose>
        <CodeBlock code={snippets.curl} language='bash' />
        <CodeBlock code={snippets.python} language='python' />
        <CodeBlock code={snippets.javascript} language='javascript' />
      </Section>

      <Section title='認証ヘッダ'>
        <Prose>
          <p>すべてのリクエストに次のヘッダを付けます。</p>
        </Prose>
        <CodeBlock code='Authorization: Bearer pk_xxxxxxxxxxxxxxxx' />
        <Prose>
          <p>
            API キーは <Code>pk_</Code> で始まります。ログイン中のユーザーが使う
            JWT も同じヘッダで受け付けられますが、サーバー間の連携には API
            キーを使ってください。
          </p>
          <p>
            キーを付けずに送ったリクエストは匿名として扱われます。公開リポジトリなら
            読み取れることがありますが、非公開リポジトリや書き込みは拒否されます。
          </p>
        </Prose>
      </Section>

      <Section title='次に読むもの'>
        <Callout title='REST と GraphQL のどちらを使うか'>
          単一のリソースを読み書きするだけなら{' '}
          <Link to='/rest' className='font-medium underline'>
            REST
          </Link>
          が簡単です。リポジトリとその中のデータのように、関連するものをまとめて
          1 往復で取得したいなら{' '}
          <Link to='/graphql' className='font-medium underline'>
            GraphQL
          </Link>
          が向きます。API キーは両方で使えます。
        </Callout>
      </Section>
    </>
  )
}
