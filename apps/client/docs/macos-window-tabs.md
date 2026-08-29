# macOS ウインドウタブ

macOS デスクトップシェルは 1 枚のネイティブ Window の中でタブを切り替えます。
各タブは Window の child WebView で、タブ切替時に Window を hide/show しません。

実装パターンは `@tachyon-sdk/native-ui` の
[`docs/tauri-macos-tabs.md`](https://github.com/quantum-box/native-ui/blob/main/docs/tauri-macos-tabs.md)
に従います。UI は同パッケージの `MacOSWindowTabs` をそのまま使い、
このアプリ側は WebView のライフサイクルだけを持ちます。

| レイヤー | 実装 |
|---|---|
| タブの見た目・キーボード・ARIA | `MacOSWindowTabs`（native-ui） |
| WebView 生成 / ready / 表示切替 / close | [`src-tauri/src/macos_tabs.rs`](../src-tauri/src/macos_tabs.rs) |
| Tauri コマンドの型付きラッパーとタイトル解決 | [`src/lib/desktop/windowTabs.ts`](../src/lib/desktop/windowTabs.ts) |
| イベント購読・タイトル更新・⌘クリック | [`src/components/desktop/WindowTabStrip.tsx`](../src/components/desktop/WindowTabStrip.tsx) |

## 操作

| 操作 | 挙動 |
|---|---|
| `+` / `⌘T` / File ▸ New Tab | 新しいタブを開いて選択する |
| アプリ内リンクを `⌘`+クリック | 背面タブで開く（現在のタブと選択状態は維持） |
| タブの `×` / `⌘W` | 選択中のタブだけを閉じる。最後の 1 枚なら Window を閉じる |
| `←` `→` `Home` `End`（タブにフォーカス時） | 隣・端のタブへ移動する |

## 白いちらつきを防ぐ handshake

新しい child WebView は Window 幅の外側に実サイズで作られます。React が mount 後に
`requestAnimationFrame` を 2 回待ってから `mark_window_tab_content_ready` を呼び、
Rust 側はそこで初めて WebView を画面内へ移動します。`load` 完了だけを ready 判定に
すると WKWebView の白い backing layer が 1 フレーム露出します。

## 制約

- **macOS の Tauri シェル専用**です。Web / Windows / Linux / モバイルでは
  `WindowTabStrip` は何も描画しません（`app_target_os` で判定）。
- child WebView の初期 URL は `/` 始まりのアプリ内パスだけを受け付けます
  （`sanitize_tab_path`）。外部 URL をタブとして開くことはできません。
- **タブごとにアプリ全体が独立して動きます。** 各 WebView が自分の PGlite / Yjs
  インスタンスを持ち、同じ IndexedDB を共有します。PGlite はローカルキャッシュで
  あり真実の源ではない（Library API と Yjs が源）ものの、同じ repository を複数タブで
  同時に編集するとローカルキャッシュ側で書き戻しが競合しえます。Web で複数タブを
  開いた場合と同じ既存の制約ですが、タブ機能によって踏みやすくはなります。

  `@electric-sql/pglite/worker` のマルチタブ共有は一度試して見送りました。WebView を
  またいだ leader 選出は実機で動きましたが、全クエリが worker 経由になり
  1 クエリ 12ms → 76ms（0.4.5 / 0.5.8 とも同じ）になります。コストはストアに
  データが溜まるほど効き、document 系の E2E がローカルで 15 秒から 85 秒に伸びました。
  `worker/index.js` がリーダーでない間 16ms ごとに `tab-here` を postMessage し続ける
  ループを持っており、改善は upstream 側の課題です。
- capability は Window ではなく WebView label を対象にしています
  (`main`, `library-tab-*`)。新しい WebView label 形式を足すときは
  `src-tauri/capabilities/default.json` も更新してください。
