# library-api の migration 運用 (PLT-3561 / PLT-3861)

preview の per-PR データベース (ADR-0049 / PLT-3328) と production の
`library` / `tachyon_apps_database_manager` に migration を適用する経路の運用手順。

## migration gate

preview と production のどちらも、適用経路は **postDeploy hook の migration
gate 一本**である。

1. `apps/api/migrations/*.sql` または `packages/database-manager/migrations/*.sql`
   を変更して push すると、Tachyon が library-api をビルドし candidate alias の
   Lambda を作る
2. stable にトラフィックを流す前に、`tachyon.yaml` の
   `hooks.postDeploy.migration-gate-before-activation` が **同じ Lambda** を
   `lambdaInvoke` する (`routeKey: LIBRARY_MIGRATION_GATE` / `rawPath: /` /
   `domainName: library-api-migration-gate.internal`、timeout 900s)
3. `apps/api/bin/lambda.rs` の `is_migration_gate_event` がこの 3 点一致だけを
   gate イベントと判定し、`library_api::migrations::run_migration_gate` を呼ぶ
4. 失敗すると Lambda が `FunctionError` を返し、candidate は破棄され、現に
   serving 中の Lambda は無傷。失敗理由は `tracing::error!` で CloudWatch
   (`/aws/lambda/lambda-library-api`) に出る

3 点一致は public Function URL から到達できない組み合わせなので、外部リクエストが
migration を起動することはない。serving 初期化は gate を呼ばない。壊れた
migration 履歴は「deploy を止める」ものであって「稼働中の API を落とす」ものでは
ないため。

## preview と production で適用するものが違う

| 環境 | 物理データベース | SQLx 履歴 |
| --- | --- | --- |
| production | `library` と `tachyon_apps_database_manager` の 2 つ | 2 つ |
| preview | per-PR データベース 1 つ | 1 つ (combined) |

`run_migration_gate` は `DatabaseLayout` に「どちらのレイアウトを runtime が
見るか」を尋ね、その答えで分岐する。preview では
`library-api-preview-migrate` の combined migrator が、`library.` /
`tachyon_apps_database_manager.` という論理データベース修飾を除去したうえで両方の
migration を 1 つの履歴に流し込む (PLT-3328)。

preview データベース名は platform の `preview_database_name` が決める。
tachyon-apps PLT-3851 (#8818, 2026-08-23) で app slug が前に付き、現在は
`{app_slug}_pr{pr_number}_{app_id の末尾 12 文字}` = library-api では
`library_api_pr<PR番号>_k6cypbws3q3j`。それ以前に払い出された open PR は旧形式の
`pr_<PR番号>_k6cypbws3q3j` を持ち続けるため、
`library_api_preview_migrate::is_pr_scoped_database_name` が両方を受け入れる。
この判定は `DatabaseLayout` も使う。gate が書くスキーマと runtime が読む
レイアウトが食い違わないようにするためで、判定を複製してはいけない。

## migration SQL の TiDB 互換

preview / production ともデータベースは TiDB 上にあり、
`tidb_enable_check_constraint` は `migration_preflight` により ON である。TiDB は
この状態で `ALTER TABLE ... DROP CONSTRAINT` を CHECK 制約としてのみ解決するため、
UNIQUE キーを落とそうとすると `ERROR 3940 (HY000): Constraint '...' does not exist`
になる。UNIQUE キーは MySQL でも TiDB でも `DROP INDEX` で落とせるので、migration
では `DROP INDEX` を使うこと (PLT-3861 で `20250305080000_update_unique_constraints`
を修正済み)。

本番が既に適用した migration ファイルを書き換えると sqlx の checksum 検証に
かかるため、`apps/api/src/migrations.rs` の `UPDATE_UNIQUE_CONSTRAINTS_CHECKSUM`
で記録側の checksum を現ファイルに揃えている。ファイルを再度変更したら
`pinned_checksum_matches_the_migration_file` が落ちるので、その定数も更新する。

## 失敗した preview データベースの復旧

migration が途中で落ちると `_sqlx_migrations` に `success = FALSE` の行が残り、
以降の実行は `partially applied` で止まる。per-PR データベースは deploy ごとに
作り直されないため、preview migrator は本番と同じく実行前にその行を削除する
(`clear_failed_sqlx_migrations`)。それでも直らない場合だけ、PR を close → reopen
してデータベースごと作り直す。

## 失敗の切り分け

gate は接続前に DNS 解決と TCP connect を先に試し、sqlx が `PoolTimedOut` に
丸めてしまう前に原因を分けて報告する。

| メッセージ | 意味 |
| --- | --- |
| `preview database host ... does not resolve` | DNS。secret の host が壊れているか、VPC 外から PrivateLink 名を引いている |
| `preview database ...:4000 is unreachable: TCP connect got no response` | 経路がない。migration が VPC 外で走っている |
| `preview database ...:4000 rejected the connection` | 経路はある。listener がないか SG で reject されている |
| `failed to connect to preview database ...: <sqlx のエラー>` | TCP は通った。認証・TLS・データベース不在などの本来のエラー |
| `preview DATABASE_URL must select a PR-scoped database` | 接続先が per-PR データベース名の形をしていない。preview 用の解決が誤っている |

## ローカルでの再現

TiDB を立てて combined migrator をそのまま replay できる。

```bash
docker run -d --rm -p 4001:4000 pingcap/tidb:v8.5.7
mysql -h 127.0.0.1 -P 4001 -u root -e "SET GLOBAL tidb_enable_check_constraint=ON"
DEV_DATABASE_URL='mysql://root@127.0.0.1:4001/mysql' \
  cargo test -p library-api-preview-migrate --lib -- --ignored
```

CI (`.github/workflows/ci.yml`) は同じ replay を MySQL に対して実行する。TiDB
固有の非互換はこのローカル手順でしか出ないので、DDL を足したときは両方で流す。

## 旧経路 (削除済み)

2026-08 まで、preview は `provisionedDatabase.migration.lambdaInvoke` で
`lambda-library-api-preview-migrate` を、production は `hooks.preDeploy` で
`lambda-library-api-migrate` を呼んでいた。どちらも migration SQL を自分の
ビルド時に埋め込む別 Lambda で、コードは out-of-band 配布だったため、
migration を足しても再デプロイを忘れると preview に反映されなかった。gate は
candidate 自身を呼ぶのでこのズレが原理的に起きない。

**残作業**: tachyon-apps 側の Terraform
(`cluster/n1-aws/library_preview_migrate.tf` ほか) に残る 2 つの Lambda 関数と、
`library_repo_oidc.tf` の `LibraryLambdaDeployPolicy` は未使用になる。gate が
production で通ったことを確認してから削除する。
