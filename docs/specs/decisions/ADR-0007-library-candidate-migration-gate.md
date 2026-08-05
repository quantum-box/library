# ADR-0007: Library migrationはcandidate APIをproduction昇格前に実行する

## Status

Accepted (2026-08-05)

## Context

Library APIは独立したmigration Lambdaをproduction deploy前にinvokeしていた。
この構成ではAPIとmigrationのartifact、runtime、observability、warm lifecycleを
別々に管理する必要があり、migration Lambdaだけが古い状態で残った。

TachyonのLambda deployはcandidate Aliasを検証し、lifecycle hook成功後に
production Aliasを昇格する。

## Decision

Library migrationはcandidate APIに組み込み、Tachyonのcandidate lifecycle hookから
認証付きendpointを呼び出す。専用`lambda-library-api-migrate`はdeploy pathで使用しない。

Tachyon上のphase名は`postDeploy`だが、production Alias昇格より前に実行されるため、
Libraryのrelease gateとして扱う。migration失敗時はcandidateをproductionへ昇格しない。

## Consequences

- API codeとmigration setが同じBuild/Versionになる。
- 独立migration Lambdaのwarm lifecycleとartifact driftがなくなる。
- candidate Function URLに内部endpointが必要になるため、専用secret-backed bearer認証と
  credential非表示を必須とする。
- schema rollbackは自動化せず、migrationはactive Versionと後方互換に保つ。

## Alternatives Considered

- 独立migration Lambdaを継続する: 二重runtime管理が残るため不採用。
- 汎用`preDeploy.command`でrepository migrationを実行する: JobRunにsourceとBuild
  artifactがないため不採用。
- production APIでmigrationを実行する: 新しいmigration setを持たないため不採用。

## References

- [Task](../../src/tasks/completed/v1.11.3/library-candidate-migration-hook/task.md)
- [Design](../../src/tasks/completed/v1.11.3/library-candidate-migration-hook/design.md)
- Tachyon ADR-0023 Cloud App Lambda Alias promotion
