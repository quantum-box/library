# Changelog

## 2026-09-08 - アーティファクトを作るスキル

- plugin に `library-artifact` スキルを足した。これまで `library` スキルには
  「Library に保存する」手順しか無く、肝心の「サンドボックスの中で成立する
  HTML をどう書くか」がどこにも無かった。plugin の利用者は Claude 側の
  artifact 用スキルを持っていないので、書けないまま保存手順だけ渡していた。
- 描画枠の制約を明文化した。`allow-same-origin` が無いので localStorage と
  cookie は例外になり、`allow-forms` / `allow-popups` / `allow-downloads` /
  `allow-modals` も無いのでフォーム送信・`target="_blank"`・ダウンロード・
  `alert` は黙って何も起きない。リンクは枠自身を置き換えるので戻る道が無い。
- `srcdoc` は埋め込み元の CSP を継承するため、desktop シェルではインライン
  script が動かない前提で書く。JS 無しでも読める HTML にして、JS は並べ替えや
  折りたたみの上乗せに留める。
- `library` スキルの artifact 節は新スキルへの案内に畳んだ。同じ手順を 2 箇所に
  置くと必ず片方が古くなるため。plugin は 0.5.0。

## 2026-09-08 - HTML アーティファクトの全画面表示

- Html Property のプレビューに全画面ボタンを足した。artifact は 1 ページ丸ごと
  なのに、記事カラムの中の固定高（ページで 560px）に押し込まれていて、縦の
  ドラッグリサイズしか逃げ道が無かった。
- ブラウザ本来の Fullscreen API を使う。Esc と OS 側の終了操作がそのまま効き、
  ボタンの表示は click ではなく document の状態に追従する。
- Code タブと、値が空のときはボタンを出さない。拡大するものが無いため。
- 共有リンクの閲覧ページ (`/s/<token>`) も同じ経路なので一緒に効く。
- あわせて、HTML アーティファクトを既定でその領域いっぱいに開くようにした。
  artifact は 1 ページ丸ごとなのに記事カラムの固定高に収まっており、窓の
  大半が余っていた。record ページではタイトル・プロパティ・添付を折りたたみ
  1 行に畳んで残す（消していない）。共有ページと公開ページは畳まず、artifact
  だけを出す — 受け取った人に渡したのは document であって record カードでは
  ないため。
- artifact 判定 (`isArtifactHtml`) を `RecordBodyEditor` から
  `lib/libraryTable/bodyProperty.ts` に移した。ページがレイアウトを決めるのに
  BlockNote ごと読み込まずに済ませるため。値が Markdown 方言の Html Property
  は従来どおり記事カラムのまま。

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
  `GET /v1beta/share/{token}` の応答も org / repo / repo 名を含まない。
- リンクの発行・失効は `library:UpdateRepo` を要求する。読めることと
  外に渡してよいことは別なので、read の関門を使い回さない。
- public repo へのリンク発行は拒否する。`/public/<org>/<repo>/<data_id>` が
  既に匿名で同じページを出しており token は access を足さないのに、後で
  private に戻したときだけ生き残って、その変更が閉じたかったものを開けたまま
  にするため。
- 不明な token・失効した token・消えた document・消えた repo は、すべて同じ
  404 本文を返す。どれを持っているかを保持者に見分けさせないため。
- REST の `PropertyResponse` に `options` を追加した。Select / MultiSelect の
  値は option の id だけを持つので、これが無いと読み手は `op_...` しか描け
  ない。`GET /properties` にも同じ穴があったので一緒に塞がる。

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
