# BlockNote エディタの日本語 IME 入力重複を防ぐ

## 概要

`RecordBodyEditor` で日本語を変換している途中に、未確定文字が複数表示されたり重複したりする。現行実装は編集内容を 500ms の debounce で保存するが、IME の composition 状態を考慮していない。そのため候補選択中にも親コンポーネントの保存・再描画が走り、BlockNote の contenteditable が保持する未確定文字列へ干渉し得る。

IME 変換中は保存タイマーを停止し、`compositionend` 後に確定済みの文書だけを debounce 保存する。BlockNote の文書モデル、Markdown / RichText / Html の保存形式、画像・コードブロック機能は変更しない。

## 対象

- `apps/client/src/components/RecordBodyEditor.tsx`
- `apps/client/src/components/RecordBodyEditor.test.tsx`

Linear issue は未作成。API・DB・認可・永続化形式の変更はなく、DD / ADR は不要。

`origin/main` の Library client `0.1.7` を基準に、patch version `0.1.8` で完了する。

## 対応

1. composition 開始時に保存タイマーを解除する。
2. composition 中の BlockNote change は最新値として保持するが、親へ commit しない。変換中に画面遷移した場合も未確定値は保存しない。
3. composition 終了後に通常の debounce 保存を再開する。
4. 変換が 500ms を超えても未確定値が commit されず、確定後の値が一度だけ commit される回帰テストを追加する。

## 検証

- `apps/client` で `mise run test -- src/components/RecordBodyEditor.test.tsx`: 13 tests pass。
- `apps/client` で `mise run test`: 58 files / 455 tests pass。
- `apps/client` で `mise run type-check`: pass。
- `apps/client` で `mise run lint`: pass。既存の `TableView` / TanStack Table warning 1件のみ。
- `apps/client` で `mise run build`: pass。既存の chunk size、PGlite `eval`、PDF worker の warning のみ。
- Chromium + ローカル E2E API: `compositionstart` 後に「にほん」を入力し 1 秒待っても保存通信なし。`compositionend` 後の debounce で保存通信が始まり、本文表示は「にほん」1回分のみ。
- 自動確認に使った E2E fixture はインメモリプロセス終了により破棄済み。

### スキップした確認と理由

- macOS 日本語入力の候補ウィンドウ操作: ブラウザ自動化は OS の実 IME 候補 UI を生成できないため。修正後の実機最終確認では「にほんご」などを入力し、500ms を超えて候補を選択してから確定する。

## 完了条件

- IME 変換中に `onCommit` が呼ばれない。
- 変換確定後、確定済み本文が既存の 500ms debounce で一度だけ保存される。
- 通常入力、unmount 時の pending 保存、read-only 同期の既存テストが維持される。

## リスクと残タスク

- ブラウザ / OS 固有の ProseMirror IME 不具合が残る場合は、再現環境、入力キー列、確定操作を固定して上流 issue と切り分ける。
- 自動テストの composition event は OS の実 IME 候補 UI 自体を再現しないため、実機確認を別ゲートとして扱う。
