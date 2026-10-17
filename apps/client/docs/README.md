# Library client — docs

このディレクトリには **library v2（`apps/client`）自身の運用ドキュメント**だけを置きます。

| ドキュメント | 内容 |
|---|---|
| [`app-platforms.md`](./app-platforms.md) | web / desktop / mobile の各シェルと、プラットフォームごとの差異 |
| [`desktop-release.md`](./desktop-release.md) | desktop アプリの配布と自動更新（署名鍵の扱いを含む） |
| [`ios-testflight.md`](./ios-testflight.md) | iOS ビルドの TestFlight 自動アップロード（App Store Connect API キーと署名） |
| [`android-firebase-distribution.md`](./android-firebase-distribution.md) | Android ビルドの Firebase App Distribution 自動配布（署名鍵とサービスアカウント） |
| [`macos-window-tabs.md`](./macos-window-tabs.md) | macOS のウィンドウタブ（単一ウィンドウの子 WebView として実装） |
| [`library-api-production.md`](./library-api-production.md) | 本番 Library API への接続 |
| [`data-editor-live.md`](./data-editor-live.md) | data editor の共同編集と Library 側の認可・保存条件 |
| [`public-docs.md`](./public-docs.md) | public repo の閲覧専用ドキュメント表示・匿名取得・検証範囲 |

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

## Repository node view

Library remains a planet-scale CMS. The Node view displays repository Data
as nodes, existing Properties as fields, and Relations as connections. Tree
layout is a generic way to view hierarchical content, not a domain-specific
content model.

- Cards and the data picker show only selected repository Properties; status and priority are not implicit fields.
- Use **Node properties** to select which repository Properties appear on cards.
- Choose a same-repository **Relation** under **Connections** to draw its links.
  The direction is source → target: the source Data's selected Relation contains
  the target Data ID. Drawing or deleting these connections updates that Property.
- Choose **View annotations** for free-form connections and editable edge labels.
  These lines do not modify repository Relations; existing canvas lines retain
  this meaning when a Relation is selected.
- Enter a title and optionally choose a parent to create Data from the canvas.
  With a Relation selected, the parent references the new child. Otherwise the
  parent/child connection is a view annotation. If the second write fails, the
  new Data remains available and the view reports the missing connection.
- **Auto arrange** lays out the graph from left to right. Branches can be collapsed
  and expanded. The Tree layout prevents newly drawn cycles in the displayed
  graph; it does not impose a new constraint on the repository's Relation schema.
- Select a node and use **Open data**, or double-click it, to edit its ordinary
  Data and Properties. **Remove from view** removes its placement and attached
  annotation lines, retaining the Data and its Relations.

Selected Properties, the selected Relation, collapsed branches, labels, and
positions are saved with the canvas and synchronized using the existing Yjs
channel. Legacy canvases remain readable. Repository listings are paginated;
**Load more** includes further Data in the picker, and **Refresh** reloads the
repository after changes elsewhere. Cross-repository Relations remain editable
in the Data editor but are not offered as this repo view's connection source.
