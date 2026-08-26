# library-user-guide

Library の利用者向けドキュメントサイト。VitePress で構築しています。

対象読者は Library を API から使う人で、社内向けの仕様書 (`docs/specs/`) とは
別物です。仕様書が「実装がどうなっているか」を記述するのに対し、こちらは
「利用者が何をすればよいか」を記述します。

## ローカルで動かす

```bash
yarn guide:dev
```

ビルド:

```bash
yarn guide:build
```

出力先は `.vitepress/dist` です。

## 構成

```
index.md              トップページ
api/getting-started.md  最初のリクエストまで
api/api-keys.md         API キーの発行・一覧・失効
api/rest.md             REST エンドポイント
api/graphql.md          GraphQL エンドポイント
```

## デプロイ

ホスティングは未設定です。静的サイトなので `.vitepress/dist` をそのまま配信できます。
公開先を決める際は、`amplify.yml` および `tachyon.yaml` への追加が必要かどうかを
あわせて検討してください。

## 書くときの約束

- ベース URL や組織 ID のような環境ごとに変わる値はハードコードせず、
  `$LIBRARY_API_URL` のようなプレースホルダで書き、確認手段を示す
- エンドポイントやフィールド名は `apps/api/schema.graphql` と
  `apps/web/src/app/v1beta/[org]/[repo]/api/_components/endpoints-data.ts` を
  実際に確認してから書く
