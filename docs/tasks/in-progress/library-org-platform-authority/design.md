# 組織の platform と上流認可の正を一つにする Design Document

## 目的

取り込み組織（別 platform から `seedLibraryTenant` で採用した tenant）で
`Forbidden: Upstream authorization rejected` が起きる構造を無くす。
個別の呼び出し箇所を直すのではなく、間違った scope を作れない設計にする。

関連: #424 (PLT-6094)、#440、#441、#447

## 背景: 同じ 403 を 4 回塞いだ

| PR | 塞いだ経路 | 原因 |
| --- | --- | --- |
| #424 | MCP `create_repo`（check-for-user） | サービスアカウントに bernard-square 用の委任 grant が無い。本番で手作業登録 |
| #440 | client の repo 閲覧 | client が `x-platform-id` に Library を送っていた |
| #441 | パス extractor | server が `LibraryOrg` を Library platform 固定で組んでいた |
| #447 | `create_repo` の認可、MCP 32 tool | #441 と同じ。extractor 以外で `with_org_and_operator` を直接使っていた |

tachyon-api は `x-platform-id` が operator 自身の platform と違うと
`PermissionDenied: x-platform-id does not match the operator's platform` を返す。
Library はこれを `Upstream authorization rejected` に丸めるため、原因が見えない。

## 現状の問題

1. **platform の決定点が複数ある。**
   - client が `x-platform-id` を送り、server の GraphQL `MultiTenancy` はそれをそのまま使う
   - server は extractor、MCP、usecase でそれぞれ `LibraryOrg` を組み立てる
   - `LibraryOrg::with_org_and_operator` は public で、既定が Library platform。新しい経路は既定で間違う
2. **判定失敗が黙って Library に倒れる。** `is_on_other_platform` が `None` のとき Library platform を名乗る。
   失敗は 403 として後から現れ、原因の場所とずれる。
3. **`organizations.platform_id` の意味が紛らわしい。** この列は「Library 側の区画」で常に `LIBRARY_TENANT`。
   組織の home platform（tachyon 上の所属）はどこにも保存されておらず、毎回 tachyon に問い合わせている（10 分 cache）。
4. **取り込みに必要な IAM が手作業。** 取り込み組織ごとにサービスアカウントへ
   `auth:EvaluatePoliciesForUserInTenant`（resource `trn:auth:operator:<tenant>`）の grant が要る。
   PLT-6094 では bernard-square 分を手で登録した。`seedLibraryTenant` の policy 付与は失敗しても warn で続行する。
5. **失敗が観測されていない。** `failed to resolve OAuth config … Upstream authentication rejected`
   （Library platform `tn_01j91h09tpj5ehwbwfwfxpak2b`）が 2026-11-08 から 12 時間あたり数百件出続けている。

## 設計

### D1. home platform を Organization に持たせる

- `organizations` に `home_platform_id VARCHAR(29) NULL` を追加する。既存の `platform_id`（区画）とは別の列にする
- `Organization` に `home_platform: PlatformId` を持たせる。復元は reconstruction factory で行い、setter は作らない
- 書き込むのは次の 2 箇所だけ
  - `CreateOrganization`: Library platform
  - `seedLibraryTenant`: tachyon の `get_operator` が返した `platform_id`
- backfill: 既存行は migration 後に一度だけ tachyon の operator を引いて埋める（運用スクリプト）。
  埋まるまでは `NULL` を「未確定」として扱い、D2 で fail closed にする

### D2. 組織 scope は一つの型からしか作れない

- `OrgScope`（現 `LibraryOrg` の置き換え）を導入し、構築子を `OrgScope::for_organization(&Organization)` だけにする
  - `operator_id = organization.id()`
  - `platform_id = organization.home_platform()`。`NULL` のときは構築できない（`Err`）
- `with_org_and_operator` / `with_org` / `on_other_platform` は削除する。`LibraryOrg::resolve`（#447）も置き換える
- 組織 scope の tachyon 呼び出しは `OrgScope` だけを受け取る
  （`MultiTenancyAction` を広く受ける SDK wrapper の引数を `&OrgScope` に狭める）
- 組織に紐づかない呼び出し（サインイン、自分の組織一覧）だけが `LibraryPlatformScope` を使う

### D3. client の `x-platform-id` を信用しない

- 組織 scope の API で、server は client が送った `x-platform-id` を読まない。`x-operator-id` は宛先の照合（`ensure_creation_tenant`）にだけ使う
- client の `platformForOperator` と `operatorPlatforms` の cache は削除する

### D4. 取り込み時に委任 grant を付ける

- `seedLibraryTenant` が、取り込む tenant 用の委任 grant（`auth:EvaluatePoliciesForUserInTenant`、
  resource `trn:auth:operator:<tenant>`）をサービスアカウントに付ける
- 付与に失敗したら取り込みを失敗させる（warn で続行しない）。冪等にして再実行で回復できるようにする
- 既存の取り込み組織（quantumbox、bernard-square ほか）は backfill で同じ grant を揃える
- 取り込みを止める（offboard）ときは grant を外す

### D5. 上流の拒否理由を観測できるようにする

- 公開エラーは今のまま `Upstream authorization rejected` に丸める
- 内部ログには上流の error code と `upstream_request_id` に加えて、送った `x-platform-id` と operator を出す
- `failed to resolve OAuth config` と、check_policy の 403 率にアラートを付ける

## 形式検証の使いどころ

形式検証で効く範囲は限られる。今回の 4 件の主因は「型を通らない経路」「手作業の IAM」「client 由来の header」で、
関数単体の証明では捕まらない。そのため優先順位は次のとおりにする。

1. **型で不正な状態を作れなくする（D2）。** 今回のクラスのバグを最も安く消せる。`OrgScope` を経由しない組織 scope がコンパイルできなくなる
2. **全経路の結合テスト。** HTTP 境界を mock した既存の `owner_access_case` 形式で、
   GraphQL / REST / MCP の各経路について「取り込み組織では送る `x-platform-id` = home platform」を表で検査する
3. **形式検証（Verus など）は純粋な判定関数に限る。** 対象と性質は次のとおり
   - `OrgScope::for_organization`: 結果の platform は常に `home_platform`。`NULL` なら構築しない
   - `ensure_creation_tenant`: scope の tenant ≠ 宛先 tenant なら、いかなる書き込みより前に拒否する
   - repo 作成の owner 判定: grant は呼び出し者自身にしか再利用されない（既存テスト
     `another_users_grant_cannot_reuse_the_callers_owner_access` の性質を全入力で証明する）
4. **tachyon との契約。** 「`x-platform-id` は operator の platform と一致しなければならない」は tachyon 側の不変条件。
   Library 側で証明しても tachyon の実装が変われば崩れるので、契約テスト（tachyon の実 API に対する smoke）で担保する

Verus は tachyon-apps にも Library にもまだ導入されていない。導入するなら 3 の範囲から始める。

## 範囲外

- 部分作成のロールバック（DB・プロパティを作った後に grant が失敗すると行が残る）。別タスクで transactional outbox か事前認可にする
- ユーザートークンと MCP リソーストークンで上流の経路（`policies/check` と `check-for-user`）が分かれている件の統一

## 移行手順

1. migration: `home_platform_id` 追加（NULL 許容）
2. backfill スクリプト: 既存 organizations に home platform を埋め、取り込み組織の委任 grant を揃える
3. `OrgScope` 導入、全呼び出し箇所を置き換え、`LibraryOrg` 削除
4. D3（client と server の header 処理）
5. backfill 完了を確認してから `home_platform_id` を `NOT NULL` に変える

## 受け入れ条件

- 取り込み組織（quantumbox、bernard-square）で、Web・MCP の両方から repo 作成・閲覧・更新ができる
- 新しい組織を取り込んだ直後に、手作業の IAM 登録なしで同じ操作ができる
- `with_org_and_operator` 相当の、home platform を経由しない scope 構築子がコードに存在しない
- tachyon-api の `x-platform-id does not match` が 7 日間 0 件
