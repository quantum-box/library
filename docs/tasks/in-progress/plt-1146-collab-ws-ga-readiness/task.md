# PLT-1146 共同編集WebSocket GA readiness

## Status

判断: Non-GA / experimental

## Scope

`GET /ws/collab/:document_key` は Library GA の標準提供範囲に含めない。標準環境では router に登録せず、検証環境でのみ `LIBRARY_COLLAB_WS_ENABLED=true` を明示して有効化する。

## Findings

1. 標準 UI の `DataDetailUi` 呼び出しでは `collaborationWsUrl` / `collaborationOperatorId` を渡さないため、通常の編集導線から共同編集は開始されない。
2. WebSocket handler は Bearer token / session を検証せず、query の `operator_id` を信頼して `DocumentManager.connect(document_key, operator_id)` を呼ぶ。
3. GA に必要な認証、編集権限、認証済み tenant/operator 境界、再接続、競合、復元、長時間接続の検証が未完了。

## Implementation

1. `apps/api/src/router.rs` は `LIBRARY_COLLAB_WS_ENABLED=true` の場合のみ `/ws/collab/:document_key` を登録する。
2. default は disabled。未設定、`false`、`1`、`yes` は有効化しない。
3. disabled 時は `DocumentManager` とバックグラウンド永続化タスクも起動しない。

## Validation

1. `cargo fmt --check`
2. `cargo test -p library-api router::tests::collaboration_ws_env_flag_requires_true`
3. `cargo check -p library-api`
