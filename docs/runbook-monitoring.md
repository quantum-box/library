# Library GA monitoring Runbook

## 目的

Library GA 後 1 週間の監視、アラート、ステータス更新、一次対応を定義する。対象は CMS / Document OS の正式提供範囲と、GA 直後に障害影響が大きい認証、公開閲覧、GraphQL / REST API、Webhook worker、sync operation、storage / Parquet 出力である。

## 監視対象

| 領域 | メトリクス / ログ / trace | 監視頻度 | 目的 |
| --- | --- | --- | --- |
| API availability | `GET /health`, `GET /version`, GitHub scheduled synthetic check, txcloud deployment status | 1-5 分 | API が利用可能で、production build が応答していることを確認する |
| API latency | API Gateway latency, backend Lambda duration p50 / p95 / p99, trace span duration | 5 分 | GA 後の実利用増加で体感速度が劣化していないことを確認する |
| GraphQL errors | `/v1/graphql` response `errors[*].extensions.code`, resolver error logs, Sentry events | 5 分 | GraphQL は HTTP 200 でも業務エラーを返すため、HTTP status だけに依存しない |
| REST errors | 4xx / 5xx rate, structured error code, Sentry events | 5 分 | public docs / API key / OAuth / webhook endpoint などの失敗を検知する |
| Authentication | Cognito auth failure, JWT verification failure, API key verify failure, 401 / 403 rate | 5 分 | ログイン不能、expired key、権限境界の誤動作を検知する |
| Public documents | `/docs/{org}/{repo}` and `/docs/{org}/{repo}/{data_id}` synthetic check | 5 分 | GA の公開閲覧導線が落ちていないことを確認する |
| Webhook worker | received / accepted / rejected / failed event count, DLQ depth, retry count | 5 分 | webhook 取り込みの停止、署名検証失敗、再試行詰まりを検知する |
| Sync operation | started / succeeded / failed / timed out count, operation age, queue depth | 5 分 | sync が backlog 化または連続失敗していないことを確認する |
| Database | connection errors, query timeout, TiDB / MySQL CPU, slow query count | 5 分 | API 失敗の下流原因を早期に切り分ける |
| Storage / Parquet | S3 `PutObject` / `GetObject` errors, bucket access denied, Parquet write failure logs | 5 分 | export / analytics 系の保存先誤設定や IAM 不足を検知する |
| Configuration guard | production startup guard logs, missing secret logs, invalid URL logs | deploy 時 / 5 分 | GA 環境の env / secret 誤設定を即時に検知する |
| Client health | web app load failure, JS error, Sentry browser event, key user flow synthetic check | 5 分 | API が正常でも UI が利用不能な状態を検知する |

## アラート条件

### Critical

即時対応。担当者が 15 分以内に一次判断し、必要なら rollback / traffic stop / incident 宣言を行う。

| 条件 | 判定窓 | 初動 |
| --- | --- | --- |
| API synthetic check が 3 回連続失敗 | 15 分 | `GET /health` / `GET /version`、txcloud deployment、既存 runtime logs、直近 deploy を確認し、起動 guard / runtime error なら rollback |
| Sentry production issue / event が 5 分で急増、または 5xx rate が 5% 以上 | 5 分 | Sentry issue と既存 runtime logs を確認し、単一 endpoint か全体障害かを分類 |
| Cognito / JWT / API key 検証失敗により 401 / 403 が通常比 3 倍以上、かつログイン成功がない | 10 分 | Cognito client id / JWKS URL / service token / API key verification を確認 |
| GraphQL mutation の `internal_server_error` が 10 件以上 | 5 分 | resolver error と affected org / repo を確認し、書き込み停止や rollback を判断 |
| Webhook worker DLQ depth が 1 以上、または event loss が疑われる | 5 分 | provider event id と retry state を保存し、再処理可否を判断 |
| Sync operation failure rate が 20% 以上、または最古 pending age が 30 分超 | 15 分 | worker / queue / provider rate limit / auth error を確認し、sync 一時停止を検討 |
| Public document synthetic check が 3 回連続失敗 | 3 分 | public/private 判定、route、storage、API upstream を確認 |

### Warning

営業時間内に優先対応。GA 初週は daily standup で必ず共有する。

| 条件 | 判定窓 | 初動 |
| --- | --- | --- |
| API p95 latency が 2 秒超 | 15 分 | slow query、Lambda cold start、external call を trace で確認 |
| GraphQL `permission_denied` / `bad_request` が通常比 2 倍以上 | 30 分 | UI regression、権限設定ミス、利用者操作ミスを分類 |
| REST 4xx rate が通常比 2 倍以上 | 30 分 | endpoint 別に集計し、認証系か入力 validation 系かを分類 |
| Sentry の新規 issue が 3 件以上 | 30 分 | Critical に昇格すべき user-impacting error がないか確認 |
| Webhook rejected event が通常比 2 倍以上 | 30 分 | signature / payload schema / provider 側変更を確認 |
| Sync operation retry count が通常比 2 倍以上 | 30 分 | provider throttling と transient failure を切り分ける |
| Storage / Parquet write error が 1 件以上 | 30 分 | bucket / IAM / object key / payload size を確認 |

### Info

週次レビューで傾向を見る。GA 初週は daily report に含める。

| 条件 | 用途 |
| --- | --- |
| traffic / active org / active repo / published docs count の増加 | 利用量の伸びと capacity 判断 |
| API key creation / deletion count | 認証導線の利用状況確認 |
| Search / list endpoint の request volume | インデックス、DB、pagination 改善候補の把握 |
| OAuth / integration access to non-GA features | 非GA導線が誤って使われていないか確認 |

## 一次対応手順

1. 影響範囲を分類する。
   - 全体障害: health / version / sign-in / public docs がまとめて失敗
   - API 部分障害: GraphQL / REST の特定 endpoint だけ失敗
   - 認証障害: sign-in、JWT verify、API key verify、service auth が失敗
   - worker 障害: webhook / sync operation / queue / DLQ が詰まる
   - storage 障害: S3 / Parquet / bucket IAM が失敗
2. 直近 60 分の deploy、env / secret 変更、provider 障害、traffic spike を確認する。
3. Sentry issue、既存 runtime logs、trace、txcloud deployment、GitHub deployment / workflow を同じ incident note にリンクする。
4. user impact がある場合は Linear project status を更新し、Critical は Slack / COO へ即時共有する。
5. rollback が必要な場合は [Library GA environment and secrets Runbook](specs/operations/library-ga-env-secrets-runbook.md) の rollback 手順に従う。
6. 一時復旧後、再発防止 issue を作成する。原因が不明なまま Done にしない。

## エスカレーション

| Severity | 目安 | 通知先 | 期待応答 |
| --- | --- | --- | --- |
| Critical | GA 中核機能の利用不能、データ破損疑い、認証全体障害、event loss 疑い | on-call owner, engineering lead, COO | 15 分以内に一次判断、30 分以内に status update |
| Warning | 部分的な失敗、性能劣化、retry 増加、単一 org 影響 | on-call owner, feature owner | 当日中に対応方針を決定 |
| Info | 利用量変化、単発 transient error、改善候補 | feature owner | 週次レビューで確認 |

## GA 後 1 週間の運用リズム

### Day 0: GA 当日

| 時点 | 実施内容 | 記録先 |
| --- | --- | --- |
| GA 直後 | health / version / sign-in / public docs / GraphQL read / GraphQL write / REST read を確認 | Linear project update |
| 30 分後 | Critical alert、Sentry new issue、runtime error、auth failures を確認 | Linear comment or project update |
| 2 時間後 | traffic、latency、GraphQL / REST errors、worker backlog を確認 | Linear project update |
| 営業終了前 | 当日 summary、open risk、翌朝確認項目を記録 | Linear project update |

### Day 1-2

- 1 日 2 回、午前と営業終了前に operational review を行う。
- 確認項目は API availability、p95 latency、GraphQL / REST error top 5、auth failure trend、webhook / sync operation failure、Sentry new issue、public docs synthetic result。
- Critical / Warning が発生した場合は、原因、影響範囲、暫定対応、恒久対応 issue を Linear project update に記録する。

### Day 3-7

- 1 日 1 回、営業終了前に operational review を行う。
- 7 日間 Critical がなく、Warning が未対応のまま増えていない場合、GA 初週 watch を終了し通常運用へ移行する。
- watch 終了時に次をまとめる。
  - 発生した incident / alert
  - false positive / missing alert
  - latency / error rate の baseline
  - 追加すべき dashboard / runbook / automated check

## Linear status update cadence

| タイミング | 内容 |
| --- | --- |
| GA 当日 | GA 開始、初回 smoke 結果、Critical / Warning 有無 |
| Day 1-2 | 午前と営業終了前の health summary |
| Day 3-7 | 1 日 1 回の health summary |
| Critical 発生時 | 30 分以内に incident status、以後 30-60 分ごとに更新 |
| Warning 継続時 | 1 日 1 回、解消予定または follow-up issue を記録 |
| Watch 終了時 | baseline、残課題、通常運用へ移行する判断を記録 |

## 一次検知経路

- Deploy failure: `.github/workflows/deploy-api.yml` が txcloud manifest apply、build trigger/watch、temporary migration bridge、Library API health/version smoke、planet-library sign-in smoke を実行する。失敗時は GitHub Actions の failed run と通知が一次検知になる。PLT-1954 完了後は migration bridge を txcloud 側の deploy hook に置き換える。
- Health failure: Library API health/version checks use `https://library-api.txcloud.app`, and public web checks use `https://planet-library.txcloud.app/sign_in`. Consecutive failures should be handled as the primary availability signal.
- Runtime error spike: Sentry production project と既存 runtime logs を確認する。Sentry Team plan で OTEL 直接送信が使えるため、PLT-1680 では CloudWatch log metric filter / alarm の新規作成はしない。全 backend の OTEL + Sentry OTLP exporter 導入は PLT-1696 で扱う。

## Dashboard 最低構成

1. API overview: request count、5xx、4xx、p50 / p95 / p99 latency。
2. GraphQL overview: operation name、error code、resolver latency、top failing fields。
3. Auth overview: sign-in success / failure、JWT verification failure、API key verification failure、401 / 403 rate。
4. Public docs overview: synthetic check status、public route latency、404 / 403 trend。
5. Worker overview: webhook received / failed / rejected、sync operation success / failure / timeout、queue depth、DLQ depth。
6. Storage overview: S3 request errors、Parquet write failures、access denied、object count trend。
7. Sentry overview: new issues、regressions、affected users / orgs、release tag。

## 運用上の注意

- GraphQL は HTTP 200 でも `errors` を返すため、HTTP status だけを成功条件にしない。
- 非GA外部連携の NoOp / experimental path は GA 成功指標に含めない。
- secret 値、bearer token、API key、OAuth code は logs / Linear / GitHub comment に貼らない。
- user impact が未確定でも、Critical 条件に達した場合は先に status update を出す。
- false positive は alert を無効化せず、条件や判定窓を調整して記録する。
