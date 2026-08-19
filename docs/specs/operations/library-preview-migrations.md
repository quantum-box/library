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
宣言する。Terraform は設定だけを固定し、コードは out-of-band で配布する
(`lambda-library-api-migrate` と同じ方式)。

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

## 失敗の切り分け

`library-api-preview-migrate` は接続前に DNS 解決と TCP connect を先に試し、
sqlx が `PoolTimedOut` に丸めてしまう前に原因を分けて報告する。

| メッセージ | 意味 |
| --- | --- |
| `preview database host ... does not resolve` | DNS。secret の host が壊れているか、VPC 外から PrivateLink 名を引いている |
| `preview database ...:4000 is unreachable: TCP connect got no response` | 経路がない。migration が VPC 外 (deploy hook runner 等) で走っている |
| `preview database ...:4000 rejected the connection` | 経路はある。listener がないか SG で reject されている |
| `failed to connect to preview database ...: <sqlx のエラー>` | TCP は通った。認証・TLS・データベース不在などの本来のエラー |
| `preview DATABASE_URL must select a PR-scoped database` | 接続先が `pr_` で始まらない。preview 用の解決が誤っている |

preview データベース名は platform が `pr_{pr_number}_{app_id の末尾 12 文字}` で
決める (`preview_database_name`)。library-api では `pr_<PR番号>_k6cypbws3q3j`。
