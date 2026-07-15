# RecordVersion expand/read rollout

Issue #134 の最初の slice として、Library の Record に永続化 revision を追加する。
この段階は expand/read のみで、CAS write、Outbox、Photon Engine 連携は有効化しない。

## 境界

- `RecordVersion` は Database BC が所有する 1-origin の単調増加 revision。
- Photon Engine の HLC / operation clock は共同編集 operation の順序を表し、
  `RecordVersion` とは相互変換しない。
- API は `BIGINT UNSIGNED` の精度を JavaScript で失わないよう、REST と
  GraphQL の `recordVersion` を10進文字列で返す。
- この slice の write path は version を比較・更新しない。既存の更新処理も
  現在値を維持する。

## Expand

Migration `20260715150000_expand_record_version.sql` は `data` に次を追加する。

```sql
record_version BIGINT UNSIGNED NOT NULL DEFAULT 1
```

`chk_data_record_version_nonzero` により、永続化された version も 0 を拒否する。
既存 row と、column を指定しない旧 write path は version 1 になる。

## Deploy order

1. migration app で expand migration を実行する。
2. 次の schema probe がすべて成立することを確認する。
3. read-capable API をデプロイする。
4. REST / GraphQL の `recordVersion` が文字列 `"1"` 以上であることを確認する。

```sql
SELECT COLUMN_TYPE, IS_NULLABLE, COLUMN_DEFAULT
FROM information_schema.COLUMNS
WHERE TABLE_SCHEMA = 'tachyon_apps_database_manager'
  AND TABLE_NAME = 'data'
  AND COLUMN_NAME = 'record_version';

SELECT CONSTRAINT_NAME, CHECK_CLAUSE
FROM information_schema.CHECK_CONSTRAINTS
WHERE CONSTRAINT_SCHEMA = 'tachyon_apps_database_manager'
  AND CONSTRAINT_NAME = 'chk_data_record_version_nonzero';

SELECT MIN(record_version), MAX(record_version), COUNT(*)
FROM tachyon_apps_database_manager.data;
```

期待値は `bigint unsigned / NO / 1`、CHECK が `record_version > 0`、
既存 row の最小値が 1 以上であること。

## Rollback

read-capable app は安全に旧 app へ戻せる。旧 app は追加 column を無視し、default
により新規 row も version 1 で作成する。CAS consumer が導入される前でも column
は drop しない。rollback は app のみとし、schema は expand 状態を保持する。

## Next slice gate

CAS write を有効化する前に、全 read surface の version parity と API consumer の
文字列取り扱いを確認する。次 slice で `expectedRecordVersion`、atomic
`record_version = record_version + 1`、conflict response、operation idempotency を追加する。
