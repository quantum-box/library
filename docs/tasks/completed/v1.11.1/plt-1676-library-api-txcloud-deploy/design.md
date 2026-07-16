# Library API databaseRef migration

## Context

Library API と migration Lambda は、Library 専用 TiDB cluster の接続 URL を
`library-api/DATABASE_URL` provider secret から受け取っている。この方式では
DB credential の所有者がアプリケーション secret として見え、TiDB resource
の lifecycle と runtime env の関係が manifest 上で表現されない。

Tachyon Field API は `databaseRef` を使い、DB resource が所有する `url` field
から `DATABASE_URL` を materialize している。Library も同じ credential ownership
model に揃える。ただし cluster、database、pool sizing、migration lifecycle は
Library 固有のままとする。

## Goals

- Library 専用 TiDB cluster を `tidb_library_api_prod` として参照する。
- API と migration Lambda が同じ DB credential source を使う。
- secret 値を Git、plan output、build log に露出しない。
- migration Lambda を先に検証してから API runtime を切り替えられるようにする。

## Non-goals

- Tachyon Field の TiDB cluster を共有しない。
- `library` と `tachyon_apps_database_manager` の schema 構成を変更しない。
- DB pool 上限や migration SQL を変更しない。
- `SERVICE_AUTH_TOKEN`、Sentry、OAuth credential の所有モデルを変更しない。

## Proposed design

`library-api`、`library-api-migrate` の `DATABASE_URL` と、migration CLI互換の
`PROD_DATABASE_URL` を次の参照へ統一する。

```yaml
type: credential
valueFrom:
  databaseRef:
    name: tidb_library_api_prod
    field: url
```

Tachyon は apply 時に Library tenant 内の database credentialを解決し、各
Cloud App の runtime env secretへmaterializeする。アプリケーションは従来どおり
`DATABASE_URL` を読み、`library` と `tachyon_apps_database_manager` を選択する。

## Rollout

1. Library tenant `tn_01j91h09tpj5ehwbwfwfxpak2b` に、Library専用clusterを指す
   `tidb_library_api_prod` の `url` fieldが存在することを確認する。
   未作成の場合は `scripts/backfill-library-database-ref-secret.sh` で既存の
   production app-env secretから値を表示せず作成する。
2. manifest validate と planを実行し、credential source未解決ならapplyしない。
3. `library-api-migrate` をapplyし、migration Lambdaが成功することを確認する。
4. `library-api` をapplyし、`/`、`/health`、認証付きread/write smokeを確認する。
5. 問題があれば manifestを従来の `library-api/DATABASE_URL` referenceへ戻し、
   直前の正常deploymentへrollbackする。

## Verification

- `tachyon manifest validate -f tachyon.yaml`
- `tachyon manifest plan -f tachyon.yaml --app library-api-migrate --environment production`
- `tachyon manifest plan -f tachyon.yaml --app library-api --environment production`
- `bash -n scripts/build-library-api-migrate-lambda.sh`
- migration Lambda成功と `_sqlx_migrations` の確認
- production healthおよび認証付きCRUD smoke

## ADR decision

新規ADRは作成しない。`databaseRef` はTachyonの既存credential source modelであり、
本変更はLibraryへの適用とrollout設計に限定される。
