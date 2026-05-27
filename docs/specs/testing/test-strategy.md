# テスト戦略

Library のテストは、仕様を守る粒度ごとに Unit Test と E2E Test を分けて管理します。

## 目的

1. Unit Test はロジック・スキーマ・エラーハンドリングの小さな振る舞いを高速に守る
2. E2E Test はユーザーが実際に触る画面、導線、表示文言、フォームバリデーションを守る
3. 外部サービスに依存する成功パスは、デフォルトでは実送信せず、安定した範囲から確認する
4. 新しいテストは t-wada の TDD に近い形で、Red -> Green -> Refactor の証跡を確認してから追加する

## コマンド

リポジトリルートから Yarn 4 経由で実行します。

```bash
corepack yarn test
corepack yarn test:e2e
corepack yarn build
```

CI では `.github/workflows/ci.yml` が同じコマンド群を実行します。

## Unit Test

Unit Test は `apps/web/src/**/*.test.ts` を対象にします。

現在の対象:

1. `apps/web/src/app/v1beta/_lib/platform-error-handler.test.ts`
   - `NOT_FOUND_ERROR` の notFound 変換
   - `PERMISSION_DENIED` の warn 継続
   - その他エラーの rethrow / no-op
2. `apps/web/src/app/(auth)/password-constants.test.ts`
   - パスワード要件
   - 記号が必須ではないこと
   - 6桁認証コード要件
3. `apps/web/src/app/(auth)/sign_up/type.test.ts`
   - 新規登録フォーム入力スキーマ
   - ユーザー名、メールアドレス、パスワードの最小要件

Unit Test では Playwright の spec を拾わないよう、`apps/web/package.json` の `test` script は `src` 配下に限定します。

```json
"test": "vitest run src --passWithNoTests"
```

## E2E Test

E2E Test は `apps/web/tests/e2e/**/*.spec.ts` を対象にします。

現在の対象:

1. `apps/web/tests/e2e/auth-sign-up.spec.ts`
   - `/sign_up` の表示
   - 新規登録フォームの日本語バリデーション
   - パスワード表示切替
   - `/sign_in` への導線
   - `/verify-email/otp` の表示
   - 認証コード画面の日本語バリデーション

E2E は `apps/web/playwright.config.ts` で `ja-JP` に固定しています。文言回帰を守る assertion は部分一致ではなく `exact: true` を使います。

## Red / Green / Refactor

E2E や Unit Test を追加するときは次の順序を守ります。

1. Red: 期待する振る舞いを表す最小のテストを書く
2. Red 確認: そのテストが意味のある理由で失敗することを確認する
3. Green: 最小の実装で通す
4. Green 確認: 狭い対象と全体テストを通す
5. Refactor: テスト名、fixture、重複、locator を整理する

既存実装がすでに満たしていて最初から Green になる場合は、一時的に対象の振る舞いを壊して Red を確認し、すぐ戻します。

## 外部サービス依存の扱い

通常の E2E では Cognito へ実際の新規登録、確認コード再送、サインイン、パスワードリセットを送信しません。

理由:

1. 実ユーザーが永続化される
2. メール / OTP の取得が必要になる
3. 重複ユーザーや cleanup 漏れで CI が不安定になる
4. 本番または共有環境の認証設定にテストが依存する

実送信まで含める場合は、専用 Cognito User Pool、テストユーザー削除、メール/OTP 取得方法、環境変数の分離を先に用意します。

## CI で必要なもの

GitHub Actions の CI は `.github/workflows/ci.yml` で管理します。

実行内容:

1. `yarn install --immutable`
2. `yarn lint`
3. `yarn test`
4. `yarn tsc`
5. `yarn build`
6. `yarn workspace library-web exec playwright install --with-deps chromium`
7. `yarn test:e2e`

Playwright を CI で回すため、E2E 前に Chromium の browser binary と OS 依存パッケージを準備します。

```bash
corepack enable
yarn install --immutable
yarn workspace library-web exec playwright install --with-deps chromium
yarn test
yarn test:e2e
```

## 残っているリスク

1. Cognito 実送信の成功パスは未カバー
2. API backend と連動したログイン後のセッション生成は未カバー
3. 英語 locale の auth UI は未カバー
4. モバイル viewport の登録画面は未カバー
5. CI の E2E はローカル Vite dev server の画面検証であり、外部 Cognito の実ユーザー作成は行わない
