# PLT-5398 検証記録

## 状態

- 実装と軽量な静的確認が完了。
- Web v1とPhoton v2の両方を対象にした。

## 実施済み

- Linear issueの本文と受け入れ条件を確認。
- Library APIのrepo policy、RichText本文形式、Web/Photonのeditor構成を確認。
- 実装用worktreeを最新取得済みの`origin/main`から作成。
- APIのGraphQL schema生成コマンドをvendored Swagger UI featureで実行し、APIバイナリのコンパイルとschema生成が完了。
- 変更したRustファイルの`rustfmt --check`が成功。
- Web v1のGraphQL型を`yarn workspace library-web codegen:gql`で生成。
- Web v1のTypeScript確認`yarn workspace library-web ts`が成功。
- `git diff --check`が成功。
- 追加migrationはrepo外部キーとrepoスコープ制約を持つ追記型テーブルとして作成。

## 未実施

- Rust tests、workspace全体の重いRust検証、CI。
- Photon v2の型チェックとWeb v1 / Photon v2のブラウザ操作確認。
- migrationのDB適用、本番deploy。


## PR準備

- PR base `main`時点の`apps/client` version: `0.1.68`。PR version: `0.1.69`（patch）。
- `apps/client/package.json`と`apps/client/package-lock.json`を`npm version patch --no-git-tag-version --force`で同期した。
- taskdocを`docs/src/tasks/completed/v0.1.69/plt-5398-richtext-templates/`へ移動した。
- repositoryに`docs/SUMMARY.md`は存在しないため、summary navigation更新は不要。
- CI、Photon v2型チェック、ブラウザ操作、DB migration適用、本番deployは未実施。
