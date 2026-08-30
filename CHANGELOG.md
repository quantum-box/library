# Changelog

## 2026-08-30 - Library CLI と MCP tool の追加

- `library` CLI (`apps/cli`) を追加。org / repo / data / property / source を
  端末と自動化から操作できる。
- CLI の出力は `--json` で機械可読になる。表形式は人間向けで互換性を保証しない。
- CLI のプロパティ値は property 名と id のどちらでも指定でき、`@path` /
  `@-` でファイルと標準入力から読める。
- CLI の削除は、確認に答える端末が無い環境では prompt を通さず失敗する。
  CI や agent が誰も答えなかった確認でデータを失わないようにするため。
- MCP tool に `get_org` / `get_property` / `create_org` / `update_org` を追加。
- CLI の property 型に `date` を追加。MCP schema は受け付けるのに CLI から
  作れない型だった。
- MCP の HTTP+SSE transport を実装したが、既定では route を登録しない。
  Lambda は 1 インスタンスにつき同時 1 リクエストのため、stream を保持する
  インスタンスと `POST /messages` の届くインスタンスが必ず分かれ、応答を
  返せない。`LIBRARY_MCP_SSE_ENABLED=true` を明示した常駐環境でのみ有効。
  `POST /mcp` は無条件に登録され、影響を受けない。
- repo の username 変更に resource-level の write 権限チェックを追加。
  REST / GraphQL / usecase の 3 層すべてで認可されていなかった。

## 2026-05-06 - Library GA API release notes

- Added GA release notes for the Library CMS / Document OS API surface.
- Documented the GA REST API scope for repository, data, property, source,
  public docs, and API documentation endpoints.
- Clarified supported authentication methods, public access behavior, and the
  non-production status of development fallback tokens.
- Documented current GA rate-limit guidance and client retry expectations.
- Captured breaking changes and non-GA exclusions for Beta/Draft capabilities.
