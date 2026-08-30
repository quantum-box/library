import { appKitConfig } from '../app/kitConfig'
import type { MessageKey } from '../i18n'

export type Priority = 'urgent' | 'high' | 'medium' | 'low' | 'none'
export type Status = 'backlog' | 'todo' | 'in_progress' | 'in_review' | 'done' | 'cancelled'

export interface DatabaseRecord {
  id: string
  identifier: string
  title: string
  status: Status
  priority: Priority
  assignee: string | null
  labels: string[]
  project: string
  createdAt: string
  updatedAt: string
  description: string
  orgUsername?: string
  repoUsername?: string
  operatorId?: string
}

export const mockUsers = appKitConfig.workspace.users
const projects = appKitConfig.workspace.projects.map((project) => project.label)
const labelSets = [
  ['bug'], ['feature'], ['improvement'], ['bug', 'critical'],
  ['feature', 'ux'], ['infra'], ['docs'], ['performance'],
]

const titles: Record<Status, string[]> = {
  backlog: [
    'ダッシュボードのレスポンシブ対応',
    'E2Eテストの追加',
    'ログ出力フォーマットの統一',
    'API Rate Limitの実装',
    'WebSocket接続のリトライ機能',
  ],
  todo: [
    'ユーザー検索APIの実装',
    'テーブルビューのフィルタ機能',
    'メール通知テンプレートの作成',
    'バッチ処理のエラーハンドリング改善',
    'OpenAPI定義の更新',
  ],
  in_progress: [
    'カンバンビューのドラッグ&ドロップ実装',
    '認証フローのリファクタリング',
    'パフォーマンスモニタリングの導入',
    'GraphQLスキーマの設計',
  ],
  in_review: [
    'CI/CDパイプラインの最適化',
    'データベースマイグレーションスクリプト',
    'アクセス制御の権限モデル実装',
  ],
  done: [
    'プロジェクト設定画面の実装',
    'Slack連携の通知機能',
    'CSVエクスポート機能',
    'ダークモード対応',
    '多言語対応(i18n)の基盤構築',
    'パスワードリセットフロー',
  ],
  cancelled: [
    'レガシーAPI v1の廃止',
    '旧UIコンポーネントの削除',
  ],
}

const priorities: Priority[] = ['urgent', 'high', 'medium', 'low', 'none']

const extraTitles = [
  'キャッシュ戦略の見直し', 'エラー画面のUX改善', 'ヘルスチェックAPIの追加',
  'バックアップ自動化', 'セッション管理の改善', 'ページネーションの最適化',
  'ファイルアップロード機能', 'Webhook配信リトライ', 'メトリクス収集基盤',
  '通知設定画面', 'データエクスポート改善', 'API v2設計', 'テスト基盤の刷新',
  'ロギング基盤の刷新', 'マルチテナント対応', 'SSO連携', '監査ログ実装',
  '検索インデックス最適化', 'レート制限の高度化', 'データ整合性チェック',
]

let counter = 1
const statuses: Status[] = ['backlog', 'todo', 'in_progress', 'in_review', 'done', 'cancelled']

function generateRecords(): DatabaseRecord[] {
  const records: DatabaseRecord[] = []
  // Original titles by status
  for (const [status, titleList] of Object.entries(titles)) {
    for (const title of titleList) {
      records.push(makeRecord(title, status as Status))
    }
  }
  // Extra records for virtual scroll testing (200+ total)
  for (let i = 0; i < 180; i++) {
    const title = `${extraTitles[i % extraTitles.length]} #${Math.floor(i / extraTitles.length) + 2}`
    const status = statuses[i % statuses.length]
    records.push(makeRecord(title, status))
  }
  return records
}

function makeRecord(title: string, status: Status): DatabaseRecord {
  const id = `record-${counter}`
  const identifier = `${appKitConfig.records.identifierPrefix}-${100 + counter}`
  const record: DatabaseRecord = {
    id,
    identifier,
    title,
    status,
    priority: priorities[counter % priorities.length],
    assignee: counter % 3 === 0 ? null : mockUsers[counter % mockUsers.length],
    labels: labelSets[counter % labelSets.length],
    project: projects[counter % projects.length],
    createdAt: new Date(2026, 2, (counter % 28) + 1).toISOString(),
    updatedAt: new Date(2026, 2, 20 + (counter % 10)).toISOString(),
    description: `${title}の詳細な説明がここに入ります。\n\n## 要件\n- 要件1\n- 要件2\n- 要件3\n\n## 関連\n- ${identifier}`,
  }
  counter++
  return record
}

export const mockDatabaseRecords: DatabaseRecord[] = generateRecords()

/**
 * Status and priority presentation. `labelKey` is resolved through `t()` at
 * render time so a language switch relabels every board column, select, and
 * badge without rebuilding the record set.
 */
export const statusConfig: Record<
  Status,
  { labelKey: MessageKey; color: string; icon: string }
> = {
  backlog: { labelKey: 'status.backlog', color: 'var(--text-muted)', icon: '○' },
  todo: { labelKey: 'status.todo', color: 'var(--status-todo)', icon: '◎' },
  in_progress: { labelKey: 'status.inProgress', color: 'var(--status-progress)', icon: '◑' },
  in_review: { labelKey: 'status.inReview', color: 'var(--accent)', icon: '◕' },
  done: { labelKey: 'status.done', color: 'var(--status-done)', icon: '●' },
  cancelled: { labelKey: 'status.cancelled', color: 'var(--status-cancelled)', icon: '⊘' },
}

export const priorityConfig: Record<
  Priority,
  { labelKey: MessageKey; color: string; icon: string }
> = {
  urgent: { labelKey: 'priority.urgent', color: 'var(--priority-urgent)', icon: '⚡' },
  high: { labelKey: 'priority.high', color: 'var(--priority-high)', icon: '▲' },
  medium: { labelKey: 'priority.medium', color: 'var(--priority-medium)', icon: '■' },
  low: { labelKey: 'priority.low', color: 'var(--priority-low)', icon: '▽' },
  none: { labelKey: 'priority.none', color: 'var(--text-muted)', icon: '─' },
}
