# はじめに

Library の API を使うには、次の 3 つが必要です。

1. **API キー** — 組織単位で発行します
2. **ベース URL** — 環境ごとに異なります
3. **組織名とリポジトリ名** — URL に含めます

## 1. API キーを発行する

Web 画面でリポジトリを開き、上部のタブから **API** を選びます。開発者ポータルの
クイックスタート「① API キーを作成」から発行できます。

キーは発行時に一度だけ表示されます。手順の詳細と取り扱いの注意は
[API キーの発行と失効](/api/api-keys) を参照してください。

## 2. ベース URL を確認する

ベース URL は環境ごとに異なるため、開発者ポータルのクイックスタート
「② ベース URL」に表示されている値をそのまま使ってください。同じ画面の
コピーボタンで取得できます。

以下の例では `$LIBRARY_API_URL` と表記します。

## 3. 最初のリクエストを送る

組織 `acme`、リポジトリ `handbook` のデータ一覧を取得する例です。

::: code-group

```bash [curl]
curl -X GET "$LIBRARY_API_URL/v1beta/repos/acme/handbook/data-list" \
  -H "Authorization: Bearer $LIBRARY_API_KEY"
```

```python [Python]
import os
import requests

response = requests.get(
    f"{os.environ['LIBRARY_API_URL']}/v1beta/repos/acme/handbook/data-list",
    headers={"Authorization": f"Bearer {os.environ['LIBRARY_API_KEY']}"},
)
print(response.json())
```

```javascript [JavaScript]
const response = await fetch(
  `${process.env.LIBRARY_API_URL}/v1beta/repos/acme/handbook/data-list`,
  {
    headers: { Authorization: `Bearer ${process.env.LIBRARY_API_KEY}` },
  },
)
const data = await response.json()
console.log(data)
```

:::

## 認証ヘッダ

すべてのリクエストに `Authorization` ヘッダを付けます。

```
Authorization: Bearer pk_xxxxxxxxxxxxxxxx
```

API キーは `pk_` で始まります。ログイン中のユーザーが使う JWT も同じヘッダで
受け付けられますが、サーバー間の連携には API キーを使ってください。

キーを付けずに送ったリクエストは匿名として扱われます。公開リポジトリなら読み取れる
ことがありますが、非公開リポジトリや書き込みは拒否されます。

## API リファレンス

エンドポイントの網羅的な一覧は、開発者ポータルからリンクされている OpenAPI の
ドキュメントを参照してください。

- Swagger UI: `$LIBRARY_API_URL/v1beta/swagger-ui`
- ReDoc: `$LIBRARY_API_URL/v1beta/redoc`
- OpenAPI 仕様 (JSON): `$LIBRARY_API_URL/v1beta/api-docs/openapi.json`

よく使うエンドポイントは [REST API](/api/rest) にまとめています。
