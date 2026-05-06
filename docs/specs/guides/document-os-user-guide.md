# ドキュメントOSとしての利用ガイド（Document OS）

対象: `library` の公開ドキュメント基盤としての使い方。
最終更新: 2026-05-03

## 1. 何を実現するか

1. `Repo` を論理的な「ドキュメント空間」として扱い、`Data` を1ページ分（記事・仕様書・ナレッジ）として公開する。
2. `/docs` 系エンドポイントを、検索・リンク配信・埋め込み素材のソースにする。
3. 外部編集は REST/GraphQL を通して行い、最終表示は公開UI or API（Markdown）で配信する。

## 2. 利用者視点の基本フロー

1. 組織・リポジトリを作成し、公開対象を `repo.is_public=true` にする。
2. ドキュメントを `Data` として登録する。
3. ページを公開URLで検証する。
4. `page` / `page_size` で一覧ページングを確認する。

### 2.1 公開URL

1. 一覧: `GET /docs/{org}/{repo}`
2. ページ: `GET /docs/{org}/{repo}/{data_id}`
3. 本文抽出: `GET /docs/{org}/{repo}/{data_id}/md`

公開 repo は認証なしで閲覧できる。private repo は同じ URL でも通常の Library 権限を持つ認証済み executor のみ閲覧できる。

## 3. ドキュメント公開品質の運用

1. ページ本体は `markdown` を基準にし、本文差分チェックは `/md` API で行う。
2. 一覧導線は `/docs/{org}/{repo}` で URL 構造が崩れていないか確認。
3. 非公開変更（`is_public=false`）のまま匿名で公開URLを叩いて、403 になることを確認すると誤公開防止に有効。
4. URL共有時は `data_id` ベースかつ slug 的名称の整合を事前に決める。
5. Canonical URL は `data_id` を使う。`slug` property は検索、frontmatter、CMS運用上の人間可読キーとして扱い、route 解決には使わない。

### 3.1 Markdown / frontmatter 仕様

1. `/md` は `text/markdown; charset=utf-8` として YAML frontmatter 付き Markdown を返す。
2. frontmatter には `id` と `title` を必ず含める。
3. `content` / `body` / `markdown` / `html` のような本文 property は body として扱い、frontmatter には重複出力しない。
4. 本文以外の property は frontmatter に出力する。`slug` を運用する場合もここに含める。
5. HTML ページは同じ Markdown から frontmatter を除いた本文を rendering する。

## 4. 外部連携（Document OS運用）

1. 外部 CMS からの取り込みや移行時は `AddData` / `UpdateData` でバッチ投入し、最終的に `/md` で整合を取る。
2. 検索エンジン向けは `Markdown` 取得の方が正規化しやすい。
3. GitHub 等との連携は `GraphQL` の import 機能（`import_markdown_from_github`）を利用するルートもあるため、差分ルールを明文化しておく。

## 5. 監査・更新フロー

1. 更新ごとに「更新者」「更新対象 data_id」「公開パス」を記録する。
2. バージョン比較は GraphQL/RESTの取得結果と `/md` 差分を組み合わせて実施。
3. 大規模更新時は、先に `data-list` で件数を採取しておき、更新後に同件数を再確認する。

## 6. 読者向け導線

1. 開発者は `GET /docs/...` だけで公開利用を開始できる。
2. 執筆者・編集者は `docs/specs/guides/cms-user-guide.md` でコンテンツ定義を行う。
3. 運用監視は `docs/specs/integrations/operations.md` を参照し、イベント系は `webhooks.md` でハンドリングルールを固定する。
