# Changelog

## 2026-09-08 - private repo の閲覧共有リンク

- private repo の Data 1 件を、Library アカウント無しで読める共有リンクを
  追加した。テナント外のレビュアーや顧客に artifact を渡すための経路。
- リンクは Data 1 件だけを開く。漏れてもその repo の他の document には
  届かない。
- token は SHA-256 だけを保存する。平文が存在するのは発行レスポンス 1 回きり
  で、後から取り出す手段は無い。失くしたら作り直して古い方を revoke する。
- 失効は削除ではなく `revoked_at` の記録。「このリンクはもう効かない」を
  owner が見られるようにするため。
- REST に `POST/GET /v1beta/repos/{org}/{repo}/data/{data_id}/share-links`、
  `DELETE /v1beta/repos/{org}/{repo}/share-links/{id}`、
  および無認証の `GET /v1beta/share/{token}` を追加。
- MCP tool に `create_share_link` / `list_share_links` / `revoke_share_link`
  を追加。artifact を private repo に置いたまま URL だけ渡せる。
- client に `/s/<token>` の閲覧ページと、data 画面の共有ダイアログを追加。
  閲覧ページは repo へのリンクを一切持たない。受け取った人はサインイン壁に
  当たるだけで、org と repo の名前自体も token が隠しているもののうち。
- リンクの発行・失効は `library:UpdateRepo` を要求する。読めることと
  外に渡してよいことは別なので、read の関門を使い回さない。

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
