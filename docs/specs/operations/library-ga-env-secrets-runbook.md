# Library GA environment and secrets Runbook

## 目的

Library GA の初回デプロイ、ロールバック、障害対応で確認する env / secret / 起動ガードを定義する。対象は txcloud Cloud App `library-api` production と、その backend として txcloud が管理する AWS Lambda runtime である。旧 `lambda-library-api` は rollback 用に残すが、通常 deploy / health / build 確認の入口は txcloud Cloud App とする。

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
| `SENTRY_DSN` | recommended secret | txcloud provider secret / Sentry project key | panic / error tracking |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | PLT-1696 scope | Sentry OTLP ingest endpoint | 全 backend の OTEL + Sentry OTLP exporter 導入時に設定 |
| `OTEL_ENABLED` | PLT-1696 scope | Lambda env | 全 backend の OTEL + Sentry OTLP exporter 導入時に設定 |
| `OTEL_SERVICE_NAME=library-api` | PLT-1696 scope | Lambda env | 全 backend の OTEL + Sentry OTLP exporter 導入時に設定 |
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

1. Cloud App と latest deployment を確認する。

```bash
tachyon compute apps get library-api \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy

tachyon compute deployments list library-api \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy
```

2. Cloud App env は値を表示せず、key と secret flag だけ確認する。`tachyon env list` の通常出力は値を表示するため使わない。

```bash
tachyon env list library-api \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy \
  --json \
  | jq -r '.[] | [.key, .target, .is_secret] | @tsv' \
  | sort
```

期待する key: `DATABASE_URL`, `SERVICE_AUTH_TOKEN`, `SENTRY_DSN`。`is_secret=false` と表示される場合でも provider secret reference 経由で runtime 解決される構成があるため、expanded value は表示せず secret store / runtime 側で解決状態を確認する。

3. 値の検査が必要な場合は、担当者の secure terminal で secret store / Lambda configuration を直接確認し、結果だけを記録する。

- `DATABASE_URL` に `localhost` / `127.0.0.1`
- `SERVICE_AUTH_TOKEN` が `dummy-token` または placeholder
- `SENTRY_DSN` が未設定、または production Sentry project 以外を指す
- `TACHYON_API_URL` が `pages.dev` / localhost / non-HTTPS
- `LIBRARY_PARQUET_BUCKET` が未設定または `library-parquet`
- `MINIO_*` または `SKIP_MINIO_SETUP`

## 初回デプロイ後チェック

1. API の health を確認する。

```bash
curl -fsS https://library-api.txcloud.app/health
```

期待値: `OK`

2. version endpoint を確認する。

```bash
curl -fsS https://library-api.txcloud.app/version
```

3. txcloud build / deployment event を確認する。

```bash
tachyon compute builds list library-api \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy

tachyon compute deployments list library-api \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy
```

4. Runtime log は既存の txcloud Cloud App backend log で確認する。`tachyon compute logs --tail` は Cloudflare-backed app 専用で、`deploymentTarget: lambda` では使わない。

```bash
aws logs tail <txcloud-managed-library-api-log-group> --since 30m --follow
```

PLT-1680 では CloudWatch log metric filter / alarm の新規作成はしない。Sentry Team plan で OTEL 直接送信が使えるため、runtime error spike の恒久検知は PLT-1696 の全 backend OTEL + Sentry OTLP exporter 導入で扱う。

確認ポイント:

- `START` / `END` / `REPORT` が出ている
- `ERROR` / `Task timed out` / `Runtime exited` / `panic` が継続していない
- `status=5xx` または server-side error log が継続していない
- DB pool acquire timeout / slow acquire warn が継続していない
- `ENVIRONMENT must be production` が出ていない
- `SERVICE_AUTH_TOKEN must be configured in production` が出ていない
- `TACHYON_API_URL must point at the production API origin` が出ていない
- OAuth bootstrap 失敗が継続していない
- S3 / Parquet write error が継続していない

5. Sentry は production project で `environment=production`, `service=library-api` 相当の event / issue を確認する。実 production event の強制発火は user impact と alert noise を生むため、既存 error event または設定確認で代替する。

## Migration 実行経路

Library repo には `library-api` の deploy / migration CI を置かない。API build / deploy は txcloud Cloud App 側の build / deployment status を正とする。

PLT-1954 が完了するまでは、Library 側に GitHub Actions の migration bridge を追加しない。migration 実行が必要な場合は txcloud / Tachyon 側の運用手順として扱い、Library repo の CI から AWS credential / Lambda invoke を実行しない。

Tachyon 側に Cloud App migration hook が実装されるまでは、`tachyon.yaml` に hook 設定は追加しない。現時点では hook phase / schema が未確定のため、Library 側は `library-api-migrate` Cloud App の定義だけを保持する。

PLT-1954 完了後の切替手順:

1. Tachyon 側の実装に合わせて、`tachyon.yaml` に migration hook 設定を追加する。
2. txcloud 側の build / deployment flow で migration hook が `library-api` deploy 前に実行されることを確認する。
3. Library repo に deploy / migration CI を再追加しない。
4. txcloud build / deploy 後に `/health`, `/version`, GraphQL introspection, `planet-library` sign-in route を確認する。

## ロールバック

1. 直前の安定版 txcloud deployment を確認する。

```bash
tachyon compute deployments list library-api \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy
```

2. txcloud deployment を戻す。

```bash
tachyon compute deployments rollback library-api <deployment-id> \
  --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy
```

3. txcloud rollback で復旧できない場合だけ、旧 rollback Lambda の安定版 version / alias を確認する。

```bash
aws lambda list-versions-by-function \
  --function-name lambda-library-api \
  --query 'Versions[*].{Version:Version,LastModified:LastModified}'
```

4. alias を安定版へ戻す。

```bash
aws lambda update-alias \
  --function-name lambda-library-api \
  --name production \
  --function-version <stable-version>
```

5. env / secret の変更が原因の場合は、Terraform / Tachyon Cloud App 側の変更を revert して再適用する。`DATABASE_URL`, `SERVICE_AUTH_TOKEN`, `SENTRY_DSN` は値をログや PR に貼らない。

6. health / version / logs を再確認する。

## 障害時の一次切り分け

| 症状 | 確認箇所 | 初動 |
| --- | --- | --- |
| 起動直後に Cloud App が落ちる | txcloud deployment status, 既存 runtime logs の guard message | env / secret を修正して redeploy |
| Sentry event が入らない | `SENTRY_DSN` secret presence, Sentry project environment filter | secret reference と production project を確認 |
| 401 / 403 が増える | `SERVICE_AUTH_TOKEN`, Tachyon API service account | token rotation / Tachyon API 側権限を確認 |
| OAuth callback が失敗する | Tachyon OAuth bootstrap, `GITHUB_REDIRECT_URI` | callback URL と provider secret を確認 |
| Parquet 出力が失敗する | `LIBRARY_PARQUET_BUCKET`, Lambda IAM policy | bucket name と `s3:GetObject/PutObject/ListBucket` を確認 |
| JWT 検証が失敗する | `COGNITO_JWK_URL`, `COGNITO_USER_POOL_ID` | pool id と JWKS URL の一致を確認 |
