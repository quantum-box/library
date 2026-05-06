# PLT-1114 — planet-library 本番 login 不能 (Cognito clientId="" Vite bundle bug、High Bug P0)

Linear: https://linear.app/quantum-box/issue/PLT-1114
worktree: /home/ubuntu/library.plt1114 (branch fix/plt-1114-cognito-vite, base origin/main)
repo: quantum-box/library

## 症状

planet-library.pages.dev/sign_in で sign-in を実行すると Cognito の InvalidParameterException が出る。
原因: Vite bundle が `VITE_COGNITO_CLIENT_ID=""` (空文字列) でビルドされており、CognitoIdentityProvider に空 ClientId が渡る。
deploy workflow (`.github/workflows/deploy.yml`) の Build step env が `VITE_BACKEND_API_URL` のみで、Cognito 系変数を渡していない。

## 修復方針 (tachyon-cowork PR #45 と同根 fix)

1. `.github/workflows/deploy.yml` の Build step env に下記を追加 (現状 `VITE_BACKEND_API_URL` のみ):
   - `VITE_COGNITO_CLIENT_ID`
   - `VITE_COGNITO_USER_POOL_ID`
   - `VITE_COGNITO_REGION`
   - `VITE_COGNITO_ISSUER`
   - (`VITE_COGNITO_CLIENT_SECRET` は public client なので使うか要確認、`apps/web/src/auth/cognito.ts` を読んで判断)
2. `apps/web/src/auth/cognito.ts` の env 読み出し: 現状 `import.meta.env.VITE_COGNITO_CLIENT_ID ?? ''` で empty fallback。production build で空ならビルド失敗 / 起動時 throw に変える (CLAUDE.md §6 設計哲学: lazy init で runtime error に先送り禁止)。
3. GitHub repo Variables / Secrets に値登録 (公開可なら Variables、秘匿なら Secrets):
   - `VITE_COGNITO_CLIENT_ID` (public client identifier、Variables 推奨)
   - `VITE_COGNITO_USER_POOL_ID` (Variables OK)
   - `VITE_COGNITO_REGION` = `ap-northeast-1` (Variables)
   - `VITE_COGNITO_ISSUER` = `https://cognito-idp.<region>.amazonaws.com/<user-pool-id>` (Variables)
   - `VITE_COGNITO_CLIENT_SECRET` (秘匿、Secrets — 必要な場合のみ)

## COO 事前調査結果 (2026-05-06)

AWS profile `n1` (account 418272779906、AdministratorAccess) で AWS SSO login 完了済、即 CLI 利用可。

**AWS Cognito 状況 (ap-northeast-1):**
- user pool は **1 つだけ**: `ap-northeast-1_8Ga4bK5M4` (name: `tachyon-user-pool`)
- us-east-1 / us-west-2 には Cognito user pool 無し
- `tachyon-user-pool` の user pool client 一覧 (10 件、name に `library` / `planet` 含む client は無い):
  - `agent-app-client` (29gub8i1sbauo5q58a7te5uti9)
  - `tachyon-ai-chat-client` (2h2pggfn7p9fh6rfn2sku2rnv)
  - `app_01kndnf7rcy9g4hqenzx23mzb6-plt1104-fixed-20260504144923` (3tei853mtgs4ljuea8qibccjq6)
  - `tachyond-client` (4101nun74fvfusnlsanc00urke)
  - `local-user-pool-client` (4ceiu7s2h9aujic0qfslej0spd)
  - `tachyon-app-client` (5002hok6cj8mjmt3gepdpdq98i)
  - `txcloud-web-client` (5189rab93k3m8gim2a91pjvl8d)
  - `bakuure-customer-app-client` (67ofq9ka321gvk1v0fj4de0j1h)
  - `tachyon-cowork-client` (78a4raqiqns509aadtv7ftjmee)
  - `tachyon-user-pool-client` (7rbbbkjg7qrie5r4b3ntkjt4c)

**判断ポイント (codex で決めて):**
- 既存 client (例: `tachyon-cowork-client` は `tachyon-cowork` PR #45 で同根 fix した先例) を流用するか、**新規 `planet-library-client` を作成するか**を decide
- 推奨: **新規 client 作成**。`tachyon-cowork-client` 流用だと callback URL / OAuth flow が cowork.txcloud.app 用に紐付いてる可能性あり、planet-library.pages.dev 専用 client を作る方が clean
- 作成 CLI 例: `aws cognito-idp create-user-pool-client --user-pool-id ap-northeast-1_8Ga4bK5M4 --client-name planet-library-client --no-generate-secret --explicit-auth-flows ALLOW_USER_PASSWORD_AUTH ALLOW_REFRESH_TOKEN_AUTH --profile n1 --region ap-northeast-1` (auth flow / callback URL は `apps/web/src/auth/cognito.ts` の実装と要照合)

**App Code 確認結果:**
- `apps/web/.env.sample`: `VITE_BACKEND_API_URL` `VITE_PLATFORM_ID=tn_01j702qf86pc2j35s0kv0gv3gy` `VITE_COGNITO_CLIENT_ID=` `VITE_COGNITO_CLIENT_SECRET=` `VITE_COGNITO_USER_POOL_ID=` `VITE_COGNITO_REGION=ap-northeast-1` `VITE_COGNITO_ISSUER=`
- `apps/web/vite-env.d.ts`: 全 5 個の VITE_COGNITO_* 宣言済 (CLIENT_ID / CLIENT_SECRET / USER_POOL_ID / REGION / ISSUER)
- `apps/web/src/auth/cognito.ts`: USER_PASSWORD_AUTH flow 利用、`generateSecretHash` あり = ClientSecret 利用ありの可能性。新規 client の secret 有無は cognito.ts を読み込んで判定

## 実装手順

1. AWS CLI で必要に応じて新規 user pool client 作成 (上記 CLI 例)
2. `.github/workflows/deploy.yml` 修正: Build step env 追加
3. `apps/web/src/auth/cognito.ts` 修正: empty fallback を throw / build-time error に変更 (Vite の `import.meta.env` は build time embed なので、空文字列なら `throw new Error("VITE_COGNITO_CLIENT_ID is required")` で起動時 fail-fast)
4. GitHub repo に Variables / Secrets 登録:
   ```bash
   gh variable set VITE_COGNITO_CLIENT_ID --body "<client-id>" -R quantum-box/library
   gh variable set VITE_COGNITO_USER_POOL_ID --body "ap-northeast-1_8Ga4bK5M4" -R quantum-box/library
   gh variable set VITE_COGNITO_REGION --body "ap-northeast-1" -R quantum-box/library
   gh variable set VITE_COGNITO_ISSUER --body "https://cognito-idp.ap-northeast-1.amazonaws.com/ap-northeast-1_8Ga4bK5M4" -R quantum-box/library
   # client secret 必要なら
   gh secret set VITE_COGNITO_CLIENT_SECRET --body "<secret>" -R quantum-box/library
   ```
5. PR open → CI green → AI review clear → admin merge → main push trigger で再 deploy → planet-library.pages.dev/sign_in で動作確認 (clientId="" の InvalidParameterException が消え、credential 検証 (NotAuthorizedException 等の正しい error) に変わる)
6. PR URL + 動作確認結果を COO に報告

## 触らないもの

- 既存 backend API URL (`VITE_BACKEND_API_URL`)
- Cognito user pool 設定 (pool 自体は触らず client のみ追加)
- lambda / 他環境変数

## 参考

- 同根 fix の先例: `tachyon-cowork` PR #45 (planet-library と同じ Cognito client_id="" 問題の修復例)
- CLAUDE.md §6 設計哲学: 依存欠落で起動時 panic も runtime error 先送りも両方 NG。**Config 解決時点で「使える依存のみ登録」** を徹底
- AWS profile = `n1` (環境変数 `AWS_PROFILE=n1` または `--profile n1`)

## 完了条件

- PR merge 済 (CI green、AI review clear)
- planet-library.pages.dev/sign_in で sign-in form が動作 (InvalidParameterException 消失、credential 検証エラーが正しく出る or 認証成功)
- COO に PR URL + 動作確認結果報告
