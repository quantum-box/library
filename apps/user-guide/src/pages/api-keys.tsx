import {
  Callout,
  Code,
  CodeBlock,
  LiveDataStatus,
  Prose,
  Section,
} from '@/components/primitives'
import { fetchSchemaSummary } from '@/lib/graphql-schema'
import { useAsync } from '@/lib/use-async'
import { sampleOrg } from '@/lib/config'

const KEY_FIELDS = ['apiKeys', 'createApiKey', 'revokeApiKey']

export function ApiKeys() {
  const schema = useAsync(fetchSchemaSummary)

  const operations = schema.data
    ? [...schema.data.queries, ...schema.data.mutations].filter(field =>
        KEY_FIELDS.includes(field.name),
      )
    : []

  return (
    <>
      <div className='space-y-3'>
        <h1 className='text-3xl font-bold tracking-tight'>
          API キーの発行と失効
        </h1>
        <p className='text-[15px] leading-7 text-slate-600 dark:text-slate-400'>
          API キーは <strong>組織 (organization) 単位</strong> で発行されます。
          組織の中のどのリポジトリを読み書きできるかは、組織に設定された権限に従います。
          リポジトリごとに別のキーが発行されるわけではありません。
        </p>
      </div>

      <Section title='発行する'>
        <Prose>
          <ol className='list-decimal space-y-1 pl-5'>
            <li>Web 画面で対象組織のリポジトリを開きます</li>
            <li>
              上部タブの <strong>API</strong> を選びます
            </li>
            <li>
              クイックスタート「① API キーを作成」の{' '}
              <strong>API キーを作成</strong> を押します
            </li>
            <li>
              キーの名前を入力します（例: <Code>ci-pipeline</Code>,{' '}
              <Code>analytics-batch</Code>）
            </li>
            <li>表示されたキーをコピーして保存します</li>
          </ol>
        </Prose>
        <Callout tone='warning' title='キーは一度しか表示されません'>
          発行直後のダイアログを閉じると、キーの値は二度と表示できません。
          控えを取り損ねた場合は、そのキーを失効させて発行し直してください。
        </Callout>
      </Section>

      <Section title='保存する'>
        <Prose>
          <p>キーはパスワードと同じ扱いをしてください。</p>
          <ul className='list-disc space-y-1 pl-5'>
            <li>
              環境変数やシークレットマネージャに置く（例:{' '}
              <Code>LIBRARY_API_KEY</Code>）
            </li>
            <li>リポジトリにコミットしない</li>
            <li>
              ブラウザで動くフロントエンドのコードに埋め込まない —
              閲覧者に見えてしまいます
            </li>
            <li>
              用途ごとに別のキーを発行する — 失効させたときの影響範囲が狭くなります
            </li>
          </ul>
        </Prose>
      </Section>

      <Section title='一覧する'>
        <Prose>
          <p>
            <strong>API</strong>{' '}
            タブの「API キー管理」に、その組織で発行済みのキーが並びます。
            表示されるのは名前・ID・作成日時で、キーの値は表示されません。
          </p>
        </Prose>
      </Section>

      <Section title='失効させる'>
        <Prose>
          <p>
            一覧の各行の <strong>失効</strong>{' '}
            を押します。確認ダイアログで実行すると、そのキーはただちに認証されなくなります。
            取り消しはできません。
          </p>
          <p>次のような場合は失効させてください。</p>
          <ul className='list-disc space-y-1 pl-5'>
            <li>キーが外部に漏れた、または漏れた可能性がある</li>
            <li>キーを使っていた連携を停止した</li>
            <li>発行直後にキーの控えを取り損ねた</li>
          </ul>
        </Prose>
        <Callout title='稼働中の連携を止めずに入れ替える'>
          先に新しいキーを発行して連携先の設定を更新し、切り替えが終わってから
          古いキーを失効させると、停止時間なく入れ替えられます。
        </Callout>
      </Section>

      <Section title='API から操作する'>
        <Prose>
          <p>
            画面を使わずに発行・一覧・失効することもできます。以下はこの API
            が現在公開している、キーに関する GraphQL の操作です。
          </p>
        </Prose>
        <LiveDataStatus
          loading={schema.loading}
          error={schema.error}
          loadingLabel='スキーマを取得しています...'
        />
        {operations.length > 0 && (
          <ul className='divide-y divide-slate-200 overflow-hidden rounded-lg border border-slate-200 dark:divide-slate-800 dark:border-slate-800'>
            {operations.map(field => (
              <li key={field.name} className='px-4 py-3'>
                <code className='font-mono text-[13px] text-slate-800 dark:text-slate-200'>
                  <span className='font-semibold'>{field.name}</span>
                  {field.args.length > 0 && `(${field.args.join(', ')})`}
                  <span className='text-slate-500'>: {field.type}</span>
                </code>
                {field.description && (
                  <p className='mt-1 text-sm text-slate-600 dark:text-slate-400'>
                    {field.description}
                  </p>
                )}
              </li>
            ))}
          </ul>
        )}
        <CodeBlock
          language='graphql'
          code={`mutation {
  createApiKey(input: { organizationUsername: "${sampleOrg}", name: "ci-pipeline" }) {
    apiKey {
      id
      name
      value
    }
  }
}`}
        />
        <Prose>
          <p>
            <Code>value</Code> がキーの本体です。このレスポンスでしか取得できません。
          </p>
        </Prose>
      </Section>

      <Section title='権限'>
        <Prose>
          <p>キーを発行・一覧・失効するには、組織に対する権限が必要です。</p>
        </Prose>
        <div className='overflow-hidden rounded-lg border border-slate-200 dark:border-slate-800'>
          <table className='w-full text-sm'>
            <thead className='bg-slate-50 text-left dark:bg-slate-900'>
              <tr>
                <th className='px-4 py-2 font-medium'>操作</th>
                <th className='px-4 py-2 font-medium'>必要な権限</th>
              </tr>
            </thead>
            <tbody className='divide-y divide-slate-200 dark:divide-slate-800'>
              {[
                ['発行', 'library:CreateApiKey'],
                ['一覧', 'library:ListApiKeys'],
                ['失効', 'library:RevokeApiKey'],
              ].map(([label, action]) => (
                <tr key={action}>
                  <td className='px-4 py-2'>{label}</td>
                  <td className='px-4 py-2'>
                    <Code>{action}</Code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <Prose>
          <p>
            組織のオーナーであれば、いずれの操作も行えます。
            操作が「権限がありません」になる場合は、組織の管理者に権限の付与を
            依頼してください。
          </p>
        </Prose>
      </Section>

      <Section title='サービスアカウントとの関係'>
        <Prose>
          <p>
            発行されたキーは、組織の <Code>default</Code>{' '}
            サービスアカウントに紐づきます。キーで送ったリクエストは、そのサービスアカウント
            として実行されます。ユーザーとして実行されるわけではないため、監査ログ上も
            サービスアカウントとして記録されます。
          </p>
        </Prose>
      </Section>
    </>
  )
}
