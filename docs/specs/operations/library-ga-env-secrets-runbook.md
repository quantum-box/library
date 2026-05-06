# Library GA environment and secrets Runbook

## 目的

Library GA の初回デプロイ、ロールバック、障害対応で確認する env / secret / 起動ガードを定義する。対象は `library-api` production Lambda と、同じ `Config` を使う server binary。

## 起動ガード

`apps/api/src/config.rs::Config::validate_for_server_startup` は production / prod で次を拒否する。

| 項目 | 拒否条件 | 理由 |
| --- | --- | --- |
| `ENVIRONMENT` | AWS Lambda 上で production / prod 以外 | Lambda を dev/test 設定で起動しない |
| `SERVICE_AUTH_TOKEN` | 空、`dummy-token`、`dummy`、`test`、`secret`、`changeme`、`placeholder` | Tachyon API への service auth を dummy で通さない |
| `DATABASE_URL` | `localhost` / `127.0.0.1` | 本番 Lambda からローカル DB を参照しない |
| `COGNITO_JWK_URL` | 空、非 HTTPS、localhost、`COGNITO_USER_POOL_ID` と不一致 | JWT 検証先の誤設定を防ぐ |
| `TACHYON_API_URL` | 空、非 HTTPS、localhost、`pages.dev` | service auth / OAuth bootstrap の向き先誤りを防ぐ |
| `LIBRARY_PARQUET_BUCKET` | 未設定、空、`library-parquet` | 開発 default bucket への書き込みを防ぐ |
| `MINIO_*` / `SKIP_MINIO_SETUP` | production で設定済み | production は S3 を使い、MinIO fallback を使わない |

## 本番必須 env

| Env | Scope | Source | 確認方法 |
| --- | --- | --- | --- |
| `ENVIRONMENT=production` | required | Lambda env / Cloud App env | Lambda configuration または Tachyon Cloud App env |
| `DATABASE_URL` | required secret | AWS Secrets Manager / Terraform var | `localhost` でない TiDB / MySQL DSN であること |
| `COGNITO_JWK_URL` | required | Cognito Terraform output | `https://cognito-idp.<region>.amazonaws.com/<pool>/.well-known/jwks.json` |
| `COGNITO_USER_POOL_ID` | required | Cognito Terraform output | `COGNITO_JWK_URL` の pool id と一致 |
| `TACHYON_API_URL` | required | Lambda env / Tachyon Cloud App env | `https://api.n1.tachy.one` など production API origin |
| `SERVICE_AUTH_TOKEN` | required secret | Tachyon service account / Secrets Manager | dummy 値でない bearer token |
| `LIBRARY_PARQUET_BUCKET` | required | Terraform `aws_s3_bucket.library_parquet.id` | production bucket name |
| `PORT` | required for server binary | deploy platform | Lambda runtime では通常不要、Cloud App では `8080` |

## 本番推奨 env

| Env | Scope | Source | 用途 |
| --- | --- | --- | --- |
| `SENTRY_DSN` | recommended | Sentry project key | panic / error tracking |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | recommended | ADOT / collector endpoint | tracing export |
| `OTEL_ENABLED` | recommended on Lambda | Lambda env | ADOT / X-Ray tracing enablement |
| `OTEL_SERVICE_NAME=library-api` | recommended | Lambda env | metric / trace service name |
| `AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH=true` | recommended on Lambda | Lambda env or bootstrap | API Gateway stage prefix handling |

## GA scope 別の optional / disabled env

| Env / provider | GA scope | 運用 |
| --- | --- | --- |
| `GITHUB_REDIRECT_URI` | optional override | Tachyon API から返る OAuth bootstrap を優先。緊急時のみ env override |
| `OAUTH_STATE_SECRET` / `GITHUB_CLIENT_SECRET` | optional fallback | GitHub OAuth config が Tachyon bootstrap から取得できない場合の fallback。通常は secret を Tachyon 側に集約 |
| `SQUARE_API_KEY` | optional integration | Square inbound sync を GA に含める場合だけ設定 |
| `SQUARE_SANDBOX` | disabled in production unless explicitly approved | production では sandbox 誤接続を避ける |
| `MINIO_ENDPOINT` / `MINIO_PUBLIC_ENDPOINT` | disabled in production | local / test 専用 |
| `MINIO_ROOT_USER` / `MINIO_ROOT_PASSWORD` | disabled in production | local / test 専用 |
| `SKIP_MINIO_SETUP` | disabled in production | local / test 専用 |

## デプロイ前チェック

1. Lambda / Cloud App env を確認する。

```bash
aws lambda get-function-configuration \
  --function-name lambda-library-api \
  --query 'Environment.Variables'
```

2. secret 値そのものは表示せず、存在と向き先だけ確認する。

```bash
aws lambda get-function-configuration \
  --function-name lambda-library-api \
  --query 'Environment.Variables.{env:ENVIRONMENT,db:DATABASE_URL,cognito:COGNITO_JWK_URL,tachyon:TACHYON_API_URL,bucket:LIBRARY_PARQUET_BUCKET}'
```

3. 次の値が含まれていたらデプロイを止める。

- `DATABASE_URL` に `localhost` / `127.0.0.1`
- `SERVICE_AUTH_TOKEN` が `dummy-token` または placeholder
- `TACHYON_API_URL` が `pages.dev` / localhost / non-HTTPS
- `LIBRARY_PARQUET_BUCKET` が未設定または `library-parquet`
- `MINIO_*` または `SKIP_MINIO_SETUP`

## 初回デプロイ後チェック

1. API の health を確認する。

```bash
curl -fsS https://library.api.n1.tachy.one/
```

期待値: `OK`

2. version endpoint を確認する。

```bash
curl -fsS https://library.api.n1.tachy.one/version
```

3. Lambda logs で guard / bootstrap エラーがないことを確認する。

```bash
aws logs tail /aws/lambda/lambda-library-api --since 30m --follow
```

確認ポイント:

- `ENVIRONMENT must be production` が出ていない
- `SERVICE_AUTH_TOKEN must be configured in production` が出ていない
- `TACHYON_API_URL must point at the production API origin` が出ていない
- OAuth bootstrap 失敗が継続していない
- S3 / Parquet write error が継続していない

## ロールバック

1. 直前の安定版 Lambda version / alias を確認する。

```bash
aws lambda list-versions-by-function \
  --function-name lambda-library-api \
  --query 'Versions[*].{Version:Version,LastModified:LastModified}'
```

2. alias を安定版へ戻す。

```bash
aws lambda update-alias \
  --function-name lambda-library-api \
  --name production \
  --function-version <stable-version>
```

3. env / secret の変更が原因の場合は、Terraform / Tachyon Cloud App 側の変更を revert して再適用する。`DATABASE_URL` と `SERVICE_AUTH_TOKEN` は値をログや PR に貼らない。

4. health / version / logs を再確認する。

## 障害時の一次切り分け

| 症状 | 確認箇所 | 初動 |
| --- | --- | --- |
| 起動直後に Lambda が落ちる | CloudWatch logs の guard message | env / secret を修正して redeploy |
| 401 / 403 が増える | `SERVICE_AUTH_TOKEN`, Tachyon API service account | token rotation / Tachyon API 側権限を確認 |
| OAuth callback が失敗する | Tachyon OAuth bootstrap, `GITHUB_REDIRECT_URI` | callback URL と provider secret を確認 |
| Parquet 出力が失敗する | `LIBRARY_PARQUET_BUCKET`, Lambda IAM policy | bucket name と `s3:GetObject/PutObject/ListBucket` を確認 |
| JWT 検証が失敗する | `COGNITO_JWK_URL`, `COGNITO_USER_POOL_ID` | pool id と JWKS URL の一致を確認 |
