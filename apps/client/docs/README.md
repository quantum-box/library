# Library client — docs

このディレクトリには **library v2（`apps/client`）自身の運用ドキュメント**だけを置きます。

| ドキュメント | 内容 |
|---|---|
| [`app-platforms.md`](./app-platforms.md) | web / desktop / mobile の各シェルと、プラットフォームごとの差異 |
| [`desktop-release.md`](./desktop-release.md) | desktop アプリの配布と自動更新（署名鍵の扱いを含む） |
| [`macos-window-tabs.md`](./macos-window-tabs.md) | macOS のウィンドウタブ（単一ウィンドウの子 WebView として実装） |
| [`library-api-production.md`](./library-api-production.md) | 本番 Library API への接続 |

## Photon Engine / Photon Live の設計

**このリポジトリでは持ちません。** [quantum-box/photon](https://github.com/quantum-box/photon) の `docs/` を参照してください。

| 知りたいこと | 参照先 |
|---|---|
| Engine と Live の責務分界 | [`docs/architecture/decisions/ADR-0001-sync-responsibility-boundaries.md`](https://github.com/quantum-box/photon/blob/main/docs/architecture/decisions/ADR-0001-sync-responsibility-boundaries.md) |
| Engine が製品である理由 | [`docs/architecture/decisions/ADR-0002-photon-engine-as-the-product.md`](https://github.com/quantum-box/photon/blob/main/docs/architecture/decisions/ADR-0002-photon-engine-as-the-product.md) |
| パッケージ構成 | [`docs/architecture/package-topology.md`](https://github.com/quantum-box/photon/blob/main/docs/architecture/package-topology.md) |
| Engine / Live のアーキテクチャ | [`docs/architecture/photon-engine-live.md`](https://github.com/quantum-box/photon/blob/main/docs/architecture/photon-engine-live.md) |
| マルチタブのローカルストア共有 | [`docs/architecture/multi-tab-local-store.html`](https://github.com/quantum-box/photon/blob/main/docs/architecture/multi-tab-local-store.html) |
| Cloudflare 経由の sync | [`docs/cloudflare-sync.md`](https://github.com/quantum-box/photon/blob/main/docs/cloudflare-sync.md) |
| 添付ファイルの sync | [`docs/attachments-sync.md`](https://github.com/quantum-box/photon/blob/main/docs/attachments-sync.md) |
| ローカル 3 層 sync の検証手順 | [`docs/architecture/three-tier-local-sync-lab.md`](https://github.com/quantum-box/photon/blob/main/docs/architecture/three-tier-local-sync-lab.md) |
| Engine / Live サーバのデプロイ | [`docs/server-deploy.md`](https://github.com/quantum-box/photon/blob/main/docs/server-deploy.md) |
| リリース追従の手順 | [`docs/release-following.md`](https://github.com/quantum-box/photon/blob/main/docs/release-following.md) |

以前はこれらのコピーを `apps/client/docs/` に置いていましたが、import 以降ほとんど更新されず upstream から乖離していたため削除しました。エンジンの設計は photon が正本です。

## Library 側の設計判断

Library の Bounded Context と Photon との境界は、リポジトリルートの
[`docs/specs/decisions/`](../../../docs/specs/decisions/) にある ADR が正本です。
