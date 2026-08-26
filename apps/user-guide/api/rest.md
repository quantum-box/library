# REST API

すべてのパスはベース URL からの相対です。以下の例では組織 `acme`、リポジトリ
`handbook` を対象にしています。自分の組織名・リポジトリ名に置き換えてください。

認証は全エンドポイント共通で `Authorization: Bearer $LIBRARY_API_KEY` です。

## データ

### 一覧

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data-list" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

### 1 件取得

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data/DATA_ID" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

### 名前で検索

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data?name=議事録" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

### 作成

```bash
curl -X POST "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data" \
  -H "Authorization: Bearer $LIBRARY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Data", "properties": {}}'
```

### 更新

```bash
curl -X PUT "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data/DATA_ID" \
  -H "Authorization: Bearer $LIBRARY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "Updated Data", "properties": {}}'
```

### 削除

```bash
curl -X DELETE "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data/DATA_ID" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

削除が成功すると `204 No Content` が返ります。レスポンスボディはありません。

## リポジトリ

### リポジトリ情報

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

### 組織情報

組織 ID (`tn_` で始まる値) はここで確認できます。GraphQL を使うときに必要です。

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/orgs/acme" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

## プロパティ

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/properties" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

## エクスポート

### Parquet

リポジトリ全体を Parquet 形式で取得します。分析用途向けです。

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data/parquet" \
  -H "Authorization: Bearer $LIBRARY_API_KEY" \
  -o handbook.parquet
```

### Markdown

1 件のデータを Markdown として取得します。`Content-Type` は `text/markdown` です。

```bash
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data/DATA_ID/md" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

## エラー

| ステータス | 意味 | 主な原因 |
| --- | --- | --- |
| 400 | リクエストが不正 | 必須項目の欠落、値の形式違い |
| 401 | 認証されていない | キーの指定漏れ、失効済みのキー |
| 403 | 権限がない | その組織・リポジトリへの権限不足 |
| 404 | 対象が見つからない | 組織名・リポジトリ名・ID の誤り |
| 409 | 競合 | 既に存在する名前での作成など |
| 500 | サーバー側エラー | 時間をおいて再試行 |

401 が返るときは、まず [API キーの発行と失効](/api/api-keys) の一覧画面で、その
キーがまだ有効かを確認してください。失効させたキーは即座に認証されなくなります。
