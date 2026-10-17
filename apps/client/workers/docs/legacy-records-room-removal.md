# 旧共有 records room の削除手順

対象：Durable Object `PHOTON_SYNC_ROOMS`（クラス `PhotonSyncRoom`）の、名前
`tenant:library:workspace:library-default:records` の room に保存されたデータ
（スナップショットと更新の記録）。

**この手順は削除を実行しない。実行するかどうか、いつ実行するかは担当者が判断する。**

## 背景

- この room は、既定ワークスペースの全アカウントが共有していた records 用の Yjs room。
  約 15MB に肥大しており（生きた中身は約 1KB、残りは同じ値の書き直し履歴）、参加の
  たびに全履歴（約 700 件）が送られていた。別アカウント（smoke-test の組織）のレコード
  情報も履歴に含まれている。
- 利用者ごとの room と `/ws` のチケット認証（`workers/sync/src/room_auth.rs`）が
  デプロイされた後は、どのチケットもこの room を指せない。誰も接続・書き込みできず、
  保存データが残っているだけになる。
- Cloudflare には、1 つの Durable Object の保存データだけを消す API がない。そのため
  ワーカーに、この 1 つの room だけを扱う保守用ルート
  （`workers/sync/src/maintenance.rs`）を用意してある。

保守用ルート：

| 操作 | リクエスト | 内容 |
|---|---|---|
| 確認 | `GET /internal/sync/legacy-room` | サイズと件数だけを返す。中身は返さない。 |
| 削除 | `DELETE /internal/sync/legacy-room`（`x-confirm-room` ヘッダーに room 名） | 全行とアラームを消し、接続中のソケットを閉じる。削除前後の確認結果を返す。 |

- シークレット `PHOTON_SYNC_MAINTENANCE_TOKEN`（32 文字以上）が設定されていない間、
  このルートは存在しない（404）。
- どの操作にも `Authorization: Bearer <PHOTON_SYNC_MAINTENANCE_TOKEN>` が必要。
- 削除対象の room 名はコードに固定されている。ほかの room（利用者ごとの room を含む）は
  このルートからは触れない。

## 1. 事前確認

1. 利用者ごとの room と `/ws` 認証の変更が、ワーカー（`library-client-sync`）と
   クライアント（library v2 の Web / desktop）の両方で本番に出ていること。
2. 本番の Web クライアントが `POST https://library-client-sync.quantum-box.workers.dev/ws/ticket`
   を呼び、`/ws?room=tenant:library:workspace:library-default:user:<自分のID>:records&ticket=...`
   に接続していること（ブラウザの開発者ツールの Network で確認できる）。
3. 旧 room に書き込みが来ていないこと。下の「確認」を数分あけて 2 回実行し、
   `updates.nextSeq` が増えていないこと、`sockets` が 0 であることを確かめる。
4. （任意）`wrangler tail library-client-sync` で `sync_open_refused`（status 401）が
   出ていれば、古いクライアントがまだ旧方式で接続を試みている。削除の可否には影響しない
   （どのみち旧 room には入れない）が、利用者への案内の要否の判断材料になる。

## 2. バックアップの要否

**推奨：バックアップは取らない。** 理由：

- レコードの正本は library-api（Photon Engine）にある。この room はリアルタイム表示用の
  Yjs 文書で、正本ではない。
- 生きた中身（約 1KB）は、各利用者の端末の IndexedDB（利用者ごとのキー）と、新しい
  利用者ごとの room に、すでに同じものがある。
- 履歴には別アカウントのレコード情報が含まれている。複製を作ると、消したい情報を
  別の場所に広げることになる。

どうしても残す場合は、この手順の外で、保存先・保存期間・閲覧者を決めてから行うこと。
保守用ルートには中身を書き出す機能はない（意図的に付けていない）。

## 3. 実行方法

以下は、担当者が削除を決めた後に実行する。

1. 保守用トークンを作る（手元の端末で。値はチャットやチケットに貼らない）。

   ```bash
   export PHOTON_SYNC_MAINTENANCE_TOKEN="$(openssl rand -base64 48 | tr -d '\n')"
   ```

2. 本番ワーカーにシークレットとして渡す。ワーカーの設定は Tachyon が `tachyon.yaml` の
   `library-client-sync` から生成しているので、Tachyon のシークレットとして保存する。

   ```bash
   printf '%s' "$PHOTON_SYNC_MAINTENANCE_TOKEN" | tachyon env set library-client-sync --config tachyon.yaml --tenant-id tn_01j91h09tpj5ehwbwfwfxpak2b --secret PHOTON_SYNC_MAINTENANCE_TOKEN --value - --target production
   ```

   - このコマンドは手元の `tachyon.yaml` にも参照を書き足すが、書き足し方が崩れることがある。
     手元の差分は戻し、宣言は `library-client-sync` の `environments.production.envVars` に
     `type: credential` で手で書いて PR にする。
   - 保存したシークレットは、次の本番デプロイでワーカーに渡る（宣言がなくても渡ることがあった）。
     `GET /internal/sync/legacy-room` が 404 から 200 に変わったことで確かめる。
   - Tachyon を通さずに直接入れるなら
     `npx wrangler secret put PHOTON_SYNC_MAINTENANCE_TOKEN --name library-client-sync`。
     この場合も、後片付けは手順 5 と同じにする。

3. 確認（読み取りだけ）。`empty: false` で、サイズが想定（約 15MB）に近いことを見る。

   ```bash
   curl -sS -H "Authorization: Bearer $PHOTON_SYNC_MAINTENANCE_TOKEN" https://library-client-sync.quantum-box.workers.dev/internal/sync/legacy-room
   ```

   応答の形（数値は説明用の例で、実際の値ではない）：

   ```json
   {"room":"tenant:library:workspace:library-default:records","empty":false,
    "snapshot":{"seq":650,"byteLength":15000000,"chunks":153,"updatedAt":"..."},
    "updates":{"oldestSeq":651,"nextSeq":700,"waiting":49},
    "alarm":null,"sockets":0}
   ```

4. 削除。`x-confirm-room` に room 名を正確に入れないと 400 で拒否される。

   ```bash
   curl -sS -X DELETE -H "Authorization: Bearer $PHOTON_SYNC_MAINTENANCE_TOKEN" -H "x-confirm-room: tenant:library:workspace:library-default:records" https://library-client-sync.quantum-box.workers.dev/internal/sync/legacy-room
   ```

   応答の `before` が削除前、`after` が削除後の確認結果。

5. シークレットを外す。**宣言を消すだけでは足りない。** Worker に一度渡ったシークレットは、
   デプロイでは消えない（`wrangler deploy --help` も、デプロイはシークレットを削除しないと
   説明している）。宣言を消しても、Tachyon に保存したシークレットが残っていても、ルートは
   使えるまま、または次のデプロイでまた使えるようになる。次の順に行う。

   1. `tachyon.yaml` の宣言を、同じ名前の**普通の値（番兵）**に置き換える PR を出してマージ・
      デプロイする。プレビューの `EXTERNAL_SYNC_SCANNER_TOKEN` と同じやり方で、普通の値で
      上書きすると、渡っていたシークレットが外れる。値は 32 文字未満にする（保守用ルートは
      32 文字未満のトークンを無効として扱い、404 を返す）。

      ```yaml
      # Plain sentinel removes the maintenance secret binding; the route stays 404.
      - name: PHOTON_SYNC_MAINTENANCE_TOKEN
        value: "disabled"
      ```

   2. Tachyon に保存したシークレットを消す（以後のデプロイで、また渡らないようにする）。
      `tachyon env list` の通常の出力は値を表示するので使わない。`--json` から ID・キー・
      対象・シークレットかどうかだけを取り出して、消す ID を確かめる。

      ```bash
      tachyon env list library-client-sync --config tachyon.yaml --tenant-id tn_01j91h09tpj5ehwbwfwfxpak2b --json \
        | jq -r '.[] | select(.key == "PHOTON_SYNC_MAINTENANCE_TOKEN") | [.id, .key, .target, .is_secret] | @tsv'
      tachyon env delete library-client-sync <上で表示された ID> --config tachyon.yaml --tenant-id tn_01j91h09tpj5ehwbwfwfxpak2b
      ```

   3. デプロイ後、`GET /internal/sync/legacy-room` が 404 を返すことを確かめる。
      `wrangler` で直接入れた場合は、代わりに
      `npx wrangler secret delete PHOTON_SYNC_MAINTENANCE_TOKEN --name library-client-sync`
      でもよいが、番兵の PR も出しておくと、後から誰かが同じ名前で渡し直すのを防げる。

## 4. 確認方法

1. 手順 3-4 の応答で `after.empty` が `true`、`after.snapshot` と `after.updates` が `null`。
2. シークレットを外す前に、もう一度「確認」を実行して `empty: true` を確かめてもよい。
3. 番兵の PR のデプロイ後、`GET /internal/sync/legacy-room` が 404 を返すこと。
4. Web クライアントで /home を開き、表示とリアルタイム同期（サイドバーの接続表示）が
   変わらないこと。利用者ごとの room は削除の対象外なので、影響はないはず。
5. `wrangler tail library-client-sync` に `sync_maintenance`（`action: purge`、
   status 200）が 1 件出ていること。

## 5. 切り戻し

- **削除前（手順 3-3 まで）**：保存データには何も変更していない。シークレットを外す
  （手順 3-5）だけでよい。
- **削除後**：保守用ルートには復元機能がない。この room は正本ではなく、どのクライアントも
  もう読まないため、通常は復元の必要がない。
  - 最後の手段として、SQLite バックエンドの Durable Object には Cloudflare の
    Point-in-Time Recovery（過去 30 日のある時点への巻き戻し。JavaScript の
    `ctx.storage.getBookmarkForTime()` と `onNextSessionRestoreBookmark()`）がある、と
    Cloudflare のドキュメントは説明している。このワーカーが使う `worker` crate 0.8.5 は
    この API を公開していないため、使うには別途コード変更とデプロイが必要。実行前に
    最新のドキュメントで、対応状況と保持期間を確認すること。
- **利用者ごとの room 自体を取りやめたい場合**：この削除とは無関係。クライアントと
  ワーカーの変更を revert してデプロイする。ただし旧 room を消した後に revert すると、
  旧 room は空から始まる（各端末の IndexedDB から中身が書き戻される）。
