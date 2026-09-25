# COM-861 食品成分表の公式Excel・正誤表をLibraryへ検証付きで取り込む

- Status: in-progress
- Branch: `feature/com-861`（`feature/com-860` の上に積む）
- Linear: COM-861（親 COM-859 / 前提 COM-860）
- 置き場所: `library food import`（`apps/cli/src/commands/food/`）＋共有crate `packages/ingredient_notation`

## 結論

公式Excel（`表全体` シート）と2026-03-27付正誤表を読み、COM-860の下書きrepo
（食材・成分定義・成分値）へ既存の upsert API だけで書く CLI を追加した。
既定は dry run で、書く前に件数・特殊値・無視した列・正誤表の結果・repoとの差分
（追加/変更/削除候補）を出す。公開（release）は行わない。

2026-09-25 時点の公式Excelは正誤表を反映済みで再公開されていた。正誤表111件の
うち110件は「適用済み」と判定され、二重適用はない。残る1件（11316 備考）は
Excel と正誤表の「正」が一字違う（`油` と `脂`）ため conflict として報告し、
Excelの表記のまま取り込む。

## 観測した原典（取得 2026-09-25T08:36:35Z）

| 役割 | URL | SHA-256 | サイズ | Last-Modified |
| -- | -- | -- | -- | -- |
| 本表（第2章データ） | https://www.mext.go.jp/content/20260327-mxt_kagsei-mext-000029402_02.xlsx | `0d5a77077dd6cd91cbc2e6e317b8b218a38728c409eed452f1c10635a0d3099c` | 1,973,402 B | Fri, 27 Mar 2026 01:00:21 GMT |
| 正誤表（データ） | https://www.mext.go.jp/content/20260327-mxt_kagsei-mext-000029402_16.xlsx | `fb61037c7f66af0db1fb0729977913a629bc17097bc3ff7bf75217f9110acabb` | 68,693 B | Fri, 27 Mar 2026 01:00:20 GMT |

- 掲載ページ: https://www.mext.go.jp/a_menu/syokuhinseibun/mext_00001.html
  （利用・出典表記: https://www.mext.go.jp/a_menu/syokuhinseibun/ 、特殊表記: https://fooddb.mext.go.jp/help.html ）
- 原典ファイルはコミットしない。検証用に `.food-import/`（gitignore済み）等へ置く。

### 本表 `20260327-…_02.xlsx`

- シート: `表全体`（全食品）＋食品群別18シート（`1穀類` … `18調理済み流通食品類`）。取込は `表全体` のみ、食品群名は群別シート名から得る。
- 行（1始まり）: 1行目 `更新日：2026年3月27日`、ヘッダ2–12行、11行目 `単位`、12行目 `成分識別子`、13行目からデータ。食品は2,538行（13–2550行）、末尾に空行1行。
- 列: A 食品群、B 食品番号（文字列 `01001`）、C 索引番号、D 食品名、E 廃棄率（`REFUSE`, %）、F–BI 成分、BJ 備考。
- 採用した成分列（成分識別子＝`nutrient_key`、53列）:
  `ENERC, ENERC_KCAL, WATER, PROTCAA, PROT-, FATNLEA, CHOLE, FAT-, CHOAVLM, CHOAVL, CHOAVLDF-, FIB-, POLYL, CHOCDF-, OA, ASH, NA, K, CA, MG, P, FE, ZN, CU, MN, ID, SE, CR, MO, RETOL, CARTA, CARTB, CRYPXB, CARTBEQ, VITA_RAE, VITD, TOCPHA, TOCPHB, TOCPHG, TOCPHD, VITK, THIA, RIBF, NIA, NE, VITB6A, VITB12, FOL, PANTAC, BIOT, VITC, ALC, NACL_EQ`
  （Excel上は `VITK ` と末尾空白あり → trim）
- 採用しない列: O・R（`CHOAVLM`/`CHOAVLDF-` の右の `*`。エネルギー計算に使った利用可能炭水化物の印。799/1738セル）、AG（空の区切り列）。
- 値の表記（全134,514セル）: 数値 90,428、`(数値)` 21,415、`-` 18,650、`Tr` 3,081、`(Tr)` 934、`数値†` 3（03032の規定法による測定値。†を外して読み、`raw_notation` に残す）、ヨウ素列の `*` 3（06371・13051・17137、「第3章参照」）。
  → value_status: measured 75,909 / estimated 13,075 / zero 14,522 / estimated_zero 8,340 / trace 3,081 / estimated_trace 934 / not_measured 18,650。
- 重複食品番号なし。先頭ゼロ落ちなし。

### 正誤表 `20260327-…_16.xlsx`

- シート: `本表第1章`, `本表第2章`, `本表`, `ア第2章`, `ア第1表`, `ア第4表`, `脂第2章`, `炭第2章`, `炭本表`, `炭別表1`。日付セル `令和8年3月27日`。
- `本表第2章`（4行目ヘッダ: 変更対象・頁・食品番号・索引番号・食品名等・項目等・誤・正・備考、5–71行の67件）: 項目等は日本語名（`食物繊維総量`, `エネルギー　kJ`, `…アスタリスク` 等）。表ヘッダの名前から成分識別子に引き当てる。`各成分` は `本表` シート参照。
- `本表`（表と同じ列配置を1列右にずらし、`誤`/`正` の行ペア）: 04046, 11310, 17036, 17042, 17043 の5食品。差のある列を1件ずつ展開（44件）。
- それ以外のシートは本表以外（第1章本文、アミノ酸・脂肪酸・炭水化物成分表編）のため対象外として件数だけ報告（第1章4件、ア第2章12件、脂第2章8件、炭第2章20件）。

## 使い方

```bash
# 1. 原典を取得（コミットしない）
mkdir -p .food-import/src && cd .food-import/src
curl -fsSLO https://www.mext.go.jp/content/20260327-mxt_kagsei-mext-000029402_02.xlsx
curl -fsSLO https://www.mext.go.jp/content/20260327-mxt_kagsei-mext-000029402_16.xlsx
cd ../..

# 2. ファイルだけ検証（認証不要・repoを読まない）
library food import --offline \
  --table .food-import/src/20260327-mxt_kagsei-mext-000029402_02.xlsx \
  --errata .food-import/src/20260327-mxt_kagsei-mext-000029402_16.xlsx \
  --retrieved-at 2026-09-25T08:36:35Z

# 3. repoとの差分を確認（読み取りのみ）
library food import --table … --errata … --retrieved-at … \
  --ingredient-repo library/food \
  --nutrient-repo library/food-nutrients \
  --value-repo library/food-nutrient-values

# 4. 書き込み（隔離分を確認したうえで）
library food import … --apply --accept-quarantine [--concurrency 4] [--max-failures 20]
```

`--json` で報告をJSONで出す。実行ごとに `--state-dir`（既定 `.food-import/`）の
`<source_id>-<source_release>-<hash>/` に次を残す:

- `manifest.json`: 出典（URL・SHA-256・サイズ・取得時刻・シート・ヘッダ行・採用列・正誤表日付）、importer（`library-food-import/v1`・CLIバージョン）、対象repo、状態（`planned` / `blocked` / `in_progress` / `failed` / `completed`）。`completed` のときだけ公開APIに渡す `publish_hint`（`source_id=mext-sfct8-2023`, `source_release=2023+errata-2026-03-27`, `source_url`, `source_retrieved_at`, `notes` にSHA-256）を載せる。
- `report.json`: 件数、value_status別件数、表記別件数、無視した列、重複食品番号、単位の不一致、正誤表の全件の結果（誤・正・適用前・適用後・根拠 `正誤表 2026-03-27 本表第2章!R52`）、repo別の追加/変更/変更なし/キー衝突/削除候補、人の編集を保持した件数。
- `plan.json`: 送る予定のupsertの全件。
- `quarantine.jsonl`: 取り込まなかった行・セル・正誤表項目と理由。
- `writes.jsonl`: upsert 1件ごとの結果（ok / failed、試行回数、エラー）。再実行でも追記。

再開は同じコマンドの再実行。repoの現状から差分を取り直すので、書けた分は「変更なし」になり残りだけ送る。

## 対応づけの決定

| 項目 | 決定 |
| -- | -- |
| DataId | `data_` + 小文字ULID形式（26文字）。値は SHA-256(`library-food-import/v1`, `org/repo`, `source_id`, 種別, キー) の先頭128bit。キーは食材=食品番号、成分定義=成分識別子、成分値=`食品番号/成分識別子`。data IDはLibrary全体で一意なので repo 名を含める（別orgへの取込で衝突しない） |
| `ingredient_key` | `mext-` + 食品番号（`mext-01001`）。`--key-prefix` で変更可 |
| 食材 Data name | 原典の食品名（正誤表適用後） |
| `category_code` / `category_name` | 食品群列（`01`）/ 群別シート名から（`穀類`） |
| `refuse_rate` | 廃棄率を `NormalizedDecimal::parse_percentage` で正規化 |
| `cooking_state` | 食品名の最後の語を表で変換（生→`raw`、ゆで→`boiled`、焼き→`grilled`、乾→`dried`、水煮→`simmered`、油いため→`stir_fried`、蒸し→`steamed`、フライ→`breaded_fried` など24語）。表にない語は空欄（1,159食品。例: こしあん入り、味付け、粉、つくだ煮）で、人が埋める |
| `skin_bone` | 名前の語 `皮つき`/`皮なし`/`皮下脂肪なし`/`脂身つき`/`骨つき`/`骨なし` をそのまま |
| `part` / `standard_name` / `reading` | 空（人が埋める） |
| `aliases` | 新規作成時のみ、備考の `別名：` 行から（`、` 区切り→1行1件） |
| `remarks` | 備考全文。repoに `remarks` プロパティがあるときだけ書く（公開処理は読まない） |
| 成分定義 | Data name=ヘッダの名前（同名は単位を付ける: `エネルギー（kJ）`/`エネルギー（kcal）`）、`unit`=単位行、`basis`=`可食部100g当たり`、`display_order`=列順×10、`default_display`=新規時のみ `ENERC_KCAL, PROT-, FAT-, CHOCDF-, NACL_EQ` を true |
| 別定義の列 | `ENERC`/`ENERC_KCAL`、`PROT-`/`PROTCAA`、`FAT-`/`FATNLEA`、`CHOAVLM`/`CHOAVL`/`CHOAVLDF-`/`CHOCDF-` は別の `nutrient_key` のまま |
| 値 | `NutrientValueStatus::from_notation` で `value_status`/`amount`。`-`・`Tr` は数値にしない。`raw_notation` は原典の表記（数値セルは最短表記 `0.92`） |

### 上書きの規則（人の編集を壊さない）

upsert は送ったプロパティだけを書き換える patch なので、送らないことで保持する。

| 所有 | プロパティ | 既存recordへの扱い |
| -- | -- | -- |
| 原典 | 食材name, `ingredient_key`, `source_food_code`, `category_*`, `refuse_rate`, `remarks`, 成分定義 `nutrient_key`/`unit`/`basis`/`display_order`, 成分値の全項目 | 差があれば上書き |
| 推定 | `cooking_state`, `skin_bone`, `part` | `attribute_review_status=reviewed` になるまで上書き |
| 人 | `standard_name`, `reading`, `aliases`, `attribute_review_status`, 成分定義の name / `method` / `default_display` | 新規作成時だけ書く |

- 同じキーが別IDのrecord（手で作ったもの等）にあれば「キー衝突」として書かない（重複を作らない）。
- 取込対象にないrecordは「削除候補」として列挙するだけで、削除しない。今回隔離した食品のrecordは候補から外す。
- 成分定義の単位がrepoと原典で違う場合、既知の単位表（`EXPECTED_UNITS`）と違う場合は `--apply` を拒否する。

## 検証・隔離・失敗時

- 隔離（書かない・理由を残す）: 食品番号が5桁の文字列でない行、食品名なし、重複食品番号（全コピー）、廃棄率が0–100外、表記が読めないセル（`*` 等）、空セル、正誤表と食い違う値（食品名・廃棄率の食い違いは食品ごと）。
- `--apply` の拒否: ヘッダ不整合、単位の不一致、repoに必要なプロパティがない／型が String 以外（`display_order` の Integer、`default_display` の Boolean は可）。隔離・正誤表の conflict/unresolved・キー衝突があるときは `--accept-quarantine` が必要。
- 書き込み: 成分定義 → 食材 → 成分値の順。並列数は `--concurrency`（1–16）。5xx・429・409・通信エラーは最大3回まで指数バックオフで再試行。ある段で失敗が出たら次の段に進まない。`--max-failures` で打ち切る。
- 1件でも失敗・未実行があれば終了コード1、`manifest.json` の状態は `failed`、`publish_hint` は出さない。

## テスト

`apps/cli/tests/fixtures/food/` の手作りExcel（`make_fixtures.py` で生成、本物と同じヘッダ配置で数行）:

- 表: 先頭ゼロの食品番号、`-`/`0`/`Tr`/`(0)`/`(Tr)`/`(12.3)`/`数値†`/`*`、空セル、廃棄率（6・0・`"0"`・120）、備考・別名、`1002`（ゼロ落ち）、重複 `99999`。
- 正誤表: 適用済み・未適用（33→31 kcal）・備考の部分修正・conflict・食品名の部分修正・アスタリスク・不明な項目・`各成分`＋`本表` 行ペア、第1章とアミノ酸編のシート。

```
cargo test -p ingredient_notation   # 43 passed
cargo test -p library-cli           # 75 passed（food import 35件）
cargo clippy -p library-cli -p ingredient_notation --all-targets  # 警告なし
```

主なテスト: 列と単位の特定、先頭ゼロ、全表記の status/amount/raw、件数と隔離、正誤表の適用と二回目で変化なし、断片の二重適用防止、DataIdの安定性（固定値で照合）、dry runで書かない、2回目の取込で送信0件・重複0件、人の編集（standard_name/aliases/reviewed）が残る、失敗時に次段へ進まず再実行で残りだけ書く、単位変更の検出。

## 計測

- 本物の2ファイルで `--offline`（debugビルド）: 5.5秒（読込・解析・正誤表・134,511件の生成・報告）。
- repoへの書き込みはローカルAPIでも本番でも未実施。1回目は約137,100件のupsert（成分定義53＋食材2,538＋値134,511）。並列4・1件50msなら約30分の見込み。2回目以降は差分だけ。

## 残り

- [ ] 本番 org `library` への取込（`--apply`）と公開はユーザー承認が必要。本ブランチでは本番・デプロイ先に一切書いていない。
- [ ] ローカルの library-api（MySQL）に対する通しの取込と、COM-860 の公開処理の所要時間計測（13.7万件を100件ずつ読む）。
- [ ] 成分値repoの一覧取得は約1,400ページ。遅ければ一括取得APIを検討（COM-860と共通の課題）。
- [ ] `*`（エネルギー計算に使った炭水化物の印）を保存するかは未決。保存するなら成分値に別プロパティを足す。
- [ ] ヨウ素列の `*`（3セル、第3章参照）の扱いを決める。今は隔離で、公開時は `not_listed`。
- [ ] `cooking_state` の語彙を広げるか、レビューで埋めるか。
- [ ] 11316 備考の `油`/`脂` の食い違いをMEXTに確認するか、Excelの表記のままとするか。
