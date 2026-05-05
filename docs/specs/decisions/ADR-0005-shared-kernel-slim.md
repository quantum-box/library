# ADR-0005: Shared Kernel を最小型セットに制限する

## Status

Accepted (2026-05-05)

## Context

bakuure ERP redesign では、商品、価格、在庫、受注、発注、顧客、出荷、決済、レポート、会計、テナント、権限、API など複数の bounded context を扱う。全 context が同じ Rust workspace / monorepo に置かれる場合、便利さを理由に domain type を共通 package に移しすぎると、Shared Kernel が事実上の巨大 domain model になり、変更影響範囲と循環依存のリスクが増える。

Library でも `packages/value_object` が `TenantId`, `Money`, `DateRange` などの基礎型を提供している。一方で、PLT-942 の `GlobalIdMapping` のような Library 固有の概念まで Shared Kernel に入れると、bakuure と Library の境界が曖昧になる。

PLT-990 では、Shared Kernel に入れる型を 4-6 件程度に絞り、context 固有の型と実装依存を除外する方針を確定する。

## Decision

Shared Kernel は「全 context が同じ意味で使い、domain rule をほぼ持たない基礎型」だけを置く slim package とする。Shared Kernel は business workflow、repository、API client、usecase、policy decision を持たない。

Shared Kernel に入れてよい型は以下に限定する。

1. `TenantId` / `PlatformId` / `OperatorId`
2. `ActorId` / `UserId` / `ServiceAccountId`
3. `MoneyAmount` / `Currency`
4. `BusinessDate` / `DateRange` / `OccurredAt` 相当の時刻・期間 wrapper
5. `ExternalSystemCode` / `ExternalReference` 相当の外部参照 key
6. `Revision` / `SchemaVersion` / `RelationVersion` 相当の version marker

Shared Kernel に入れないものは以下とする。

- `Product`, `SKU`, `Order`, `Inventory`, `Procurement`, `Shipment`, `Payment`, `Refund`, `AccountingEntry` など業務 entity / aggregate
- `GlobalIdMapping` など Library context が所有する domain model
- `TaxRule`, `PriceRule`, `StockReservationPolicy`, `RefundApprovalPolicy` など業務判断を含む rule
- API client、GraphQL type、DB row、migration、repository trait
- UI 表示名、画面都合の enum、CSV/import/export 専用 DTO

循環依存の扱いは Rust workspace と TypeScript workspace で分ける。

- Rust: Phase 1 で `cargo metadata` ベースの CI gate を導入し、workspace package 間の path dependency cycle を検出する。
- TypeScript: context package が増えた時点で `dependency-cruiser` などの lint を検討する。
- PR review checklist で Shared Kernel 追加理由を必須化する。

Rust 側は workspace package graph が `Cargo.toml` で明確なため、最小の cycle gate を先に入れる。domain layer の細かい import rule は、context package 構成が固まった後に追加する。

## Consequences

### Positive

- Shared Kernel が context 固有の業務知識を吸い込まない。
- Library/bakuure 間の境界が保たれ、ADR-0004 の stub 方針と矛盾しない。
- 共通化する型の理由が明示され、便利さだけの移動を review で止めやすい。
- Rust workspace の循環依存は CI で検出できる。

### Negative

- context 間で似た型が一時的に重複する可能性がある。
- Shared Kernel に入れる前の review 判断が必要になり、初期実装は少し遅くなる。
- Rust の path dependency cycle は検出できるが、module-level import rule や TypeScript 側の循環はまだ検出対象外。

### Neutral

- 既存の `packages/value_object` はすぐに大規模分割しない。
- `Money<T>` の内部表現や decimal 精度改善は別判断とする。
- Library の `GlobalId` は Library context の ID とし、Shared Kernel の必須型にはしない。

## Alternatives Considered

### Shared Kernel に業務 entity を集約する

重複は減るが、ERP 全体の変更が一箇所に集中する。商品や在庫などの言葉は context ごとに意味が変わるため不採用。

### context 間の共通型を一切持たない

境界は最も強く保てるが、tenant、actor、money、date など全 context で同じ意味の型まで重複し、変換処理が増えるため不採用。

### Phase 1 から module-level の厳格な cyclic dependency lint を導入する

品質ゲートとしては有効だが、まだ package topology が安定していない。まず Rust workspace package 間の cycle gate に絞り、module-level rule は Phase 2 以降に送る。

## Follow-up

- Shared Kernel 追加時の PR checklist を `docs/specs/implementation-notes.md` に追記する。
- context package 境界が固まったら Rust の module-level dependency rule を追加する。
- TypeScript 側に context package が増えた場合は `dependency-cruiser` の導入を検討する。
- `packages/value_object` の既存型を、Shared Kernel に残す型と context 側へ戻す型に棚卸しする。

## References

- [PLT-942 task](../../tasks/in-progress/plt942-mdm-hub-phase1-registry/task.md)
- [Library workspace overview](../overview.md)
- [value_object package](../../../packages/value_object/src/lib.rs)
- [GlobalIdMapping domain](../../../apps/api/src/domain/global_id_mapping.rs)
- [Rust workspace cycle check](../../../scripts/check-rust-workspace-cycles.mjs)
