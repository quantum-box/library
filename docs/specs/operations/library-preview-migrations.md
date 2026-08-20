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

Lambda 本体は `tachyon.yaml` の `library-api-preview-migrate` Cloud App として
宣言する。Terraform で手作りせず Cloud App にしているのは、platform が
このリポジトリからビルドして `enterprise-library` subnet へ配布してくれるためで、
`aws lambda update-function-code` も専用の IAM も要らない。関数名は
`lambda-{app 名}` の規約で `lambda-library-api-preview-migrate` になる。

## コードのデプロイ

main にマージされると Cloud App が再デプロイされ、production alias (`prod`) が
更新される。`library-api` 側の hook は `qualifier: prod` でこの alias を指しており、
`$LATEST` は見ない。`$LATEST` は任意の PR の preview build に上書きされるため、
pin しないと別の PR の migration が流れ込む。

migration SQL は `sqlx::migrate!` でビルド時に埋め込まれる。したがって
**PR で追加した migration は、その PR が main にマージされるまで preview
データベースには適用されない。** field (PLT-3561) も同じ制約で運用している。

手元でクロスコンパイルの健全性だけ確認したい場合:

```bash
scripts/build-library-api-preview-migrate-lambda.sh
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
