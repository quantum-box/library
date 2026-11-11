# TiDB runtimeとmigrationのSQL identity分離

## 背景

LibraryのTiDB `role_readwrite` はglobal DDLを含み、候補Lambdaのmigration gateとHTTP servingが同じ `library_api` userを使っている。runtimeの権限を先に削ると次のdeploymentを止めるため、専用 `library_migrator` の選択を先に導入する。

## 実装

Production candidate gateは `MIGRATOR_DATABASE_URL` を必須とし、runtimeと同じcluster prefix・host・port、別の `library_migrator` userであることをSQL実行前に検証する。CLIのprod migrationも明示したmigration credentialだけを使い、runtimeの `DATABASE_URL` へfallbackしない。Previewは従来のper-PR DBとgrantを維持する。

`sources` tableは既存migration `20250313002330_add_tags_and_sources.up.sql` の責任である。HTTP request中のtable作成fallbackを除き、schema欠落をDDL権限で隠さない。

Runtimeは `library` と `tachyon_apps_database_manager` のDMLへ、migratorは同2schemaのDDL/DMLへ縮小する。SQL grantとSSM配送はquantum-box/tachyon-appsの対応taskで管理し、このソースだけをmergeしてcredential未設定の本番をdeployしない。

## 実行境界とrollout

同じcandidateのartifactを検証するため、内部migration eventは既存Lambda内に残る。SQL identityは分離するが、migration credentialも同じprocess environmentに存在する。プロセス侵害に対する完全分離ではなく、別実行単位化はPrivateLink到達性・同じcommitの検証を保つ後続設計とする。

新credentialを1PasswordとSSMへ保存・読み戻しし、candidate migrationの成功を確認してからruntime grantを撤去する。HTTP healthとDMLを確認する。失敗時は専用userの旧grantを一時復旧し、rootへ戻さない。

## 検証・状態

- dedicated migrator / wrong cluster / wrong host / root / runtime userのbinding unit testは成功。
- sources fallback除去後もmigration moduleのunit test 9件が成功。fmt/diff checkも成功。candidateでの確認は新credential配送後に行う。
- 本番runtime grantはまだ変更していない。新migratorの保存・配送が完了するまで保持する。
- API patch version: origin/main 1.11.13 → 1.11.14。
