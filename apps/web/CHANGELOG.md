# Changelog

All notable changes to the Library application will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.9.0] - 2025-12-20

### Added
- **ガントチャートビュー**: Data ビューにガントチャートビューを追加
  - 開始日・終了日プロパティを選択して表示
  - タイムライン表示（月/週/日ヘッダー、グリッド線）
  - タスクバーのドラッグ＆ドロップで日付変更
  - ズーム機能（日/週/月/年）
  - タスクバークリックで日付編集ダイアログ
- **Date プロパティタイプ**: 日付のみ（時刻なし）を保存するプロパティタイプ
  - ISO 8601 フォーマット (`YYYY-MM-DD`)
  - DatePicker コンポーネント（shadcn/ui準拠）
  - テーブルビューでの日付表示対応
  - データ詳細ページでの DatePicker 実装

### Added (Components)
- `apps/library/src/app/v1beta/[org]/[repo]/data/components/data-gantt-view.tsx` - DataGanttView
- `apps/library/src/components/ui/date-picker.tsx` - DatePicker
- `apps/library/src/components/ui/calendar.tsx` - Calendar

### Added (Backend)
- `packages/database/domain/src/data/property_type.rs`: `PropertyType::Date` を追加
- `packages/database/domain/src/data/property_data_value.rs`: `PropertyDataValue::Date` を追加
- GraphQL API: `DateValue` 型を追加

### Changed
- `property-dialog.tsx`: DATE オプションをセレクトボックスに追加
- `viewer.tsx`: Date 型の表示ロジック追加
- `property-value/index.tsx`: Date 型の編集・表示ロジック追加
- `row.tsx`: データテーブルに Date 型の表示追加
- `data-gantt-view.tsx`: カスタム実装によるガントチャート表示（`date-fns` を使用）

### Documentation
- 仕様ドキュメント: `docs/src/services/library/gantt-chart-view.md`
- 仕様ドキュメント: `docs/src/services/library/date-property-type.md`
- タスクドキュメント: `docs/src/tasks/completed/library-v1.9.0/library-gantt-chart-view/task.md`

## [1.8.0] - 2025-12-14

### Added
- **Location 型プロパティ対応**: 緯度・経度を保存する位置情報プロパティタイプ
  - Google Maps を使用した地図表示（編集・閲覧モード）
  - **場所検索機能**: Google Places Autocomplete による場所名検索
  - **POI名表示**: Places Nearby Search / Reverse Geocoding によるピンポイントな場所名取得
    - 駅（transit_station）を優先検索（例：高田馬場駅）
    - 一般POI、住所へのフォールバック
  - **多言語対応**: アプリの言語設定に連動した場所名表示（日本語/英語）
  - データテーブルでのコンパクト表示（ホバーで地図プレビュー）
  - リポジトリのマップビュー（複数マーカー表示、フィット表示）

### Added (Components)
- `apps/library/src/app/v1beta/_components/location-map/index.tsx` - LocationMap, LocationMapCompact
- `apps/library/src/app/v1beta/_components/location-map/data-locations-map.tsx` - DataLocationsMap

### Added (Dependencies)
- `@react-google-maps/api` - Google Maps React バインディング

### Changed
- `property-dialog.tsx`: LOCATION オプションをセレクトボックスに追加
- `viewer.tsx`: Location 型の表示ロジック追加
- `property-value/index.tsx`: Location 型の編集・表示ロジック追加
- `row.tsx`: データテーブルに Location 型のコンパクト表示追加
- `repo.graphql`: DataFieldOnRepoPage に LocationValue 追加
- Storybook にサンプルデータ・インタラクションテスト追加

### Backend (library-api)
- `mutation.rs`: PropertyInput に Location 型のケース追加

### Documentation
- 仕様ドキュメント: `docs/src/services/library/location-type.md`
- タスクドキュメント: `docs/src/tasks/completed/v1.8.0/library-location-type-support/task.md`

## [1.7.0] - 2025-12-09

### Added
- **多言語対応（i18n）**: アプリケーション全体を日本語/英語の2言語に対応
  - React Context ベースの翻訳システム実装
  - `useTranslation` フックによる Client Component での翻訳取得
  - `detectLocale` / `getDictionary` による Server Component での翻訳取得
  - Cookie ベースの言語設定保存
  - `Accept-Language` ヘッダーによる自動言語検出
  - `LanguageSwitcher` コンポーネントによる言語切り替え UI

### Changed
- 認証画面（sign_in, sign_up, sign_out, forgot-password, reset-password, verify-email）の多言語対応
- ダッシュボードの多言語対応
- v1beta 配下のすべてのページの多言語対応
  - 組織ページ、リポジトリページ、設定ページ、データ詳細ページ
  - ナビゲーション、ペジネーション、フォーム
  - バリデーションメッセージ、トースト通知

### Documentation
- 仕様ドキュメント: `docs/src/services/library/i18n.md`
- タスクドキュメント: `docs/src/tasks/completed/library-v1.7.0/library-i18n-support/task.md`

## [1.6.0] - 2025-12-08

### Added
- **GitHub Markdown Import機能**: GitHubリポジトリからMarkdownファイルをLibraryにインポート
  - Orgページに「Import from GitHub」ボタンを追加
  - 4ステップウィザード形式のインポートUI（リポジトリ選択→ファイル選択→設定→完了）
  - ディレクトリの再帰的インポート対応
  - Frontmatter解析によるプロパティ自動生成
  - 値バリエーションが5以下の場合にSelect型を提案
  - `ext_github`プロパティによるGitHub連携メタデータ管理
  - `sync_to_github`フラグでfrontmatterへの出力を制御可能
  - 同名リポジトリ警告表示
  - インポート時のサンプルデータ作成スキップ

### Changed
- `markdown_composer.rs`: `sync_to_github`フラグが`false`または未設定の場合、frontmatterに`ext_github`を含めない（後方互換性対応）

### Added (Backend)
- `apps/library-api/src/usecase/list_github_directory.rs` - ディレクトリ内容取得
- `apps/library-api/src/usecase/get_markdown_previews.rs` - Markdownプレビュー取得
- `apps/library-api/src/usecase/analyze_frontmatter.rs` - フロントマター分析
- `apps/library-api/src/usecase/import_markdown_from_github.rs` - インポート実行
- `packages/providers/github/src/oauth.rs` - `list_directory_contents`, `get_raw_file_content`追加

### Added (Frontend)
- `apps/library/src/app/v1beta/[org]/_components/github-import-dialog.tsx` - インポートUI
- `apps/library/src/app/v1beta/[org]/_components/github-import-actions.ts` - Server Actions
- `apps/library/src/app/v1beta/[org]/_components/github-import-dialog.stories.tsx` - Storybook

## [1.5.1] - 2025-12-06

### Fixed
- **OGP画像URLの修正**: 本番環境でOGP画像URLが `localhost:3000` になる問題を修正
  - `getBaseUrl()` ユーティリティを追加し、リクエストヘッダーからホスト情報を取得
  - `x-forwarded-proto` ヘッダーでプロトコルを判定
  - 環境変数 `NEXT_PUBLIC_APP_URL` へのフォールバックも維持

## [1.5.0] - 2025-12-06

### Added
- **GitHub風OGP画像生成**: Organization、Repository、Dataページに動的OGP画像を生成
  - ダークテーマのGitHub風デザイン
  - Libraryロゴ、パス表示、タイトル、説明文
  - タグ、統計情報（リポジトリ数、メンバー数、データ数、コントリビューター数）
  - 公開/非公開バッジ
- **OGP画像生成API**: `/api/[org]/og`, `/api/[org]/[repo]/og`, `/api/[org]/[repo]/[dataId]/og`
- **generateMetadata**: 各ページに動的メタデータ生成を追加
  - `og:title`, `og:description`, `og:image` の設定
  - Twitter Card (`twitter:card`, `twitter:image`) の設定

### Changed
- OGP画像はEdge Runtimeで動的生成（`next/og` ImageResponse使用）

## [1.4.0] - 2025-12-04

### Added
- **GitHub Sync 機能**: ライブラリデータをFrontmatter付きMarkdownとしてGitHubへ同期
  - GitHub OAuth 連携（Settings > GitHub Integration）
  - GitHub Sync トグル（Settings > Integrations）
  - `ext_github` プロパティによるリポジトリ・パス設定
  - 一括同期機能（Bulk Sync）
  - データ更新時の自動同期
- **ext_ プレフィックス予約語化**: システム拡張プロパティとして保護
  - Add New Property で `ext_` 開始名をバリデーションエラー
- **Properties 画面の分離**: User Properties と System Extensions セクション

### Changed
- Settings > Integrations に GitHub Sync トグルを追加
- Properties 画面でシステム拡張プロパティを別セクションで表示

## [1.3.0] - 2025-11-27

### Changed
- バックエンドの Library サインイン時ポリシー自動付与に合わせてアプリケーションバージョンを同期（UI 変更なし）。

### Added
- 公開データの Markdown 取得に対応する `/v1beta/:org/:repo/data/:id/md` ルートを追加し、バックエンドの `/md` エンドポイントをフェッチするようにした（サインイン時は accessToken を付与）。
