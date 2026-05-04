# CMS運用ガイド（Library）

対象: ライターや編集者が `library` を CMS（構造化データ管理）として使うための実務手順。
最終更新: 2026-05-03

## 1. 何が CMS として使えるか

1. `Repo` をコンテンツ領域（記事一覧、ドキュメント集合、データベース）として扱う。
2. `Property` をフィールド定義として設計し、`Data` を1件ずつのコンテンツエントリとして保存する。
3. `markdown` / `string` / `select` / `multi_select` / `relation` / `location` / `image` などで入力仕様を固定できる（`html` は将来互換含めて注意）。
4. 公開用の読み取りは `/docs/{org}/{repo}` 系で提供する。

## 2. CMS 利用の最短フロー

1. 組織を作る (`POST /v1beta/orgs`)
2. 公開対象のリポジトリを作る (`POST /v1beta/repos/{org}`)
3. コンテンツモデルを作る (`POST /v1beta/repos/{org}/{repo}/properties`)
4. 記事を登録 (`POST /v1beta/repos/{org}/{repo}/data`)
5. 既存の表示確認 (`/docs/{org}/{repo}` / `/docs/{org}/{repo}/{data_id}`)

## 3. コンテンツ設計パターン

### 3.1 ブログ記事

1. `title`（string）
2. `slug`（string）
3. `published_at`（date）
4. `category`（select）
5. `tags`（multi_select）
6. `body`（markdown）

### 3.2 仕様

1. 画面側（CMS管理UI）では、`Data.name` がタイトル、`property_data` が本文/分類として扱われる想定。
2. 変更が頻繁な項目は `select` / `multi_select` で正規化し、`tag` の文字ゆらぎを避ける。
3. 外部連携で画像URLを持つ場合は `image`/`string` を使い、公開時は `docs/{org}/{repo}/{data_id}` で確認。

## 4. 運用の実務ルール（管理者向け）

1. 一意キーは `slug` などで別途運用し、`GET /docs/{org}/{repo}` で重複を防ぐ。
2. フィールド追加は後方互換が必要。既存Dataの property_id 変更は避ける。
3. 不要な `property_type` は変更しにくいので、最初の設計で型を固定する。
4. `is_public` false の場合はプレビューが見えないため、公開前検証は内部環境ルートを使う運用にする。

## 5. CMS担当向け API 参照（最小セット）

1. コンテンツ一覧: `GET /v1beta/repos/{org}/{repo}/data` または `data-list`
2. コンテンツ詳細: `GET /v1beta/repos/{org}/{repo}/data/{data_id}`
3. コンテンツ作成: `POST /v1beta/repos/{org}/{repo}/data`
4. 更新: `PUT /v1beta/repos/{org}/{repo}/data/{data_id}`
5. 削除: `DELETE /v1beta/repos/{org}/{repo}/data/{data_id}`
6. 公開閲覧: `GET /docs/{org}/{repo}` / `/docs/{org}/{repo}/{data_id}`

### 5.1 更新・公開前チェック

1. `GET /v1beta/repos/{org}/{repo}/data/{data_id}/md` で本文のみを取り出し、検索エンジン投入や差分比較がしやすい。
2. `GET /v1beta/repos/{org}/{repo}/data/{data_id}` で JSON 構造ごとレビューし、必須 property 欠落がないか確認する。

## 6. よくある運用例

1. 公開前は `is_public` を true/false に分けて公開状況管理し、切替だけで公開停止を実施。
2. 大規模編集時は、`webhook` 連携を先に停止して手動投入に切り替え、完了後に再開する。
3. 同期が入る組織では、REST投入とGraphQL更新を同時実行せず、どちらかに集約する。
