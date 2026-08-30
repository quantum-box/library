# ADR-0008: Library migration は candidate Lambda を production 昇格前に実行する

## Status

Accepted (2026-08-24)

## Context

Library API は migration を専用 Lambda に載せていた。production は
`hooks.preDeploy` から `lambda-library-api-migrate` を、preview は
`provisionedDatabase.migration.lambdaInvoke` から
`lambda-library-api-preview-migrate` を呼ぶ構成である。どちらの関数も
`sqlx::migrate!` で **自身のビルド時に** migration SQL を埋め込み、コードは
tachyon-apps の Terraform が器だけ宣言して out-of-band で配布していた。

この構成には二つの問題があった。

1. **artifact drift。** migration を追加してもその関数を再デプロイするまで反映
   されない。リポジトリを直しても、所有者が曖昧な関数を誰かが手で配るまで何も
   変わらない。2026-08-21 から 3 日間 preview が全滅した PLT-3861 は、まさに
   この形で長引いた。
2. **warm lifecycle。** 専用 migration Lambda は warm な実行環境を再利用し、
   global tracing subscriber の再初期化で 2 回目の invoke が panic した。成功した
   ビルドが promotion されない原因になっていた。

Tachyon の Lambda deploy は candidate Alias を作って検証し、lifecycle hook が
成功してから production Alias を昇格する。CourseBoard は既にこの candidate を
使って migration gate を実装している (courseboard-api)。

## Decision

Library migration は **candidate Lambda 自身** が実行する。Tachyon の
`hooks.postDeploy` が candidate alias を synthetic な API Gateway v2 イベントで
invoke し、`apps/api/bin/lambda.rs` の `DeployMigrationGate` がそれを検知して
`library_api::migrations::run_migration_gate` を呼ぶ。専用の
`lambda-library-api-migrate` / `lambda-library-api-preview-migrate` は deploy 経路
から外す。

gate イベントの判定は次の 3 点一致に限る。

- `routeKey: LIBRARY_MIGRATION_GATE`
- `rawPath: /`
- `requestContext.domainName: library-api-migration-gate.internal`

Tachyon 上の phase 名は `postDeploy` だが production Alias 昇格より前に走るため、
Library の release gate として扱う。migration が失敗すれば candidate は昇格せず、
現に serving 中の Lambda は無傷のままになる。

preview と production で適用するものは異なる。production は `library` と
`tachyon_apps_database_manager` の 2 データベース / 2 履歴、ADR-0049 の preview は
per-PR データベース 1 つに combined 履歴 (PLT-3328) を適用する。

## Consequences

- API コードと migration set が同じ Build / Version に乗る。artifact drift が
  原理的に起きない。
- 独立 migration Lambda の warm lifecycle 問題が消える。
- **内部 endpoint を公開しない。** 3 点一致は public Function URL から作れない
  組み合わせなので、認証付きの内部 HTTP endpoint を candidate Function URL に
  生やす必要がない。判定は `bin/lambda.rs` の単体テストが manifest と突き合わせ
  て固定しており、片側だけ改名しても静かに migration が止まることはない。
- serving 初期化は gate を呼ばない。壊れた migration 履歴は deploy を止めるもの
  であって、稼働中の API を落とすものではない。
- **Lambda の invocation timeout を明示的に上げる必要がある。** 既定は 30 秒で、
  library-api の 72 本の migration を fresh な per-PR データベースへ初回適用すると
  32.3 秒かかり超過する。library-api は enterprise の cargo_lambda app なので
  platform 側の scaling 設定は効かず、値は tachyon-apps の
  `cluster/n1-aws/library_api_lambda.tf` が持つ (900 秒)。
- schema rollback は自動化しない。migration は active Version と後方互換に保つ。

## Alternatives Considered

- **独立 migration Lambda を継続する。** artifact drift と warm lifecycle の問題が
  残るため不採用。PLT-3861 はこの構成が実際に長期障害を生んだ実例である。
- **candidate に認証付き HTTP endpoint (`POST /internal/deploy/migrate`) を生やして
  呼ぶ。** 同じ candidate gate を実現できるが、migration を起動できる endpoint が
  public Function URL に載る。専用 secret による bearer 認証で守る前提だったが、
  synthetic イベント方式なら公開面をそもそも作らずに済むため不採用。
- **汎用 `preDeploy.command` で migration を実行する。** JobRun には source も
  Build artifact もなく、per-PR TiDB は PrivateLink 専用で runner から到達不能
  (PLT-3561)。不採用。
- **production API で migration を実行する。** 稼働中の Version は新しい migration
  set を持たない。不採用。

## References

- [preview / production の migration 運用](../operations/library-preview-migrations.md)
- Tachyon ADR-0023 Cloud App Lambda Alias promotion
- Tachyon ADR-0049 per-PR preview database
- courseboard-api の migration gate (`bin/lambda.rs` / `src/migrations.rs`)
