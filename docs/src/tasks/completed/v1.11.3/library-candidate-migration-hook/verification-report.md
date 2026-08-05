# 検証レポート

## 対象

- Cloud App: `library-api`
- App ID: `app_01kshpeg8ppzemk6cypbws3q3j`
- Tenant: `tn_01j91h09tpj5ehwbwfwfxpak2b`
- base version: `1.11.2` (`origin/main`)
- PR version: `1.11.3` (patch)

## 実装検証

| 確認 | 結果 |
| --- | --- |
| `cargo +nightly-2026-06-04 fmt --all -- --check` | 成功 |
| `cargo +nightly-2026-06-04 test -p library-api --lib --no-default-features` | 105 passed、3 ignored |
| deploy hook bearer unit tests | 2 passed |
| `cargo +nightly-2026-06-04 clippy -p library-api --no-default-features --all-targets -- -D warnings` | 成功 |
| `cargo +nightly-2026-06-04 build -p library-api --bin lambda-library-api --no-default-features` | 成功 |
| production manifest dry-run | 成功、registry差分なし、専用secret reference解決済み |
| dedicated migration Lambda参照 | active manifest、build target、build scriptから削除 |

## 本番事前確認

- 修正前の最新Buildは成功したが、deploymentはdedicated migration Lambdaの
  warm invocation panicにより失敗した。
- そのためproduction Aliasは旧deploymentのままであり、authenticated組織作成は
  `FORBIDDEN: library:CreateOrganization`のままである。
- productionには`LIBRARY_DEPLOY_HOOK_TOKEN`のsecret referenceを登録済み。
  値は取得・表示・保存していない。

## Merge後に必要な確認

1. main Buildが`1.11.3`を生成すること。
2. candidate `postDeploy.command` migration hookが成功すること。
3. production Aliasが新しいVersionへ昇格すること。
4. `/version`が`1.11.3`を返すこと。
5. Playwrightでテストユーザーがログインし、組織作成に成功すること。

本番確認は未mergeのcommitをproductionへ適用しないため、Ready PR時点では意図的に保留する。
