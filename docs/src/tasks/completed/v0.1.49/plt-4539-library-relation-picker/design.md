# Library Relation Property picker design

## 背景と目的

Relation Property の wire value は対象 `databaseId` と `dataIds` の集合として既に定義され、API は対象 Record が同じ tenant と対象 Database に属することを検証する。しかし Library v2 は値を件数表示するだけで、Property 作成時の対象指定も Database ID の手入力である。利用者が repository と Record の名前を基準に Relation を作成・編集できる Client workflow を追加する。

## 採用する設計

### 対象リポジトリ

- 既存 `fetchLibraryRepositories()` が返す repository のうち、Property 元と同じ tenant / organization に属するものを候補とする。
- UI は `organization / repository` 名を表示し、mutation には canonical repository ID を `relationDatabaseId` として送る。
- 編集時に既存 target が現在の一覧へ存在しなくても ID を保持し、候補取得失敗や権限変更だけで schema を別 target に書き換えない。

### 関連 Record picker

- Relation セルはボタンとして開き、対象 repository の `dataList` を既存ページサイズで取得する。
- 読み込んだ Record は名前と ID で絞り込める。続きがある場合は明示的な追加読込を提供する。
- 複数選択をローカル draft として保持し、確定時だけ既存 `updateData` mutation に `relation: [dataId...]` を送る。
- 全解除は空配列を明示的に送り、patch payload から Relation を落とさない。
- API が返した順序に依存せず、重複 ID を除去する。未取得・削除済みの選択済み ID は ID 表示で保持し、picker を開いただけで消さない。

### 表示とキャッシュ

- 読込済みの対象 Record 名と in-flight detail request は同一 target Database 内で共有する。
- 未解決時は件数を表示し、解決後は先頭の Record 名と残件数を表示する。
- target 取得失敗時は既存値を read-only 表示し、再試行できる。保存は picker が正常に読み込めた場合だけ許可する。

## API とデータモデル

GraphQL schema と DB schema は変更しない。REST fallback でも GraphQL と同じ Relation editor を使えるよう、`PropertyResponse` に optional `database_id` を追加する。Relation 以外では省略される additive な response 変更とする。その他は既存の以下を利用する。

- repository list: canonical repository ID と `orgUsername` / `username`
- `repo(...).dataList(pageSize, page)`: Relation 候補 Record
- `updateData`: `{ propertyId, value: { relation: dataIds } }`
- `RelationType.databaseId` / `RelationValue.databaseId`
- REST `PropertyResponse.database_id`: Relation target の canonical repository ID

## エラーと認可

- repository list と target data list は既存 token、platform、operator header の解決規則を使う。
- target repository が見つからない場合は locale catalog のエラーを表示して Relation を変更不可にし、既存 ID を保持する。
- 選択済み Record の detail 取得は 404 だけを削除済みとして扱い、認証・通信・server の一時失敗は picker の retry 対象にする。
- mutation error は既存の table mutation error と同じ場所へ表示し、楽観的に更新した値を server snapshot へ戻す。
- picker の loading 中や保存中は多重送信を防ぐ。

## 代替案

- **Database ID / Data ID の手入力を継続**: ID を利用者が発見できず、誤った target を作れるため不採用。
- **picker 起動時に対象 repository の全 Record を取得**: 大規模 repository で初期表示を阻害するため不採用。
- **GraphQL に部分一致検索を新設**: 理想的だが API と検索 semantics の設計を伴うため、既存ページングだけで完成できる本タスクから分離する。
- **RelationEdge を先に本番有効化**: backfill、parity、mixed-fleet drain の既存 hard gate を迂回するため不採用。

## テスト

- resolver: canonical database ID から repository を解決し、ページング結果を重複なく統合する。
- Property dialog: repository 名選択、既存 unavailable target の保持、loading / retry。
- Relation picker: current selection、検索、追加読込、複数選択、全解除、cancel、error retry。
- Table mutation: Relation の空配列を明示的に送信し、成功後の再取得で名前表示を復元する。
- Browser: fixture の target repository と Record を選び、保存・reload 後の一致を確認する。

## ADR 判定

既存 ADR-0006 の Relation と段階的 migration 方針を変更せず、既存 API 上の Client workflow を完成させる変更である。新しい長期的 architecture decision はないため ADR は追加しない。
