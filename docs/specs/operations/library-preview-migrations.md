# Preview データベース migration の運用 (PLT-3561)

PR ごとの preview データベース (ADR-0049 / PLT-3328) に migration を適用する
経路の運用手順。

## なぜ Lambda 経由なのか

per-PR の preview データベースは user Cloud App TiDB 上に作られ、この TiDB は
PrivateLink 専用 (`aws_vpc_endpoint.user_tidb` は `private_dns_enabled = false`)
である。一方 `provisionedDatabase.migration.command` は Hetzner k3s の JobRun
runner で実行され、この runner は egress default-deny + 443/tcp のみ許可という
NetworkPolicy 下にある (`tachyon-jobrun-egress-default-deny`,
`tachyon-jobrun-egress-allow-public-https`)。TiDB の 4000/tcp は静かに drop され、
sqlx の acquire timeout 30 秒後に `pool timed out` になるだけで、原因は表に出ない。

そのため `tachyon.yaml` は `migration.lambdaInvoke` を使い、`enterprise-library`
subnet に置いた `lambda-library-api-preview-migrate` で migration を実行する。この
subnet の `enterprise_library_lambda_sg` には `user_tidb_endpoint_sg` への 4000/tcp
egress が既にある。platform は解決済みの接続 URL を invoke payload の
`databaseUrl` / `databaseUrlEnv` に入れて渡す。

Lambda 本体は tachyon-apps の `cluster/n1-aws/library_preview_migrate.tf` が
宣言する。Terraform は関数の器（実行ロール、VPC 配置、timeout）と fail-loud
placeholder だけを持ち、コードは out-of-band で配布する。

## なぜ Cloud App にできないのか

`create_deployment.rs` の `deploy_to_lambda` は Lambda Cloud App 共通の唯一の経路で、
candidate alias が Function URL を返さなければ hard fail し、その URL に HTTP GET の
serving probe をかける。この関数は `databaseUrl` を含む invoke payload を処理する
ものなので、コードを配るたびに probe で落ちる。

probe に応答させることは可能だが、VPC 内にいて呼び出し側が渡した URL に対して
migration を実行する関数に public Function URL が付くことになる。`databaseUrl` は
呼び出し側が指定できるため、内部への pivot に使える。field-preview-migrate が
out-of-band で配布されているのも同じ理由。

## コードのデプロイ

migration SQL は `sqlx::migrate!` でビルド時に埋め込まれる。**preview データベース
には、成果物をビルドした時点の migration しか適用されない。**
`apps/api/migrations` または `packages/database-manager/migrations` を変更したら、
必ず再ビルドして再デプロイすること。

```bash
scripts/build-library-api-preview-migrate-lambda.sh
aws lambda update-function-code \
  --function-name lambda-library-api-preview-migrate \
  --zip-file "fileb://target/lambda/lambda-library-api-preview-migrate/bootstrap.zip"
```

権限は `library_repo_oidc.tf` の `LibraryLambdaDeployPolicy` にある。

## 失敗の切り分け

`library-api-preview-migrate` は接続前に DNS 解決と TCP connect を先に試し、
sqlx が `PoolTimedOut` に丸めてしまう前に原因を分けて報告する。

| メッセージ | 意味 |
| --- | --- |
| `preview database host ... does not resolve` | DNS。secret の host が壊れているか、VPC 外から PrivateLink 名を引いている |
| `preview database ...:4000 is unreachable: TCP connect got no response` | 経路がない。migration が VPC 外 (deploy hook runner 等) で走っている |
| `preview database ...:4000 rejected the connection` | 経路はある。listener がないか SG で reject されている |
| `failed to connect to preview database ...: <sqlx のエラー>` | TCP は通った。認証・TLS・データベース不在などの本来のエラー |
| `preview DATABASE_URL must select a PR-scoped database` | 接続先が per-PR データベース名の形をしていない。preview 用の解決が誤っている |

preview データベース名は platform の `preview_database_name` が決める。
tachyon-apps PLT-3851 (#8818, 2026-08-23) で app slug が前に付き、現在は
`{app_slug}_pr{pr_number}_{app_id の末尾 12 文字}` = library-api では
`library_api_pr<PR番号>_k6cypbws3q3j`。それ以前に払い出された open PR は旧形式の
`pr_<PR番号>_k6cypbws3q3j` を持ち続けるため、`require_pr_scoped_database` は
両方を受け入れる。

## migration SQL の TiDB 互換

preview データベースは TiDB 上にあり、`tidb_enable_check_constraint` は
`migration_preflight` により ON である。TiDB はこの状態で
`ALTER TABLE ... DROP CONSTRAINT` を CHECK 制約としてのみ解決するため、UNIQUE
キーを落とそうとすると `ERROR 3940 (HY000): Constraint '...' does not exist` に
なる。UNIQUE キーは MySQL でも TiDB でも `DROP INDEX` で落とせるので、migration
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
