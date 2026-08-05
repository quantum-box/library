# Library candidate migration hook

## 概要

Library API の production deploy は、独立した
`lambda-library-api-migrate` を `preDeploy.lambdaInvoke` で呼び出している。
この Lambda は warm invocation で tracing subscriber を再初期化して panic し、
修正版 API の production 昇格を停止させた。

専用 migration Lambda は廃止し、Tachyon が production Alias を昇格する前の
candidate lifecycle hook から、candidate API 自身に組み込まれた migration
endpoint を実行する。

関連: PLT-1954、PLT-2398、Tachyon ADR-0023。

## スコープ

- candidate API に bearer 認証付き deploy migration endpoint を追加する。
- `tachyon.yaml` を candidate `postDeploy.command` hook へ切り替える。
- `lambda-library-api-migrate` build target と専用 build script を削除する。
- migration 成功後だけ production Alias が昇格することを本番で確認する。

## 非スコープ

- 旧 AWS Lambda resource の即時削除。
- migration rollback の自動化。
- Tachyon の hook phase 名や実行順序の変更。

## 設計

[design.md](./design.md) および
[ADR-0007](../../../../../specs/decisions/ADR-0007-library-candidate-migration-gate.md)
を参照する。

## 検証

- deploy hook bearer 認証の unit test。
- `library-api` の format、clippy、unit test、build。
- production manifest dry-runと、merge後の実Build/deployment。
- candidate migration hook、deployment、`/version` の確認。
- Playwrightによるテストユーザーログインと組織作成。

## 完了条件

- deployment が `lambda-library-api-migrate` を参照しない。
- candidateでmigrationが成功した後にだけproductionへ昇格する。
- productionで組織作成が成功する。

## 実装完了時点のfollow-up

Ready PRの実装とローカル検証は完了した。本番candidate hook、Alias昇格、
authenticated組織作成は、mainへのmerge後に実Buildで確認する。
