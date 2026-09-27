# COM-860 共通食材マスタ: スキーマ・検索・版付き参照

- Status: in-progress
- Branch: `feature/com-860`
- Linear: COM-860（親 COM-859 / Milestone M1｜主力メニューの原価・栄養を根拠付きで出す）
- 格納先: Library org `library`（2026-09-25 決定）。食材は `library/food`、成分定義と成分値は別repo

## 背景

バナスク経営管理（Food Loss Management）で、店舗をまたいで使う食材の共通情報
（食品番号、名称、状態、成分値、出典・版）をLibraryに置き、Fieldの仕入品・
レシピ・栄養試算から「食材キー＋公開版」で参照する。店舗の仕入単価や非公開
レシピは持たない。

コードを確認した結果（Library `5d979600`）、既存のData/Propertyだけでは次の
4点が満たせなかった:

- 組込のDecimal型がない（Integerは`i32`）。Propertyは`value0..value50`の51枠。
- `search_by_name`は名前の完全一致だけ。別名・状態・分類での検索がない。
- `record_version`は楽観ロック用のカウンタで、旧版の内容を返す経路がない。
- Dataのsnapshot／リリースの仕組みがない。

## 方針（決定）

**編集はLibraryのrepo（既存のData CRUD／upsert）で行い、公開時に固定版を専用テーブルへ書き出す。**

```
org `library`
  ├─ 食材 repo        (Data: 1食材 = 1レコード)
  ├─ 成分定義 repo    (Data: 1成分項目 = 1レコード)
  └─ 成分値 repo      (Data: 食材×成分 = 1レコード)
        │ POST .../releases （全件を検証し、1トランザクションで固定）
        ▼
library DB: ingredient_releases + items / aliases / nutrients / values
        │ 読み取りのみ
        ▼
Field: ingredient_key + release_id で参照
```

- 3つのrepoを`ingredient_catalogs`（org単位の`catalog_key`）で束ねる。
- 成分は列ではなくレコードで持つので、51枠を超える成分項目も表せる。
- 小数はfloatを通さず、正規化した10進文字列（`012.50`→`12.5`）で持つ。
  Field側でDecimalに変換する。
- 値の状態（`value_status`）を必須にし、`Tr`・`-`・`0`を数値に潰さない。

### 下書き repo のプロパティ

プロパティ名で読む。型はString（`display_order`はIntegerも可、`default_display`はBooleanも可）。

| repo | Data name | プロパティ |
| -- | -- | -- |
| 食材 | 原典の食品名 | `ingredient_key`*, `source_food_code`*, `standard_name`, `reading`, `aliases`（1行1件）, `category_code`, `category_name`, `part`, `cooking_state`（`[a-z_]`、例 `raw`/`boiled`）, `skin_bone`, `refuse_rate`（0–100の10進）, `attribute_review_status`（`unreviewed`/`reviewed`） |
| 成分定義 | 表示名 | `nutrient_key`*（例 `ENERC_KCAL`, `PROT-`）, `unit`*, `basis`*（例 可食部100g当たり）, `method`, `display_order`, `default_display` |
| 成分値 | 任意 | `ingredient_key`*, `nutrient_key`*, `value_status`*, `amount`, `raw_notation` |

`*`は必須。キーは`[A-Za-z0-9._-]{1,64}`。`source_food_code`は先頭ゼロを保つため文字列。

### value_status

| status | 原典表記 | amount |
| -- | -- | -- |
| `measured` | `12.3` | 必須 |
| `estimated` | `(12.3)` | 必須 |
| `zero` | `0` | `0` |
| `estimated_zero` | `(0)` | `0` |
| `trace` | `Tr` | なし |
| `estimated_trace` | `(Tr)` | なし |
| `not_measured` | `-` | なし |
| `not_listed` | （値レコードなし） | なし。読み取り時のみ返す |

`NutrientValueStatus::from_notation`で原典表記から変換できる（COM-861の取込で使う）。

### 公開版の不変性

- リリース系テーブルはINSERTのみ。アプリにUPDATE/DELETEの経路がない。
- 公開後に下書きrepoを編集・削除しても、既存リリースの値は変わらない（読み取りは固定テーブルだけを見る）。
- `content_hash`（`sha256:`、固定行の正規JSON）を保持。`ReleaseSnapshot::compute_hash`で再計算して照合できる。
- 同じ`source_id`＋`source_release`での再公開は、内容が同じなら既存を返し（安全な再試行）、違えば409。
- `record_version`は公開版として使わない。

### 権限

- catalog登録・公開: `library:UpdateRepo`（下書きrepoの編集と同じ）。新しいactionは追加しない。
- 読み取り: 3つの下書きrepoがすべてpublicなら匿名で読める。privateなrepoがあれば、既存のrepo読み取り権限（`library:ViewRepo`／`library:ViewPrivateRepo`）が要る。
- 店舗利用者（Field）は読み取りだけ。共有マスタへ書き戻す経路はない。

## API

| Method | Path | 内容 |
| -- | -- | -- |
| POST | `/v1beta/orgs/{org}/ingredient-catalogs` | catalog登録（3つのrepo username） |
| POST | `/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases` | 下書きを検証して公開 |
| GET | `/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases` | リリース一覧（新しい順） |
| GET | `.../releases/{release_id}` | リリースと成分定義 |
| GET | `.../releases/{release_id}/ingredients?q=&category_code=&cooking_state=&page=&page_size=` | 名称・別名・読みの部分一致検索、ページング |
| GET | `.../releases/{release_id}/ingredients/{ingredient_key}` | 食材と全成分の値（値なしは`not_listed`） |

別名で一致しても状態は自動で選ばない（`玉ねぎ`で生・ゆでの両方が返る）。

## 完了条件との対応

- [x] 51枠を超える成分を列を増やさずに表せる（値をレコードで持つ）
- [x] 0.1・0・微量・未測定をAPIの往復で区別できる（`value_status`＋10進文字列）
- [x] 名前で一致しない別名での検索、状態での絞り込み、2ページ目以降
- [x] 公開後に下書きを編集・削除しても旧リリースの値が変わらない
- [x] 同名に近い食材を部位・状態で区別し、同じIDと版で同じ値を得られる
- [x] 共通マスタの編集者と店舗利用者の権限を分ける
- [ ] DB結合テスト（MySQL）: 公開→下書き変更→旧リリース不変、201件目以降の検索
- [ ] org `library` に成分定義・成分値のrepoとcatalogを作成（食材repo `library/food` は作成済み）

## 対象外・後続

- 公式Excel・正誤表の取込: COM-861（`from_notation`を使う）
- Field側の検索adapter・紐付け: COM-832
- 公開処理は下書きを1ページ100件ずつ読む。全件（約2,500食品×約50成分）での所要時間は
  COM-861の取込後に計測する。長ければ非同期ジョブ化かdatabase-managerへの一括読み取り追加を検討する。
- 公開中に下書きが編集されると不整合になりうる。公開は編集を止めてから行う運用にする。
