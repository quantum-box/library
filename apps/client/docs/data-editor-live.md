# Data editor の Photon Live 連携

[PLT-4204](https://linear.app/issue/PLT-4204) の Library adapter。
Photon 自体の設計は [upstream](https://github.com/quantum-box/photon) を参照する。
Library の authority / ACL / checkpoint 境界は
[ADR-0006](../../../docs/specs/decisions/ADR-0006-library-photon-bounded-contexts.md) と
[ADR-0009](../../../docs/specs/decisions/ADR-0009-retire-photon-engine-server.md) に従う。

## 対象

data editor の RichText / Markdown 本文を BlockNote の Y.XmlFragment に接続する。
参加者のカーソルは Awareness で共有する。HTML artifact のソース、タイトル、
通常 Property は従来の編集経路を使う。

Live の状態は共同作業中の本文であり、Library への保存完了とは区別する。
保存は本文 Property だけの CAS checkpoint とし、他の Property やタイトルを
古い値に戻さない。競合は保存成功にせず、編集内容を保持して表示する。
通常 Property の更新で record version だけが進んだ場合は、本文が直前の保存内容と
同じことを再確認してから次の checkpoint を送る。本文自体が外部で変更された場合は
自動的に上書きしない。

## 認可とルーム

ブラウザは利用者の認証情報を付けて Library Live adapter にセッションを要求する。
adapter は Library API で対象データの存在、Property、書き込み権限を確認する。
ルームは API が返す tenant / database / data / property の識別子から決める。
汎用の `/ws?room=` と data editor の private room は別の binding に置く。

WebSocket URL には短命の ticket のみを入れる。再接続時にも認可を受け直す。
初回に canonical 本文から作る Yjs seed はサーバが一つだけ受け付ける。
後から入ったクライアントは自分の seed を混ぜず、Photon の snapshot を使う。

## 有効化前の条件

新しい環境で有効にするときは、まず
[Record patch decision UoW の導入条件](../../../docs/specs/operations/record-patch-decision-uow.md)
に従って PropertyValue backfill / parity と dual-write の準備を完了させる。
`legacy_only` のまま CAS mutation port を公開しない。

条件がそろった環境で次を設定する。

| 場所 | 設定 |
|---|---|
| Library API | `LIBRARY_PHOTON_LIVE_ENABLED=true` と、検証済みの dual-write `PROPERTY_VALUE_STORAGE_MODE` |
| Worker | `PHOTON_LIVE_ENABLED=true`、`PHOTON_LIVE_API_BASE_URL`、`PHOTON_LIVE_ALLOWED_ORIGINS` |
| Client build | `VITE_LIBRARY_DATA_LIVE_URL` に Worker の HTTP(S) origin |

Worker の Live用 Durable Object binding は `PHOTON_LIVE_ROOMS` と
`PHOTON_LIVE_TICKETS`。許可Originは実際に使うWeb / Tauriシェルに合わせる。
利用者Bearerを固定のサービス権限に置き換えない。

デスクトップの配布ビルドは `.env.production` の本文Live URLを読み込む。
Tauriの本番CSPには同じWorkerの `https:`（session）と `wss:`（接続）の
両方を許可する。Workerの本番OriginはWebの2種類に加え、macOS/Linuxの
`tauri://localhost` とWindowsの `http://tauri.localhost` のみを完全一致で許可する。
`null`、任意のlocalhost、ワイルドカードは許可しない。

Liveは補助機能であり、接続待ち・失敗中も本文の表示・入力・通常保存を続ける。
Liveを有効にするために入力を待たせたり、入力中のエディタを差し替えたりしない。
同時編集の検証では接続済みの本文Liveと、通常保存へのフォールバックを分けて確認する
（オンライン人数だけでは本文Liveの証明にならない）。

### 接続前の入力と再参加（2026-09-23）

以前は接続前に1文字でも入力すると、そのエディタは開いている間ずっと通常保存に
なった。通常保存は `record_version` を進めずに本文を変えていたため、次の再接続で
ルームに誰も入れなくなり、相手の編集が反映されない不具合になっていた
（本番で再現。APIとWorkerの対応は末尾の節）。

クライアントは `LiveBodySession`（`src/lib/photonLive/liveBody.ts`）で本文を扱う。

- エディタは最初から協調編集モードで作り、ページが読んだ本文から作ったローカルの
  下書きY.Docに結びつける。ルームの準備ができたら、BlockNoteのYjsプラグイン
  （sync / cursor / undo）をルームのY.Docへ付け替える。エディタは再マウントしない。
  内容が同じなら再描画後もカーソル位置は変わらない。IME変換中は付け替えない。
- 付け替え時の判定（比較はRichTextの生成ブロックIDと末尾の空ブロックを除いた内容で行う）:
  - 同じルーム（`room_generation` が同じ）への再接続: 旧Y.Docの差分を新しいY.Docへ
    Yjsのままマージする。providerの通常の再接続と同じで、何も失わない。
  - 画面とルームが同じ、または画面に未保存の変更がない: ルームをそのまま採用する。
    参加しただけでは保存しない（採用した本文を書き戻さない）。
  - ルームが保存済みの本文のまま: 画面の本文をYjsの差分としてルームへ書き込む。
    接続前の入力は通常保存を経由せず、同じルームの参加者へそのまま届く。
  - 接続前の入力があり、ルームも変わっていた: ブロック単位の3方向マージ
    （`merge3.ts`）の結果をルームへ書き込む。片側だけの変更はその側を採用し、
    同じブロックを両側が変えた場合は両方残す（相手→自分の順）。
  - 外部で本文が変わった後（4410）に未保存の変更があった: 競合。自動では混ぜず、
    通常保存もしない。次の入力で通常保存し、保存確定後にその本文で再参加する。
    ルームが判断できないまま10秒経った場合も競合として表示する。
- 接続前の入力はルームを待つ間（最大10秒）送らずに保持する。ルームが来なければ
  通常保存し、保存が確定した後に開いたルームで再参加する。保存前に認可された
  ルームは古い本文を持つため使わない。
- 参加中のルームが本文を運べなくなった場合（拒否されたcheckpoint、オンラインのまま
  15秒以上つながらない）は、新しいルームを開いて同じ判定で再参加する。
  再試行可能な失敗（認可のtimeout・5xxなど）ではLiveを諦めない。同じ本文の
  checkpointが続けて拒否された場合（大きすぎる本文など）は通常保存に切り替える。
  参加を拒否された回数と再試行の間隔は、ルームに参加できた時点で数え直す
  （参加できた後の単発の拒否が積み重なってLiveを諦めることはない）。
- `CHECKPOINT_RETRY` は同じoperation_id・version・本文をbackoff後に再送する。
- ページが非表示になったとき・`pagehide`・アンマウント時は、デバウンス中の入力を
  確定し、ルームがあればcheckpointを送る。保持中の入力は通常保存する。
  離れるページのルームは最後の本文の確認応答を最大5秒待ち、届かなければ
  （または切断中なら）通常保存する。
- モバイルのブラウザは非表示のページを `pagehide` なしで破棄することがあるため、
  非表示になった時点の通常保存（保持中の入力・利用不可などの間の入力）も
  `keepalive` で送り、保存キューに並ばずその場で送る。応答待ちの通常保存を
  追い越した場合は、両方の完了後に同じ保存をもう一度順番に送り、その完了を
  もって保存済みとする（ページが生きていれば最新の本文が最後に書かれ、
  それまでルームを開き直さない）。
  ブラウザが `keepalive` を拒否した場合（ページ全体で64KiBの枠を超えたときなど）は
  通常のリクエストで送り直す。GraphQLの更新が無いAPI（404/405/501）と一度
  分かったら、`keepalive` の保存は最初からRESTで送る（GraphQLの応答を待ってから
  切り替える時間が、離れるページには無いため）。`keepalive` の保存は、アクセストークンの更新時期に
  入っていても、期限内であれば手元のトークンでその場で送る（更新の往復を待つ間に
  ページが破棄されると保存自体が始まらないため）。
- ルームを使っている間は、非表示になってもcheckpointをすぐ送るだけで通常保存は
  しない（再接続中でも同じ）。通常保存はバージョン条件なしの後勝ちで、非表示の
  たびに正規の本文を変えると、相手が保存した内容を古い本文で上書きし、タブを
  切り替えるだけでルームが全員分作り直される。接続中の入力はYjsの差分として
  ルームに届いている。切断中の入力はページ内にだけあり、再接続時にマージされる。
  前のcheckpointの応答待ちで送れなかった本文は、デバウンスを待たずにその応答と
  同時に送る。停止しただけのページは、再開して再接続・応答を受けた時点で自分で送る。
  再接続中（切断中）に非表示になった場合は、ルームが最後に確認したrecord versionを
  条件にして、APIのLive checkpoint（`/live/checkpoint`、本文だけのCAS）へ
  `keepalive` で直接送る。その後に誰かが保存していればバージョンが合わず何も
  書かれないので、相手の保存を上書きしない。同じページのタイトル・プロパティ保存も
  record versionを進めるため、この保存は保存キューを通し、自分の書き込みが
  作ったバージョンの分だけ条件を進める（他人の書き込みのバージョンは含まれない）。
  自分の保存が送信中・待機中なら、まず送り、それらの完了後にもう一度送る。
  書き込めた場合、ページが戻ると
  ルームのcheckpointは同じ本文として確定する。相手にルームだけの未保存の入力が
  あれば、次の参加でルームが作り直され、その人には競合として表示される。
  既知の制約:
  - 応答が届く前にページが破棄されると、最後の本文はルームのYjsの状態には残るが、
    Libraryへの保存はルームで次に誰かが入力したときになる。それまでに正規の本文が
    外部で変わる（ルームが作り直される）と失われる。
  - 切断中（再接続待ち・オフライン）の入力は、上記の条件付き保存が届かない
    （オフライン、またはその間に誰かが保存した）まま非表示のページが破棄されると
    失われる。オンラインのまま15秒つながらなければ通常保存に切り替わるが、
    オフラインの間はルームのままで保存されない。
- bfcacheに入らない `pagehide`（タブを閉じる・再読み込み）では待てないため、
  ルームの確認応答がない本文を `keepalive` 付きで通常保存する。この保存は
  保存キューに並ばずその場で送り、まだ送られていない先行の保存はこれで
  代える（本文の保存はページが持つレコード全体を送るので、先行の変更も含む）。
  ルームを使わず通常保存している間（利用不可・競合・参加待ちの猶予切れ）も、
  デバウンス中の最後の入力は `keepalive` で送る。最新の本文の通常保存が
  まだ応答待ちなら、同じ本文を `keepalive` で送り直す（同じ本文なので
  どちらが後に着いても結果は変わらない）。
  既知の制約: 古い本文の通常保存が送信済みのまま、より新しい本文を
  `keepalive` で送ると、サーバーでの適用順は保証されない。通常保存は
  バージョン条件なしの後勝ちなので、古い方が後に適用されると新しい本文が
  上書きされうる。ページが生きていれば上記の再送で最新の本文に戻るが、
  閉じたページでは戻らない。確実に防ぐにはAPI側で書き込み順を判定する仕組み
  （バージョン条件付き保存や、ページごとの送信順の記録）が要る。
- 付け替えの再描画は編集として扱わない。付け替えるとundo履歴は新しいルームから
  やり直しになる。

`DataEditorPage` の本文保存は、保存が確定したら `true`、失敗したら `false` で
resolveするPromiseを返す。セッションはこれで「保存済みの本文」を更新する。

## PR301 の隔離Preview

`wrangler.preview.jsonc` は `library-client-live-pr301` 専用で、本番とは別の
Durable Object namespace を作る。既定では無効。公開時に CLI の `--var` で
`PHOTON_LIVE_ENABLED:true`、`PHOTON_LIVE_API_BASE_URL:<PR専用API origin>`、
`PHOTON_CLOUD_ENGINE_BASE_URL:<同じAPI origin>`、
`PHOTON_LIVE_ALLOWED_ORIGINS:<実際のPages Preview origin>` を明示する。
先に `wrangler deploy --config wrangler.preview.jsonc --dry-run` を確認する。

APIの `LIBRARY_PHOTON_LIVE_PREVIEW_REQUIRE_EMPTY=true` は、候補Lambdaの
migration gateでPR専用DB名を検証し、migration後の `data` が0件であることを
DB側のCOUNTで確認する。既存レコードがある環境では候補の昇格を拒否する。
この空DBチェックは既存データのbackfill/parity検証を代替しない。

APIのLive設定・dual-write設定は、Tachyonの
`--target preview --branch feature/plt-4204-data-editor-photon-live` に限定する。
クライアントはPreview専用の `npm run build:preview` を使う。branch限定の
`LIBRARY_PREVIEW_API_BASE_URL` / `LIBRARY_PREVIEW_SYNC_WS_URL` /
`LIBRARY_PREVIEW_DATA_LIVE_URL` を最後にVite設定へ適用し、manifestの本番URLによる
上書きを防ぐ。3つをまとめて指定し、本番APIや異なるLive/Sync hostは拒否する。
未設定のPRではLiveを無効にする。
初回の空DB確認からdual-writeを維持し、その状態で作成した検証データを
継続利用する場合は、初期化用の空DBチェックを解除する。PR301では初回の候補昇格後、
全検証データを通常のdual-write作成経路で追加し、Live保存を実確認してから解除した。
既存データを持つ別環境のbackfill/parityを省略する用途には使わない。

## ローカル検証

`apps/client` で `npm run test:e2e:live` を実行する。
`playwright.live.config.ts` が Library API fixture（50063）、実際の Photon Worker
（8788）、client（5187）を起動する。`wrangler.live-test.jsonc` はローカル専用で、
公開用の設定ではない。各テストのAPI fixtureは新しいdatabase IDを発行するため、
過去のDurable Object状態を誤って再利用しない。

「接続前に入力した内容が、すでにルームにいる参加者へ届く」ケースは、
`page.route` で `/live/session` を保留して入力後に解放し、確実に再現する。

このテストは実際のLibrary DB、権限サービス、CASのDBトランザクションを
検証するものではない。それらはRust側と実環境のゲートで確認する。

検証では次の結果を分けて記録する。

- エディタと接続 adapter の unit tests / 型チェック
- Photon Durable Object を使ったローカルの2ブラウザ検証（Library API は fixture）
- 実際の Library API / DB に対する認可・CAS検証
- 本番設定・デプロイ後の認証済みブラウザ検証

DB migration / backfill、Worker の公開、本番設定変更はローカル検証の成功から
推測しない。

## 公開Previewと検証結果（2026-09-05）

- 画面: https://pr301--library-client.txcloud.app
- API: https://pr301--library-api.txcloud.app
- Worker: `library-client-live-pr301`（本番と別のDurable Object）
- 検証データ: [共同編集テスト](https://pr301--library-client.txcloud.app/test-org/photon-live-check/data/data_01m1rbpv6tt0efg6zhfyhk0sx4)

初回の空DB確認付きAPI候補を昇格後、test-orgをPreviewへ取り込み、非公開の
`photon-live-check` リポジトリを作成。初期データ2件と共同編集テスト1件を表示した。
配信JavaScriptのAPI / Live / Sync URL、許可OriginのCORS 204、未認証401、
許可外Origin403を確認済み。同一アカウントの2タブで本文の双方向反映と
「本文を共同保存しました」表示、片方を閉じて再読み込み後の本文保持を確認し、
利用者からも共同編集できることを確認いただいた。

初回実装のCIは全項目成功。client 489件、Worker 14件、実Photon Worker + fixture
APIのLive E2E 5件、既存E2E desktop 22件 / mobile 3件が成功した。
Preview migration gateの局所テスト8件も成功。
レビュー修正後の最終検証はPR #301の検証欄を参照する。

別アカウント間と実macOS日本語IME候補ウィンドウの手動検証は未実施。
このPreview検証時点では本番のLiveは無効だった。本番の移行結果は次節を参照。

![RichText の共同編集と保存完了](screenshots/plt-4204/richtext-collaboration.png)

## 本番有効化（2026-09-05）

- 画面: https://planetlibrary.txcloud.app（https://library-client.txcloud.app も許可）
- API: https://library-api.txcloud.app
- Worker: https://library-client-sync.quantum-box.workers.dev
- API: `PROPERTY_VALUE_STORAGE_MODE=dual_write_legacy_read` と
  `LIBRARY_PHOTON_LIVE_ENABLED=true` を production のみに設定。
- Client: `VITE_LIBRARY_DATA_LIVE_URL` を production のみに設定。
  Previewは引き続き専用の設定・DBを使う。

本番の8スコープ、21レコードを対象に既存のPropertyValue backfill実装を実行した。
同一の悲観的トランザクションでdatabase ID順に親objectをロックし、全件dry-run、
48値の書き込み、全件parity検証を行い、すべて成功した場合だけcommitした。
commit後の独立した全件dry-runでも matched=48、missing=0、opaque=0、追加書き込み=0。
両検証のchecksumは `00041e760bda5b018b513cda4b39fa2acfab9be3fe4f2619b324209b3f224d97`。
件数は移行時点の値であり、その後の通常更新・検証データ作成は含めない。

承認を得て作成した非公開の一時Lambdaで移行し、検証後に削除した。
古い本番DB共有のPreview 10 aliasと無修飾API/dev Function URLは、
利用状況とPRの終了を確認してAWS_IAMに切り替えた。prod URLは維持している。
旧writerを戻すとparityを壊すため、復旧時にもlegacy-onlyで本番DBへ接続させない。

API build `bld_01m1rx8vxnsecvw9hmpcf5wch9` とClient build
`bld_01m1ry5xwbdj3br0wbxzqxr8w6` が成功した。
本番配信JavaScriptのAPI/Live URLと、許可Originのpreflight 204、未認証401、
許可外Origin 403を確認。

認証済みの同一アカウント2タブで、非公開の
`quantumbox/photon-live-production-check` の検証データ
`data_01m1ryepyxpnpnxada925f3spc` を編集した。AからB、BからAの反映、
編集したタブでの「本文を共同保存しました」、片方を閉じて再読み込み後の
両方の本文保持を確認した。別アカウント間・実macOS IMEの検証は未実施。

残件 [PLT-4267](https://linear.app/issue/PLT-4267): 相手側の編集を受信したタブに
「共同編集でエラーが発生しました」が残る。
そのタブから次の編集を行うと保存成功へ戻り、再読み込みでも本文は保持された。
保存成功の証拠と、この受信側のcheckpoint/表示問題は区別する。

### PLT-4267の修正

認可応答中に作業バージョンが進んだ場合、WorkerはDB書き込み前の拒否を
`live-error`の`code: CHECKPOINT_STALE`で返す。Clientは待機中の最新本文を
送信し直す。シリアライズ前の古い本文を新しい作業バージョンへ付け替えず、
保存済み本文との`live-conflict`は停止を維持する。

Worker内ではcheckpointだけを直列化し、実行中の保存要求を別参加者が
復旧対象として置き換える競合を防ぐ。Yjs更新とAwarenessはその待機対象にしない。
ローカルの実Workerでは認可750ms・保存1000msの遅延を加え、Markdown/RichTextの
両タブで保存完了することを確認した。本番反映はWorkerを先、Clientを後に行う。

PR #302マージ後のrunner契約バージョン不一致は、2026-09-06の再実行で解消した。
Client `bld_01m1s2n1d6hg7k3jfx6mbkk7cm`、API `bld_01m1s2qy6wrw8g1bywn1ng6k9n`
はいずれもmain `98d3d770227ff3d9aa917ee1d487fe0481a84bfd` の本番デプロイに成功。

緊急停止はClient/Worker/APIのLiveフラグを無効化して行う。
`PROPERTY_VALUE_STORAGE_MODE` はdual-writeを維持し、旧writerを再公開しない。
再有効化前に全件parityとcheckpointの保存を確認する。

## Worker: 外部書き込み後の世代交代とcheckpoint再試行（2026-09-23）

本番で、通常保存（GraphQL `updateData` / REST PUT、MCP、inbound sync）が
`record_version` を進めずに本文だけを変えたため、Workerが同じversionで異なる本文を
順序付けできず、ヘッダーなしの409を返し続けてルームへ再参加できなくなった。
APIはすべての書き込みでversionを進めるよう修正し、Workerは次のように動作する。

### 参加時の判定（`PhotonLiveRoom::fetch`）

ルームの本文hashとセッションのcanonical本文hashが異なる場合:

| セッションの `record_version` | 動作 |
|---|---|
| ルームより新しい、かつ保留中checkpointの本文と一致 | ACK喪失したcheckpointとして保存済みに確定（`live-saved` を配信）し、世代交代せず参加 |
| ルームより新しい（上記以外） | 未保存の編集があっても新しい世代へ交代 |
| ルームより古い（古いticket） | 409。古い本文へ戻さない。クライアントはセッションを取り直す |
| 同じ（本文だけ異なる） | 409。順序付けできないため。API修正後の新規書き込みでは発生しない |

世代交代では、基点ルームのpointerが実際に新世代を指した後にだけ、既存参加者を
close 4410で切断する。pointerを確定できなかった場合は参加者を切断しない。
旧世代のYjs状態と保留中checkpointは旧世代のstorageにそのまま残り、新世代へは
混ぜない。旧世代は後継を記録し、pointer移動直後に届いた参加要求を後継へ転送する。

新世代のメタデータは、世代名に含まれる `record_version` と本文hashから作る。
最初に届いたセッションからは作らない。pointer移動直後に古いticketが新世代へ
届いても、その古い本文で新世代を初期化しない（初期化すると正しいticketが
すべて拒否され、次のcanonical書き込みまで再参加できなくなっていた）。
基点ルームのpointerと旧世代の後継記録は、後継より古いticketを転送せず、その場で
409（`stale_ticket`）を返す。まだ誰も入っていない世代へ、それより新しいセッションが
届いた場合は、その世代を飛ばして次の世代へ交代する。

WebSocket upgradeの拒否はブラウザでは1006としか見えないため、Workerは拒否ごとに
`live_join_refused`（ルーム）/ `live_open_refused`（edge）をstatusと固定の理由で
ログに出す。ticket、session ID、本文、認証値は出さない。`wrangler tail` で確認する。

### checkpointの再試行（`code: CHECKPOINT_RETRY`）

API（`record_mutation_operations`）はoperation_idごとに判定を記録し、同じ
operation_id・expected version・本文・actorの再送には元の判定をそのまま返す。
Workerは保留中checkpointの本文とexpected versionをjournalから読み直して再送するため、
初回がcommit済みでACKだけ失われても、同じoperation_idの再送は元の
`record_version` で `live-saved` になる。

次の場合、Workerは予約を保持して `live-error` に `code: CHECKPOINT_RETRY` を付ける。

- API呼び出しのtimeout・通信失敗
- APIの401 / 408 / 429 / 5xx
- 2xxだが `record_version` がない応答
- APIの409後にcanonical本文を確認できない場合
- 認可やstorageの失敗（`Live checkpoint recovery failed`）

400 / 403 / 404 / 422など、同じ要求で同じ結果になる拒否は従来どおりcodeなし（終端）。
このときWorkerは予約を削除する（commitされないことが確定しているため）。
APIの409でも、canonical本文が保留中checkpointと一致し、versionが進んでいれば
保存済みとして確定し、`live-conflict` にしない。

#### 結果が未確定の予約（in doubt）

Workerが応答を受け取れなかった要求（timeout・通信失敗・5xx）は、API側でまだ
実行中で、後からcommitされる可能性がある。予約には送信時刻から2分
（`CHECKPOINT_IN_DOUBT`）の期限を記録し、その間は「認可で見た `record_version` が
予約時のまま」でも、commitされなかった証拠とはみなさない。別の参加者の
checkpointはこの予約を置き換えず、`CHECKPOINT_RETRY`（理由
`reservation_in_doubt`）を返す。予約の持ち主が同じフレームを再送すると、APIの
冪等性で結果が確定する（同じoperation_idの要求はAPI側で一意キーのロックにより
直列化され、commit済みなら元の判定が返る）。期限後は認可の結果で判断する。
401 / 408 / 429、およびAPIの409は、APIが適用しなかったことが確定した応答なので
未確定の印を外す。
APIに要求の期限はないため、2分は十分な余裕を見た上限であり保証ではない。

#### 本文を変えない書き込み（タイトル・プロパティの保存）

API修正後は、本文を含まない通常保存でも `record_version` が進む。canonical本文が
予約の基準にした本文（ルームの `body_hash`）のままversionだけ進んだ場合は競合に
しない。

- 送信中のcheckpointがAPIの409になった場合: 予約を削除して新しいversionを採用し、
  送信者へ `code: CHECKPOINT_STALE` を返す。APIはこのoperation_idに競合を記録した
  ため、同じIDでは保存できない。クライアントは最新本文を新しいoperation_idで送る。
- 保持中の予約がある状態で別のcheckpointが届いた場合: 予約を削除し、その
  operation_idを付けた `CHECKPOINT_STALE` を全参加者へ配信する（持ち主だけが反応し、
  新しいoperation_idで送り直す）。届いたcheckpointは新しいversionで続行する。
- 予約がなく、送られた本文がすでに新しいversionでcanonicalな場合（予約が置き換え
  られた後に自分のCASがcommitされた、または別の書き込みが同じ本文を保存した）:
  CASを送らずに保存済み（`live-saved`）として確定する。

クライアントの契約:

- `CHECKPOINT_RETRY`: 送信中のcheckpointを保持し、backoff後に**同じ
  operation_id・version・本文**のフレームをそのまま再送する。本文を再シリアライズしない
  （同じIDで内容が違うと終端の `Checkpoint operation was reused` になる）。
  同じ世代（`live-ready.room_generation` が同じ）への再接続後も同じフレームを再送する。
  再送が `CHECKPOINT_STALE` になった場合は既存どおり最新本文を新しいoperation_idで送る。
  別の参加者の予約が未確定の間は、新しいcheckpointにも `CHECKPOINT_RETRY` が返る
  （最大で約2分）。backoffは上限付きで、ソケットが開いている間は諦めない。
- `CHECKPOINT_STALE`: 全参加者へ配信されることがある。`operation_id` が自分の
  送信中checkpointと一致する場合だけ処理し、一致しなければ無視する。
- close 4410: canonical本文が外部で新しいversionへ変わり、ルームが新世代へ移った。
  送信中checkpointは保存済みとみなさない。未保存の編集は旧世代のstorageにも残るが
  再参加できないため、ローカルのY.Docの内容を破棄せず競合として扱う。
  新しいセッションを取得して接続し、
  新しい `room_generation` では旧Y.Docを適用せず、セッション本文から作り直す
  （`initialized: false` ならその本文でseedする）。
- ヘッダーなし409（ブラウザでは1006）: 新しいセッションを取得して再接続する。
