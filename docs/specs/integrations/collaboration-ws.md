# コラボレーション WebSocket 仕様

対象: `GET /ws/collab/:document_key`（query `operator_id` 必須）。最終更新: 2026-05-03

## 1. 接続

1. ルート: `GET /ws/collab/:document_key`
2. クエリ: `operator_id=<string>` を必須で受ける。
3. 実装: `/apps/api/src/collaboration/handler.rs`
4. ルータ登録: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)

## 2. 連携方式

1. `WebSocketUpgrade` が成功すると `DocumentManager.connect(document_key, operator_id)` が呼び出される。
2. 接続直後に `initial_msgs`（初期同期用）があれば `Binary` でクライアントへ送出。
3. 以降、双方向は `Binary` のみを受け付ける。
4. Server → Client は room へ流れた更新を非同期タスクで転送する。
5. Client → Server は `Room::handle_message(peer_id, data)` へパース済みバイナリを渡す。

## 3. 切断処理

1. Text/Ping/Pong は基本無視。
2. Close または受信エラー時、`manager.disconnect(document_key, operator_id, peer_id)` を実行してルームから離脱する。
3. 再接続時は同一 `document_key` / `operator_id` で再参加され、初期同期の再配信を受ける前提。

## 4. 保存・状態管理

1. 持続化は `SqlxDocumentPersistence`（DB）を使用。
2. コラボ専用のバックグラウンド永続化タスクが `DocumentManager` で起動される。
3. 実装参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/persistence.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/persistence.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/manager.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/manager.rs)

## 5. 利用上の注意

1. 認可は `operator_id` 文字列のみで、JWT 検証はこのルートでは行われない想定である。
2. `initial_msgs` 受信失敗時は接続を即時終了する。
3. メッセージ形式は実装に準拠した binary（Yjs/CRDT 系）に依存するため、外部クライアントはその前提で実装する。
4. 監視観点は切断率、messageエラー率、初回同期遅延。

## 6. 外部公開・エンドユーザー向け

1. エンドユーザー向けの直接使用例は基本的に存在せず、編集クライアント同梱機能からの利用が前提。
2. 仕様整合は `DocumentManager` と room の更新ループを合わせて検証する。
