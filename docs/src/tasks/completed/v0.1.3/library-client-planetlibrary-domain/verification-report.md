# 検証レポート

## 対象

- Cloud App: `library-client`
- App ID: `app_01kxx9qmg2vc4t2v1xm7zv2dtm`
- URL alias: `planetlibrary.txcloud.app`
- Alias ID: `dom_01kxxgag0ejknj93m9xz9zbb34`
- Active build: `bld_01kxxfjrvbe9508s5hnwkh8hh9`

## 結果

| 確認 | 結果 |
| --- | --- |
| Alias API readback | `planetlibrary.txcloud.app` を確認 |
| `GET /home` | HTTP 200 |
| 配信origin | `dd447257.library-client.pages.dev` |
| 配信asset | `assets/index-CL5YL7cx.js` |
| Browser | 新URLでLibrary Clientのサインイン画面を表示 |
| Console | error 0件 |
| TypeScript | `npm run type-check` 成功 |
| Lint | 0 errors、既存のTanStack Table warning 1件 |
| Tests | 40 files、241 tests成功 |
| Cloud build | `npm run build:cloud` 成功 |

## スキップした確認

- DB/APIの変更はないためmigrationおよびAPI scenario testは対象外。
- Alias切替は既存active deploymentへのroute変更であり、新規Cloud App buildは不要。

新しいサブドメインには旧URLのブラウザ認証状態が共有されないため、初回は再ログインが必要である。認証情報を再送信する確認は実施していない。

## 運用上の補足

現行CLIの `compute domains add` は予約された `txcloud.app` を一般custom domainとして拒否する。分散CLIにはURL aliasサブコマンドが未公開のため、今回はPlatform APIの正式なalias endpointを使用した。
