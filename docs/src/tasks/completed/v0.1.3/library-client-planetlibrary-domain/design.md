# Library Client 正式URL変更 設計

## 目的

Library Clientを `https://planetlibrary.txcloud.app` で提供する。既存Cloud Appのidentity、build履歴、環境変数を維持しながら公開名だけを追加する。

## 採用方式

`library-client` の名前は変更せず、Tachyon PlatformのURL aliasとして `planetlibrary` を設定する。Cloud App名の変更は別アプリの作成や履歴分断につながるため採用しない。`txcloud.app` は予約ドメインであり一般custom domainでは扱えないため、CloudflareやDNSへの手動変更は行わず、Tachyonのalias APIとtxcloud-proxyに任せる。

`library-client.txcloud.app` はプラットフォームがCloud App名から生成する標準URLとして残る。プロダクト上の正式URLは `planetlibrary.txcloud.app` とし、必要なリダイレクトは別途要件化する。

## 影響

- API、DB、Cognito、認可、同期バックエンドは無変更。
- `planet-library.txcloud.app`（ハイフンあり）の旧Webアプリは無変更。
- alias設定時にactive production deploymentを向いたtxcloud-proxy routeが更新される。

## ロールアウトと検証

1. 現在のactive production deploymentを確認する。
2. URL aliasを設定し、alias登録状態を確認する。
3. active deployment、HTTP 200を確認する。
4. in-app browserでログイン済み画面とconsole errorを確認する。

## ADR

既存のCloud App custom domain機構を利用するだけで、長期的なアーキテクチャ判断は追加しないためADRは不要。
