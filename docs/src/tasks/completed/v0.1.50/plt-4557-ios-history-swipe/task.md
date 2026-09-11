# PLT-4557 — iOS の履歴スワイプナビゲーション

## 概要

Planet Library の iOS アプリでは、アプリ内リンクによる画面遷移は TanStack
Router を通してブラウザ履歴へ保存されている。一方、ネイティブシェルの
`WKWebView` は back-forward navigation gesture が既定で無効なため、iOS の
左端スワイプで直前の画面へ戻れない。本タスクでは WebKit の標準ジェスチャーを
有効化し、既存履歴を iOS 標準の操作で辿れるようにする。

## スコープ

- iOS の `WKWebView` で標準の戻る・進むジェスチャーを有効化する。
- アプリ内リンクで積まれた既存履歴を利用する。
- 履歴がない場合は現在画面に留まる。

Web、macOS、Android のナビゲーション、Router の履歴形式、画面ごとの戻る
ボタンは変更しない。独自の touch event 判定や履歴スタックも追加しない。

## 対象

- `apps/client/src-tauri/src/ios_webview.rs`
- `apps/client/src-tauri/src/lib.rs`

## 関連

- Linear: [PLT-4557](https://linear.app/issue/PLT-4557)
- リリース対象: Library Client `0.1.50`（`origin/main` の `0.1.49` から patch bump）
- Apple WebKit:
  [`allowsBackForwardNavigationGestures`](https://developer.apple.com/documentation/webkit/wkwebview/allowsbackforwardnavigationgestures)

DD / ADR は作成しない。WebKit と既存 Router が提供する標準履歴を接続する
プラットフォーム設定であり、新しい永続仕様やアプリ固有の履歴モデルを導入しない
ためである。

## 実装

1. Tauri が iOS の main `WKWebView` を取得した時点で
   `allowsBackForwardNavigationGestures` を有効化する。
2. 既存の full-screen / safe-area 調整と同じ main-thread callback 内で設定する。
3. iOS 向け Rust check と native build で Objective-C selector の型とリンクを
   検証する。

## 検証

- `cargo check --target aarch64-apple-ios --manifest-path apps/client/src-tauri/Cargo.toml`
- iOS Simulator で Repository → Data と遷移し、左端スワイプで Repository に
  戻る。
- 戻った直後に右方向の進むジェスチャーで Data に進む。
- 初期画面で左端スワイプしてもアプリ外へ遷移しない。

### 2026-09-11 実施結果

- `cargo fmt --manifest-path apps/client/src-tauri/Cargo.toml -- --check`: 成功
- `cargo check --target aarch64-apple-ios --manifest-path apps/client/src-tauri/Cargo.toml`: 成功
- `npm run tauri:ios:build:sim`: `0.1.50` で2回実行し、いずれも frontend
  build、Rust `aarch64-apple-ios-sim` release build、Xcode build は
  `BUILD SUCCEEDED`。その後の Tauri CLI による `.app` 名変更が
  `Directory not empty (os error 66)` で失敗した。1回目の生成物は使用中でない
  ことを確認して Trash へ退避したが、再実行でも同じため、CI の simulator
  smoke を packaging の残存ゲートとする
- iPhone 17 Pro / iOS 26.5 Simulator へ生成した `Library.app` をインストールし、
  Home → Repository の Data 一覧 → Data 詳細という複数履歴の遷移を確認
- Simulator の自動操作では画面端ドラッグを開始できたが、操作ツールのドラッグ速度では
  interactive transition を完了できなかった。戻る・進む・履歴なし時の実操作、および
  エディタ内操作との競合は実機確認を残す
- `git diff --check`: 成功

## 完了条件

- iOS の左端スワイプがブラウザ履歴の一つ前へ戻る。
- 履歴の戻る／進むで TanStack Router の表示と URL が一致する。
- Web、macOS、Android のコードパスに変更がない。

## リスクとフォローアップ

エディタ内の横スクロールやドラッグと画面端ジェスチャーの競合は Simulator と実機で
確認する。ネイティブの interactive transition 表示は WebKit 管理であり、React 側で
独自アニメーションを重ねない。
