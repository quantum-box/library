# ADR-0011: 位置・条件・テキスト検索はLibrary内のインメモリ索引で提供する

## Status

Proposed (2026-09-26)

COM-814（犬と行ける場所マップ）。`GET /v1beta/repos/{org}/{repo}/data-search` と
その索引をこのADRと同じPRで実装する。

## Context

犬と行ける場所マップは、LibraryのPlaceレコードを地図・リストに表示する。
アプリが必要とする取得方法は次の通り。

- 地図の表示範囲（bbox）に入るPlace
- 現在地から近い順のPlace、指定半径内のPlace
- category / brand / 犬同伴条件（大型犬可、体重上限など）での絞り込み
- テキスト検索
- Published のレコードだけを一般利用者へ返す
- スポット詳細
- ページング・キャッシュ

実装前のLibraryには、これらを満たす仕組みがなかった。

- Data検索は `name` の完全一致とoffsetページングだけ
  （`SELECT … WHERE name = ?`）。
- Location Propertyは `value0..value50` のLONGTEXT列に `"lat,lng"` の
  文字列として入る。SQLで範囲検索や距離計算をするには、値の射影列と索引が
  別途要る。
- `index_definitions` テーブルはあるが、制御面だけで射影テーブルはない。
- Dataに Published / Draft の区別がない。公開範囲はRepo単位の `is_public`
  だけ。

「データ量が増えたとき、検索索引をLibrary内部に持つか、アプリ側に別の検索基盤を
持つか」を決める必要がある。

## Decision（提案）

**検索はLibraryの汎用APIとして提供する。索引は当面、library-apiプロセス内で
Databaseごとに全件を保持するインメモリ索引とし、Databaseのリビジョンが
変わったときに作り直す。アプリ側に検索基盤は持たない。**

### 1. API

`GET /v1beta/repos/{org}/{repo}/data-search`。犬スポット専用ではなく、
任意のRepoに対して使える。

| Query | 意味 |
| --- | --- |
| `bbox=minLng,minLat,maxLng,maxLat` | 範囲内のレコード。順序はGeoJSON／地図SDKのviewportと同じ |
| `lat`, `lng`, `radius_m` | 地点からの距離を返し、既定で近い順に並べる。`radius_m` で半径を絞る |
| `location_property` | 位置に使うLocation Propertyのkey。省略時は最初のLocation Property |
| `filter=key:value` | 繰り返し可能で、すべてAND。`key:a\|b` はOR。`key>=n` / `key<=n` はInteger・Dateの範囲指定 |
| `q` | 空白区切りの全語を含むレコード。name、テキスト値、選択肢のkey・表示名が対象。NFKCと小文字化で全角半角・大小文字を同一視する |
| `ids` | レコードID指定（最大100）。スポット詳細を一覧と同じ公開ルールで取得する |
| `sort` | `distance` / `name` / `updated` |
| `include_unpublished` | 下書きも含める。Repoの編集権限（`library:UpdateRepo`）が必要 |
| `page`, `page_size`, `include_body` | 既存の一覧APIと同じ |

応答は `data-list` と同じ `DataResponse` に `distanceMeters` を加えた形。

フィルタで選択肢を指定するときは、選択肢ID・key・表示名のどれでもよい。
存在しない選択肢はエラーにせず、何にも一致しない扱いにする。選択肢を廃止しても、
古いクライアントの画面がエラーにならないようにするため。Booleanは値なしを
`false` と扱う。未チェックのチェックボックスは保存されないことが多いため。

### 2. Publishedの表現

Databaseに `publication_status` という key の Property を定義したRepoだけが、
公開制御の対象になる。

- Selectの場合、選択肢keyが `published` のレコードだけが公開。
- Booleanの場合、`true` のレコードだけが公開。
- それ以外の型の場合、読み取れない公開設定から下書きを漏らさないよう、
  全件を非公開扱いにする（fail closed）。
- このPropertyを持たないRepoには下書きがないので、全件を公開扱いにする。

`data-search` は、`include_unpublished` を付けない限り、編集者に対しても
公開レコードだけを返す。こうすると同じクエリの応答が利用者によって変わらず、
共有キャッシュに載せられる。

**制約**：公開Repoの `data-list` や `data/{id}` は、今回の変更後も
`publication_status` を見ない。下書きを第三者に見せてはならない運用
（COM-818のユーザー投稿など）では、未確認の投稿を非公開Repoに置き、確認後に
公開Repoへ登録する。Repo単位の可視性が、現状唯一の強制境界である。
既存の読み取りAPIにも公開制御を広げるかは、COM-818で決める。

### 3. 索引と更新の反映

- 検索のたびに、Databaseのリビジョンを集約クエリ1本で読む
  （`COUNT(*)`、`BIT_XOR(CRC32(id))`、`SUM(record_version)`、`MAX(updated_at)`）。
  レコードの値列は読まない。
- リビジョンとProperty定義のダイジェストの組を指紋とする。指紋が
  キャッシュと一致すれば索引を再利用し、違えば全件を読み直して作り直す。
- Libraryへの書き込みは、どのインスタンスでも次の検索から反映される。
  無効化メッセージを配る必要はない。Lambdaの各インスタンスが独立に持つ
  キャッシュでも正しく動く。
- プロセスあたり最大32 Databaseを保持し、LRUで追い出す。

各指紋要素で検知する変更は次の通り。

| 要素 | 検知する変更 |
| --- | --- |
| 件数 | 作成・削除 |
| IDチェックサム | 件数が同じになる削除と作成の組 |
| record_version合計 | パッチによる値の更新 |
| updated_at最大値 | パッチを経由しない値の書き換え |
| Property定義 | 選択肢の追加・keyの変更など、レコードは変わらず一致結果が変わる変更 |

### 4. HTTPキャッシュ

- 応答には `ETag`（指紋、公開範囲、正規化したクエリのハッシュ）を付ける。
  `If-None-Match` が一致すれば304を返す。
- 公開Repoへの匿名アクセスには
  `Cache-Control: public, max-age=60, stale-while-revalidate=300` を付ける。
  CDNやブラウザでの反映遅れは最大60秒。
- 認証付き、非公開Repo、`include_unpublished` の応答は
  `private, no-cache`（ETagによる再検証のみ）とする。

### 5. アプリ側に検索基盤を持たない理由

- Source of TruthはLibraryである。アプリ側の索引は同期遅延・同期漏れの
  扱いを増やすだけで、MVPの規模では速度面の利点がない。
- 北海道全域の犬同伴スポットは多く見積もっても数万件で、線形走査で十分に速い。
  5,000件程度のDatabaseを走査する1回の検索は1ms未満。
- 地理・条件・テキスト検索は、ほかのRepo（店舗、イベントなど）でも使える
  汎用機能である。

### 6. 次の段階へ移る条件

以下のどれかに当てはまったら、`index_definitions` を実装し、射影テーブル方式に
移行する。射影テーブルは、緯度・経度・フィルタ対象の値を型付き列で持つ。
移行は `domain_outbox_events` の新しいconsumerとして行い、アプリ側APIの契約は
変えない。

- 1 Databaseのレコードが5万件を超える、または再構築に1秒以上かかる。
- 書き込み頻度が高く、キャッシュがほぼ当たらない（再構築が検索回数の
  1割を超える）。
- 形態素解析・スコアリングなど、部分一致を超えるテキスト検索が必要になる。
  この場合は外部の全文検索エンジンへの射影もあわせて比較する。

## Consequences

- 犬スポットを含め、Location Propertyを持つRepoは、スキーマ変更なしで
  地図検索に対応できる。
- 検索1回ごとに、Property一覧の取得とリビジョン集約の2クエリがかかる。
  索引の再構築はDatabase全件の読み込みになる。
- `data-search` 以外の読み取りAPIは、Published制御の対象外のままである。
  §2の制約を運用で守る。
- PetPolicyを別Databaseに分けた場合、Relation先の値では絞り込めない。
  マップで絞り込む条件は、Placeレコード自身のPropertyとして持つ
  （COM-813のモデル設計で考慮する）。
