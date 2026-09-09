# Changelog

## 2026-09-09 - iOS アプリの UI 修正

シミュレータ実機（iPhone 17 Pro / iOS 26）で一通り触って見つかった 7 件。

- **画面下 130pt が死んでいた。** Tauri は webview を window の *inner* size に
  合わせるが、iOS の `tao` はそれをセーフエリアとして返す。画面 874pt に対して
  webview は 778pt、しかも上端 y=0 に置かれるので不足分 96pt が全部下に溜まる。
  一方 `env(safe-area-inset-*)` は端末本来の 62/34 を返し続けるため、
  `index.css` の `body` padding が同じ領域をもう一度確保していた。
  `src-tauri/src/ios_webview.rs` で webview を親ビューいっぱいに張り直し、
  scroll view の自動 content inset を切って原因側を潰した。
- **アプリの枠を `body` から `#root` に移した。** Radix はメニュー・ダイアログ・
  ポップオーバーを開くたび `react-remove-scroll-bar` 経由で
  `body[data-scroll-locked] { position: relative !important; padding-top: … }`
  を注入する。`position: fixed` とセーフエリアの padding を `body` に載せて
  いたので、それが両方消えていた。アカウントメニュー（テーマ 3 + 言語 11 で
  約 600pt）を開くと body が popover の高さまで伸び、ページ全体が 244pt 上に
  ずれてアプリバーが Dynamic Island の下に隠れる、という形で表に出ていた。
  `#root` は誰も書き換えない。
- **背の高いメニューを画面に収めた。** Radix が測った空き高さで頭打ちにして
  スクロールさせ、`collisionPadding` にセーフエリアを渡した
  （`useSafeAreaInsets`。Radix は JS で位置を決めるので `env()` を読めない）。
- **ホーム画面だけモバイル対応が入っていなかった。** デスクトップのヘッダと
  擬似タブ列が電話でも出ており、シェルのアプリバーと合わせて「Library」も
  「新しいデータ」も 2 回ずつ描かれていた。`md` 未満で両方隠した。
- **HTML アーティファクトの全画面ボタンが iPhone で無反応だった。**
  iPhone に element fullscreen は存在せず `requestFullscreen` が生えていない。
  固定オーバーレイにフォールバックし、ツールバーごと画面を占有して戻る導線を
  残す（電話には Esc が無いため）。
- **`bundle.iOS.minimumSystemVersion` を 14.0 から 16.4 に上げた。**
  Tailwind v4 が出す `@property` / `color-mix()` / `oklch()` は Safari 16.4 以降。
  低い床は「入るが壊れて見える」ビルドを配ることになる。
- **横向きの電話がデスクトップレイアウトになっていた。** 幅 874pt が `md` を
  超えるため。`md` を `(min-width: 768px) and (min-height: 500px)` に再定義し、
  `MOBILE_VIEWPORT_QUERY` と `.detail-panel` も同じ線に揃えた。
- **リポジトリの「リンクをコピー」が `tauri://localhost` を配っていた。**
  `shareableUrl()` を通していない唯一のコピー経路だったので通した。

### 横方向のはみ出し

電話でアプリが横に滑る経路を潰した。ページ自体は絶対に pan せず、はみ出しは
必ずそれ用の pane の中で起きる、という線を引き直している。

- **HTML アーティファクトが横に滑っていた。** artifact は他人が書いた 1 枚の
  HTML で、電話向けには書かれていない。viewport meta を持たない文書と、
  デスクトップ幅の `pre` や table がフレームからはみ出す。
  `fitArtifactToFrame` が viewport meta と最小限の `max-width` 規則を
  **著者の記述より前に**差し込む（後から書いたものが勝つので上書きは自由）。
  iPhone 17 Pro の実機計測で、内側の文書幅が 757px → 386px（フレーム幅）に。
  自分で viewport を宣言している文書は幅について意思表示しているので触らない。
- **長いタイトルがカードを押し広げていた。** `body` に
  `overflow-wrap: anywhere` を敷いた。`break-word` ではなく `anywhere` なのは、
  要素の min-content 幅を縮めるのは `anywhere` だけで、flex / grid の item を
  画面外へ押し出すのがその min-content 幅だから。190 文字のタイトルで
  リポジトリのカードが 937px はみ出していた。
- **ブレークポイントでしか列を定義していない grid が max-content まで伸びて
  いた。** 暗黙の列は `auto` で、中に `truncate`（= `nowrap`）があると
  その max-content まで育つ。ホームの「最近のアクティビティ」は 402px の画面で
  1290px あった。`grid-cols-[minmax(0,1fr)]` を base に足した
  （home / 組織概要 / API キー / リポジトリ設定 / サインイン）。
  `grid-cols-[1fr_auto]` も `minmax(0,1fr)` に直した（`1fr` の min は auto）。
- 意図して横に流す帯（リポジトリタブ・ビュータブ・ボード・chat の table と
  コード）に `overscroll-x-contain` を付けた。横のドラッグが後ろのアプリに
  伝播しない。

## 2026-09-09 - content Property を UI から消せるようにする

- Repository 設定のプロパティ一覧で、名前が `content` のプロパティだけ削除
  ボタンが常に disabled だった。保護リストに名前がハードコードされていて、
  型に関係なく効いていたため。API 側は削除を許しており、UI だけの制限。
- `create_repo` は必ず RichText の `content` を作る。一方 client は本文に
  RichText を Html より優先するので、Html アーティファクト用の repo では
  `content` を消さないと artifact が本文にならない。UI からその repo を
  仕上げる経路が無く、MCP か CLI に降りるしかなかった。
- 保護するのはレコードのキーとタイムスタンプ (`id` / `name` / `createdat` /
  `updatedat`) だけにした。`ext_` のシステム Property と、client が型を
  知らない Property の読み取り専用扱いは従来どおり。

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
- 設計の指針も入れた。色・書体・レイアウトの計画を先に立てる、依頼の性質に
  応じて仕上げの強度を変える、繰り返す要素は 1 つの部品として揃える、読ませる
  ものは読み込み直後に全部見えている、といった話。Claude の artifact 向け指針
  とは 2 点で逆になる — Google Fonts と cdnjs は desktop の CSP に弾かれるので、
  書体はシステムスタックか data URI、ライブラリは使わず inline SVG を手で描く。
- `library` スキルの artifact 節は新スキルへの案内に畳んだ。同じ手順を 2 箇所に
  置くと必ず片方が古くなるため。plugin は 0.5.0、desktop は 0.1.36。

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
