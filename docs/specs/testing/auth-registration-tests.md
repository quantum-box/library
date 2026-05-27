# 新規登録 / 認証フローのテストカバレッジ

このドキュメントは、Library Web の新規登録とメール確認フローについて、現在テストで守っている内容を整理します。

## 対象ファイル

実装:

1. `apps/web/src/routes/_auth/sign_up.tsx`
2. `apps/web/src/app/(auth)/sign_up/form.tsx`
3. `apps/web/src/app/(auth)/sign_up/type.ts`
4. `apps/web/src/routes/_auth/verify-email/otp.tsx`
5. `apps/web/src/app/(auth)/password-constants.ts`
6. `apps/web/src/auth/cognito.ts`
7. `apps/web/src/lib/i18n/auth-translations.ts`

テスト:

1. `apps/web/src/app/(auth)/password-constants.test.ts`
2. `apps/web/src/app/(auth)/sign_up/type.test.ts`
3. `apps/web/tests/e2e/auth-sign-up.spec.ts`

## Unit Test で守っていること

### パスワード要件

`password-constants.test.ts` で次を確認しています。

1. 大文字、小文字、数字を含むパスワードを許可する
2. 記号なしのパスワードを許可する
3. 8文字未満を拒否する
4. 必須文字種がないパスワードを拒否する

現在の許可例:

```text
Password1
NoSymbol1
```

現在の拒否例:

```text
Pass1
password
```

### 認証コード要件

`password-constants.test.ts` で次を確認しています。

1. 6桁の数字を許可する
2. 英字混じりのコードを拒否する
3. 5桁や7桁のコードを拒否する

現在の許可例:

```text
123456
```

現在の拒否例:

```text
12ab56
12345
1234567
```

### 新規登録フォーム入力

`sign_up/type.test.ts` で次を確認しています。

1. 有効な username / email / password を許可する
2. 3文字未満の username を拒否する
3. 半角英数字以外を含む username を拒否する
4. 不正な email を拒否する
5. password は共通の `passwordSchema` を使う

現在の許可例:

```json
{
  "username": "user123",
  "email": "user@example.com",
  "password": "Password1"
}
```

## E2E Test で守っていること

### `/sign_up` の表示

`auth-sign-up.spec.ts` で次を確認しています。

1. 見出し `新規登録` が表示される
2. `ユーザー名を入力` が表示される
3. `メールアドレスを入力` が表示される
4. `パスワードを入力` が表示される
5. `パスワードを再入力` が表示される
6. button `新規登録` が表示される
7. link `サインイン` が `/sign_in` を指す

見出しや主要ボタンは `exact: true` で確認します。これは `新規登録_RED_CHECK` のような部分一致で通ってしまう回帰を防ぐためです。

### `/sign_up` のバリデーション

次の入力で送信し、画面に日本語メッセージが出ることを確認しています。

```text
username: ab
email: not-email
password: Password1
confirmPassword: Password2
```

期待する表示:

```text
ユーザー名は3文字以上で入力してください
有効なメールアドレスを入力してください
パスワードが一致しません
```

このテストは Cognito への実送信を発生させません。

### パスワード表示切替

次を確認しています。

1. 初期状態では password input の `type` が `password`
2. `表示` を押すと `type` が `text`
3. ボタン表示が `非表示` になる

### サインイン導線

`/sign_up` から `サインイン` link を押し、`/sign_in` に遷移して見出し `サインイン` が表示されることを確認しています。

### `/verify-email/otp` の表示

次を確認しています。

1. 見出し `メールを確認してください` が表示される
2. `ユーザー名を入力` が表示される
3. `6桁のコードを入力` が表示される
4. button `認証` が表示される
5. button `コードを再送信` が表示される
6. link `やり直す` が `/sign_up` を指す

### `/verify-email/otp` のバリデーション

空のまま `認証` を押し、次の日本語メッセージが出ることを確認しています。

```text
ユーザー名を入力してください
6桁の認証コードを入力してください
```

このテストも Cognito への実送信を発生させません。

## Red 確認の実績

E2E の assertion が弱くないことを確認するため、次の Red 確認を行いました。

1. 一時的に `authTranslations.ja.signUp.title` を `新規登録_RED_CHECK` に変更
2. `corepack yarn test:e2e --grep "renders the registration form"` を実行
3. `getByRole('heading', { name: '新規登録', exact: true })` が見つからず失敗することを確認
4. 一時変更を戻して `corepack yarn test:e2e` が成功することを確認

この確認により、見出しの文言回帰が部分一致で見逃されないことを確認済みです。

## 未カバー

現在は次をテストしていません。

1. Cognito への実際の `SignUpCommand`
2. Cognito への実際の `ConfirmSignUpCommand`
3. Cognito への実際の `ResendConfirmationCodeCommand`
4. メールに届く OTP の取得
5. 登録完了後の自動サインイン
6. platform API の `signInOrSignUp` 成功パス
7. 英語 locale の表示
8. モバイル viewport

これらを追加する場合は、専用の認証テスト環境、mock / intercept 方針、またはテストユーザー cleanup 方針を先に決めます。
