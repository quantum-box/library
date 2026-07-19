# Library Client の正式URL変更

## 概要

Library Client の正式な公開URLを `https://planetlibrary.txcloud.app` に変更する。Cloud App名と自動生成URLは互換性のため維持し、TachyonのURL aliasを公開経路として利用する。

## スコープ

- `library-client` Cloud AppへのURL alias設定
- productionへのmanifest適用とmain build
- 新URLでのHTTPおよびログイン済み画面確認
- Client patch versionの更新

## 非ゴール

- 旧Planet Library Web (`planet-library`) の変更
- 自動生成URL `library-client.txcloud.app` の強制削除
- API、DB、認証設定の変更

## 関連文書

- [設計](./design.md)
- Linear issue: なし

## 実装と検証

1. Tachyon PlatformのURL aliasに `planetlibrary` を設定する。
2. Clientのbuildを実行する。
3. Ready PRをマージする。
4. deployment、HTTP、ログイン済み画面を確認する。

## 完了条件

- `planetlibrary.txcloud.app` が `library-client` のactive deploymentを配信する。
- 新URLで認証済みLibrary Clientが表示される。
- 旧Planet Library Webの `planet-library.txcloud.app` に影響がない。

## 実施結果

- URL alias `dom_01kxxgag0ejknj93m9xz9zbb34` を `library-client` に設定した。
- `https://planetlibrary.txcloud.app/home` がactive deploymentのassetをHTTP 200で配信することを確認した。
- Client `0.1.3` でtype-check、lint、241件のテスト、cloud buildが完了した。

## リスク

- URL aliasの反映にはtxcloud-proxyの収束時間が発生し得る。
- 標準URLはCloud App名に基づくため、alias変更後も到達可能なままとなる。
