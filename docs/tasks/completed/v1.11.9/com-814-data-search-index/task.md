# COM-814: Library連携API・検索インデックスを実装する

- Issue: https://linear.app/quantum-box/issue/COM-814
- Branch: `cfeature/library-integration-search-index-4e1a05`
- Status: Completed (2026-09-26, PR #383)
- 設計: [ADR-0011](../../../../specs/decisions/ADR-0011-data-search-index.md)

## 背景

犬と行ける場所マップは、LibraryのPlaceを地図上で範囲・距離・条件・
テキストで引く必要がある。LibraryのData検索は `name` の完全一致しかなく、
Location Propertyは `"lat,lng"` 文字列で保存されていて、SQLで範囲検索できない。

## 実装

- `GET /v1beta/repos/{org}/{repo}/data-search` を追加した。bbox、
  `lat`/`lng`/`radius_m`（距離順）、`filter`（Select・MultiSelect・Boolean・
  Integer・Date・Relation・テキスト）、`q`（NFKC＋小文字化の部分一致）、
  `ids`（詳細取得）、`sort`、ページングに対応する。
- Published制御：`publication_status` Property（Select `published` /
  Boolean `true`）を持つRepoでは、公開レコードだけを返す。
  `include_unpublished=true` は編集権限（`library:UpdateRepo`）が必要。
- 索引：Databaseごとの全件インメモリ索引。検索のたびに、database-managerの
  `DataSnapshot::revision`（件数・IDチェックサム・record_version合計・
  updated_at最大値の集約1本）とProperty定義のダイジェストで鮮度を確認し、
  変化していれば作り直す。
- HTTPキャッシュ：ETag / `If-None-Match` による304。公開Repoへの匿名アクセスは
  `Cache-Control: public, max-age=60, stale-while-revalidate=300`。

## 検証

- [x] query / index / cache / handler のunit test（範囲、距離、半径、
  フィルタ各型、公開制御、全角半角の同一視、ETag）
- [x] `database-manager` DBテスト `data_snapshot`：作成・更新・削除のたびに
  リビジョンが動き、読み取りでは動かない。他テナントからは読めない
- [x] PR #383 / CI / マージ（2026-09-26）
- [ ] 本番で犬スポットRepoを作成し、札幌圏データで地図アプリから確認（COM-813・COM-817）

## 残課題

- `data-list` / `data/{id}` は `publication_status` を見ない。下書きの秘匿が
  必要な投稿（COM-818）は非公開Repoに置く。ADR-0011 §2を参照。
- MCPツール / CLI / GraphQLからの `data-search` の利用は未対応。
