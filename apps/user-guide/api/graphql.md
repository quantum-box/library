# GraphQL API

REST が 1 エンドポイント 1 リソースなのに対し、GraphQL は必要なフィールドだけを
1 回の往復でまとめて取得できます。エンドポイントは 1 つです。

```
POST $LIBRARY_API_URL/v1/graphql
```

## API キーで呼ぶときは組織 ID が必要

REST のパスには `/repos/acme/handbook` のように組織名が含まれるため、サーバーは
どの組織のキーとして検証すればよいか分かります。GraphQL のパスには組織名が
含まれません。そのため、**API キーで GraphQL を呼ぶときは `x-operator-id` ヘッダで
組織 ID を指定してください。**

```bash
curl -X POST "$LIBRARY_API_URL/v1/graphql" \
  -H "Authorization: Bearer $LIBRARY_API_KEY" \
  -H "x-operator-id: $LIBRARY_ORG_ID" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ apiKeys(orgUsername: \"acme\") { id name createdAt } }"}'
```

::: warning ヘッダを付け忘れると匿名になります
`x-operator-id` がないと、キーは検証されずリクエストは匿名として扱われます。
認証エラーではなく「権限がありません」や空の結果として返るため、気づきにくい
挙動です。
:::

組織 ID (`tn_` で始まる値) は REST から取得できます。

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/orgs/acme" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

レスポンスの `id` が組織 ID です。値は組織ごとに固定なので、一度確認したら
環境変数に入れておくとよいでしょう。

## クエリの例

### リポジトリとデータ一覧をまとめて取得する

REST なら 2 回に分かれる問い合わせが、1 回で済みます。

```graphql
query {
  repo(orgUsername: "acme", repoUsername: "handbook") {
    id
    name
    description
    dataList(pageSize: 20, page: 1) {
      items {
        id
        name
        updatedAt
      }
      paginator {
        currentPage
        totalPages
        totalItems
      }
    }
  }
}
```

### API キー一覧

```graphql
query {
  apiKeys(orgUsername: "acme") {
    id
    name
    createdAt
  }
}
```

## ミューテーションの例

### API キーを発行する

```graphql
mutation {
  createApiKey(input: { organizationUsername: "acme", name: "ci-pipeline" }) {
    apiKey {
      id
      name
      value
    }
  }
}
```

`value` がキーの本体です。このレスポンスでしか取得できません。

### API キーを失効させる

```graphql
mutation {
  revokeApiKey(input: { organizationUsername: "acme", apiKeyId: "API_KEY_ID" })
}
```

`apiKeyId` は `apiKeys` クエリで得られる `id` です。失効は取り消せません。

## スキーマを調べる

ブラウザから開ける Playground と、SDL を返すエンドポイントがあります。

- Playground: `$LIBRARY_API_URL/v1/graphql` を GET で開く
- SDL: `$LIBRARY_API_URL/v1/graphql/introspection`

## エラーの読み方

GraphQL は HTTP としては 200 を返しつつ、レスポンスの `errors` 配列に失敗を
入れて返します。REST のようにステータスコードだけで判定すると、失敗を成功として
扱ってしまいます。

```json
{
  "data": null,
  "errors": [
    {
      "message": "PermissionDenied: action: library:RevokeApiKey",
      "extensions": { "code": "FORBIDDEN" }
    }
  ]
}
```

`errors` が空でないかを必ず確認してください。
