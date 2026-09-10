# PLT-4516 Library MCP 通知レスポンス修正

[Linear](https://linear.app/issue/PLT-4516) / [検証記録](verification-report.md)

Codex `0.153.4` からLibrary MCPへ接続すると、初期化通知へのHTTP応答をJSON-RPCメッセージとして解釈できず、`get_me` より前にハンドシェイクが終了していた。原因は `POST /mcp` がJSON-RPC通知に対して `200 OK` と `{}` を返していたことである。Streamable HTTPの契約に合わせ、受理した通知へ `202 Accepted` と空本文を返すよう修正した。

## 対応

1. `dispatch_rpc` の結果をHTTPレスポンスへ変換する処理を分離し、`Ok(None)` を `202 Accepted` の空レスポンスへ変換した。
2. 通知レスポンスのHTTPステータスと0 byte本文を固定する回帰テストを追加した。
3. Library APIを `1.11.3` から `1.11.4` へpatch更新した。

通常のJSON-RPC request、OAuth検証、ツール一覧・呼出し、SSE transportは変更していない。新しいADR/DDは不要と判断した。

## 検証

- MCP focused test 34件、crate check、clippy、build、format、diff checkが成功した。
- 詳細なコマンドと結果は[検証記録](verification-report.md)に記載した。

## 残タスク

マージ後にLibrary API `1.11.4` をデプロイし、本番の `notifications/initialized` がHTTP 202・空本文になることを確認する。その後、新しいCodexセッションで既存OAuth認証を利用し、`get_me` と認証済みツール一覧を確認する。ローカル成功を本番接続成功とは扱わない。
