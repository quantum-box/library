# library-user-guide

Library の利用者向けドキュメントサイト。Vite + React で、`apps/web` と同じ
スタックです。Cloud App `library-user-guide` として Cloudflare Pages に
デプロイされます（定義は repo ルートの `tachyon.yaml`）。

対象読者は Library を API から使う人で、社内向けの仕様書 (`docs/specs/`) とは
別物です。仕様書が「実装がどうなっているか」を記述するのに対し、こちらは
「利用者が何をすればよいか」を記述します。

## 内容は API から取得する

エンドポイント一覧と GraphQL スキーマは、ページを開いたときに Library API
から取得しています。手で書き写した一覧は実装から必ずずれるため、ここには
置きません。

| 表示するもの | 取得元 |
| --- | --- |
| REST エンドポイント一覧 | `GET /v1beta/api-docs/openapi.json` |
| GraphQL の Query / Mutation | `GET /v1/graphql/introspection` |
| API のバージョン | 同じ OpenAPI ドキュメントの `info.version` |

取得に失敗しても、ページの散文は単体で意味が通るように書いてあります。
一覧の位置には取得できなかった旨が表示されます。

新しいページを書くときも、API から取れるものは取ってください。散文で書くのは
手順・注意・判断の理由など、API が答えられないことに限ります。

## ローカルで動かす

```bash
cp apps/user-guide/.env.example apps/user-guide/.env.local
yarn guide:dev
```

`.env.local` の `VITE_LIBRARY_API_BASE_URL` が、ガイドが説明する API です。
ローカルの API に向ければ、その API のエンドポイント一覧が表示されます。

```bash
yarn guide:build   # dist/ に出力
yarn guide:ts      # 型チェック
yarn guide:lint    # Biome
```

## 構成

```
src/lib/openapi.ts         OpenAPI ドキュメントの取得と整形
src/lib/graphql-schema.ts  SDL の取得と Query/Mutation の抽出
src/lib/use-async.ts       loading / error / data の 3 状態
src/components/            レイアウトと表示部品
src/pages/                 4 ページ分の本文
```

ルーティングは react-router の SPA です。`public/_redirects` が全パスを
`index.html` に返すため、直接 URL を開いても動きます。
