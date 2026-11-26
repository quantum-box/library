# 組織の platform と上流認可の正を一つにする（実装）

設計: [design.md](design.md)

## 合意した方針（2026-11-19、プロダクトオーナー確認済み）

| # | 判定 | 変更前 | 変更後 |
|---|---|---|---|
| 1 | 組織 scope の `x-platform-id` | 毎回 tachyon に問い合わせ、分からなければ Library | `organizations.home_platform_id` から決める。未記録なら tachyon で確定して記録。確定できなければ当面は Library（B1） |
| 2 | client が送る `x-platform-id` | GraphQL でそのまま上流へ | `x-operator-id` が Library の組織を指すときは、組織の platform で上書き |
| 3 | `x-operator-id` と作成先の照合 | 食い違えば書き込み前に拒否 | 同じ（Verus で証明） |
| 4 | 取り込み時の policy 付与失敗 | warn して組織を作る | 取り込みを失敗させ、組織を保存しない |
| 5 | 委任 grant の自動付与 | 手作業 | 取り込み時に、取り込む admin が Library の service account へ委任を同意する（tachyon ADR-0177、D4-a） |

- B1: 未記録の組織は初回利用時に記録する（自動 backfill）。全行が埋まったら、確定できない場合を拒否に切り替え、列を `NOT NULL` にする
- C1: Verus を導入（`packages/verified_org_scope`）

## 実装

- `organizations.home_platform_id`（NULL 許容）を追加。`Organization::new`（Library 作成）、`Organization::adopt`（取り込み）、`Organization::reconstruct`（復元）、`place_on_home_platform`（一度だけ記録、別の platform への置き換えは Conflict）。保存は `COALESCE` で既存値を上書きしない
- `LibraryOrg::for_organization` が組織 scope の唯一の構築子。`with_org_and_operator` / `on_other_platform` / `resolve` は非公開か削除。例外は `library_native`（ネイティブ組織の復元時の認可）と `for_local_read`（tachyon に届かない読み取り）
- `OrgScopes::scope_for`: extractor・MCP（32 tool）・`create_repo`・GraphQL の `LibraryMultiTenancy` が使う
- `seedLibraryTenant`: tenant scope を組織の platform から作る（従来は Library 固定で、取り込み組織の `/v1/auth/users` が 403 だった）。policy 付与は失敗で中断し、組織の保存は最後
- 上流 403 のとき、送った platform と operator を `upstream refused the request's scope` として記録

## D4: 委任の同意（D4-a、2026-11-19 確認済み）

`seedLibraryTenant` は、policy 付与のあとで `POST /v1/auth/delegation-consents` を取り込む admin として呼び、
Library の service account（`GET /v1/auth/caller` をサービス資格情報で引いて得る）への同意を記録する。
失敗したら取り込みを失敗させる。tachyon 側は quantum-box/tachyon-apps#14108。**tachyon のデプロイ後に
この変更をデプロイする**（先に出すと同意 API が無く、取り込みが失敗する）。

経緯（A1 が成立しなかった理由）:

tachyon の `authorize_evaluation_tenant`（`packages/auth/src/usecase/delegated_user_subject.rs`）は、
委任 grant を呼び出し元サービスアカウントの acting tenant（Library platform tenant）で評価する。
取り込み先の組織側に置いた grant は参照されないため、合意した A1 は tachyon の変更なしには成立しない。
方式（tachyon 側で同意 grant を評価する / Library に自己付与権限を持たせる / 手作業のまま検出だけ）を改めて決める。

## 形式検証

```bash
python3 scripts/verus/verify.py --all --self-test --mutations
```

- 対象: `scope_platform`、`creation_tenant_matches`、`may_reuse_owner_access`（本番コードが同じ関数を呼ぶ）
- 2026-11-19: verified 6 / errors 0、false postcondition と mutation 7 件すべて拒否
- toolchain は tachyon-apps の `scripts/verus/toolchain.json` と同じ版に固定。CI には入れていない

## 残作業

- [ ] 本番で `SELECT COUNT(*) FROM library.organizations WHERE home_platform_id IS NULL` が 0 になったら、B1 のフォールバックを拒否に切り替え、列を `NOT NULL` にする
- [ ] client の `platformForOperator` / `operatorPlatforms`（`apps/client/src/lib/recordsApi.ts`）を削除する（server が上書きするので動作には影響しない）
- [ ] 組織の取り込みをやめる操作ができたら、同意を撤回する（`DELETE /v1/auth/delegation-consents/{id}`）
- [ ] 既存の取り込み組織（quantumbox、bernard-square ほか）は、取り込み直すと同意が付く。手作業の grant（`library:DelegateBernardSquare` など）は同意の確認後に外せる
- [ ] 失敗した作成で残った database_manager の行の掃除（範囲外の「部分作成のロールバック」）
