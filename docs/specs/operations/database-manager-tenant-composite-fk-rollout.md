# Database Manager tenant composite FK rollout

## 目的

`tachyon_apps_database_manager` の参照整合性へ `tenant_id` を含め、アプリケーションの検証を迂回した SQL でも別 tenant の database、field、data を参照できないようにする。対象 migration は `20260715090000_add_tenant_composite_foreign_keys.sql` であり、既存データの更新・削除は行わない。

新しい制約は次のとおり。

| Child | Foreign key | Columns | Parent key |
| --- | --- | --- | --- |
| `fields` | `fk_fields_tenant_object` | `(tenant_id, object_id)` | `objects(tenant_id, id)` |
| `data` | `fk_data_tenant_object` | `(tenant_id, object_id)` | `objects(tenant_id, id)` |
| `indexes` | `fk_indexes_tenant_data` | `(tenant_id, object_id)` | `data(tenant_id, id)` |
| `relationships` | `fk_relationships_tenant_object` | `(tenant_id, object_id)` | `objects(tenant_id, id)` |
| `relationships` | `fk_relationships_tenant_target_object` | `(tenant_id, target_object_id)` | `objects(tenant_id, id)` |
| `relationships` | `fk_relationships_tenant_object_field` | `(tenant_id, object_id, field_id)` | `fields(tenant_id, object_id, id)` |

追加するunique / support indexは次のとおり。

| Table | Index | Columns |
| --- | --- | --- |
| `objects` | `uq_objects_tenant_id_id` | `(tenant_id, id)` |
| `fields` | `uq_fields_tenant_object_id_id` | `(tenant_id, object_id, id)` |
| `data` | `uq_data_tenant_id_id` | `(tenant_id, id)` |
| `data` | `idx_data_tenant_object_id` | `(tenant_id, object_id)` |
| `indexes` | `idx_indexes_tenant_data_id` | `(tenant_id, object_id)` |
| `relationships` | `idx_relationships_tenant_object_field_id` | `(tenant_id, object_id, field_id)` |
| `relationships` | `idx_relationships_tenant_target_object_id` | `(tenant_id, target_object_id)` |

`indexes.object_id` は data record id を保持する既存の列名であり、列名の変更はこの rollout に含めない。

## MySQL 8 の前提とロック

- MySQL は foreign key の参照列と子列に、同じ列順を先頭に持つ index を必要とする。このため `uq_objects_tenant_id_id`、`uq_fields_tenant_object_id_id`、`uq_data_tenant_id_id` と tenant-leading child index を先に作る。[MySQL 8.0 FOREIGN KEY Constraints](https://dev.mysql.com/doc/refman/8.0/en/create-table-foreign-keys.html)
- `FOREIGN_KEY_CHECKS=1` では foreign key 追加に `ALGORITHM=COPY` が必要となり、既存行の検査と書き込み停止を伴う。index DDL も開始・終了時に metadata lock を取得する。長時間 transaction を解消し、書き込みをdrainしたmaintenance windowで実行する。[MySQL 8.0 Online DDL Operations](https://dev.mysql.com/doc/refman/8.0/en/innodb-online-ddl-operations.html), [Online DDL Limitations](https://dev.mysql.com/doc/refman/8.0/en/innodb-online-ddl-limitations.html)
- `FOREIGN_KEY_CHECKS` は無効化しない。無効化中に入った行は再有効化しても再検査されないため、この migration は checks が有効な状態で constraint 追加を失敗させる。
- 新しい複合FKを旧single-column FKと併存させ、全ての複合FK追加が成功した後に旧FKを削除する。検査失敗時にも旧制約は残る。

## rollout前のpreflight

以下は `tachyon_apps_database_manager` を選択したread-only sessionで実行する。最初のqueryは `foreign_key_checks = 1` でなければ中止する。

```sql
SELECT VERSION() AS mysql_version,
       DATABASE() AS database_name,
       @@SESSION.foreign_key_checks AS foreign_key_checks;

SELECT trx_id, trx_started, trx_state, trx_mysql_thread_id, trx_query
FROM information_schema.innodb_trx
ORDER BY trx_started;

SELECT table_name, table_rows, data_length, index_length
FROM information_schema.tables
WHERE table_schema = DATABASE()
  AND table_name IN (
    'objects', 'fields', 'data', 'indexes', 'relationships'
  )
ORDER BY table_name;
```

次の件数queryは全行が `orphan_count = 0` でなければならない。

```sql
SELECT 'fields_tenant_object' AS check_name, COUNT(*) AS orphan_count
FROM fields AS f
LEFT JOIN objects AS o
  ON o.tenant_id = f.tenant_id
 AND o.id = f.object_id
WHERE o.id IS NULL
UNION ALL
SELECT 'data_tenant_object', COUNT(*)
FROM data AS d
LEFT JOIN objects AS o
  ON o.tenant_id = d.tenant_id
 AND o.id = d.object_id
WHERE o.id IS NULL
UNION ALL
SELECT 'indexes_tenant_data', COUNT(*)
FROM indexes AS i
LEFT JOIN data AS d
  ON d.tenant_id = i.tenant_id
 AND d.id = i.object_id
WHERE d.id IS NULL
UNION ALL
SELECT 'relationships_tenant_object', COUNT(*)
FROM relationships AS r
LEFT JOIN objects AS o
  ON o.tenant_id = r.tenant_id
 AND o.id = r.object_id
WHERE o.id IS NULL
UNION ALL
SELECT 'relationships_tenant_target_object', COUNT(*)
FROM relationships AS r
LEFT JOIN objects AS o
  ON o.tenant_id = r.tenant_id
 AND o.id = r.target_object_id
WHERE o.id IS NULL
UNION ALL
SELECT 'relationships_tenant_object_field', COUNT(*)
FROM relationships AS r
LEFT JOIN fields AS f
  ON f.tenant_id = r.tenant_id
 AND f.object_id = r.object_id
 AND f.id = r.field_id
WHERE f.id IS NULL;
```

0でないcheckがあれば、対応する次の詳細queryで行を特定する。1行でも返った場合はdeployを中止し、データ修復を別変更としてreviewする。このmigration自体は行を修復しない。

```sql
-- fields(tenant_id, object_id) -> objects(tenant_id, id)
SELECT f.id, f.tenant_id, f.object_id
FROM fields AS f
LEFT JOIN objects AS o
  ON o.tenant_id = f.tenant_id
 AND o.id = f.object_id
WHERE o.id IS NULL;

-- data(tenant_id, object_id) -> objects(tenant_id, id)
SELECT d.id, d.tenant_id, d.object_id
FROM data AS d
LEFT JOIN objects AS o
  ON o.tenant_id = d.tenant_id
 AND o.id = d.object_id
WHERE o.id IS NULL;

-- indexes.object_id is the referenced data id
SELECT i.id, i.tenant_id, i.object_id AS data_id
FROM indexes AS i
LEFT JOIN data AS d
  ON d.tenant_id = i.tenant_id
 AND d.id = i.object_id
WHERE d.id IS NULL;

-- relationships source database
SELECT r.id, r.tenant_id, r.object_id
FROM relationships AS r
LEFT JOIN objects AS o
  ON o.tenant_id = r.tenant_id
 AND o.id = r.object_id
WHERE o.id IS NULL;

-- relationships target database
SELECT r.id, r.tenant_id, r.target_object_id
FROM relationships AS r
LEFT JOIN objects AS o
  ON o.tenant_id = r.tenant_id
 AND o.id = r.target_object_id
WHERE o.id IS NULL;

-- relationships field must belong to the source database and tenant
SELECT r.id, r.tenant_id, r.object_id, r.field_id
FROM relationships AS r
LEFT JOIN fields AS f
  ON f.tenant_id = r.tenant_id
 AND f.object_id = r.object_id
 AND f.id = r.field_id
WHERE f.id IS NULL;
```

## rollout

1. Database snapshot / backup pointを記録する。
2. API、worker、手動SQLの書き込みをdrainし、長時間transactionがないことをpreflight queryで再確認する。
3. `FOREIGN_KEY_CHECKS` を変更せず、既存migration runnerから実行する。

```bash
PROD_DATABASE_URL='<admin-dsn>' \
  cargo run -p database-manager --bin database_manager_migrate prod
```

4. 次のqueryでFKの列順と参照列を確認する。

```sql
SELECT TABLE_NAME,
       CONSTRAINT_NAME,
       ORDINAL_POSITION,
       COLUMN_NAME,
       REFERENCED_TABLE_NAME,
       REFERENCED_COLUMN_NAME
FROM information_schema.KEY_COLUMN_USAGE
WHERE TABLE_SCHEMA = DATABASE()
  AND CONSTRAINT_NAME IN (
    'fk_fields_tenant_object',
    'fk_data_tenant_object',
    'fk_indexes_tenant_data',
    'fk_relationships_tenant_object',
    'fk_relationships_tenant_target_object',
    'fk_relationships_tenant_object_field'
  )
ORDER BY TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION;

SELECT TABLE_NAME, CONSTRAINT_NAME
FROM information_schema.TABLE_CONSTRAINTS
WHERE CONSTRAINT_SCHEMA = DATABASE()
  AND CONSTRAINT_NAME IN (
    'fk_fields_objects',
    'fk_data_objects',
    'fk_indexes_data',
    'fk_relationships_object_id',
    'fk_relationships_target_object_id',
    'fk_relationships_field_id'
  );
```

1つ目は6制約の全列が上表の順番で返り、2つ目は0行であることを確認する。その後write trafficを再開し、API errorとDB constraint violationを監視する。

## failure時

- constraint追加で失敗した場合は、エラーに含まれるconstraint名と上記孤児queryを照合する。`FOREIGN_KEY_CHECKS=0` で通過させない。
- MySQL DDLはstatement単位でcommitされるため、migration途中までのcandidate key / composite FKが残る場合がある。次のqueryで実状態を確認する。

```sql
SELECT TABLE_NAME, CONSTRAINT_NAME, CONSTRAINT_TYPE
FROM information_schema.TABLE_CONSTRAINTS
WHERE CONSTRAINT_SCHEMA = DATABASE()
  AND TABLE_NAME IN (
    'objects', 'fields', 'data', 'indexes', 'relationships'
  )
ORDER BY TABLE_NAME, CONSTRAINT_NAME;
```

- 失敗したmigrationを再試行する場合は、下記rollbackのうち実在する新制約・indexだけを削除し、旧FKが存在することを確認する。schemaを元に戻した後に限り、失敗記録を削除できる。

```sql
DELETE FROM _sqlx_migrations
WHERE version = 20260715090000
  AND success = FALSE;
```

## rollback

このmigrationはforward-onlyである。完全適用後の緊急rollbackでは、まず旧FKを再追加してから新FKを削除し、制約の空白期間を作らない。`FOREIGN_KEY_CHECKS` は常に有効のままにする。

```sql
ALTER TABLE fields
  ADD CONSTRAINT fk_fields_objects
    FOREIGN KEY (object_id) REFERENCES objects (id);

ALTER TABLE data
  ADD CONSTRAINT fk_data_objects
    FOREIGN KEY (object_id) REFERENCES objects (id);

ALTER TABLE indexes
  ADD CONSTRAINT fk_indexes_data
    FOREIGN KEY (object_id) REFERENCES data (id);

ALTER TABLE relationships
  ADD CONSTRAINT fk_relationships_object_id
    FOREIGN KEY (object_id) REFERENCES objects (id),
  ADD CONSTRAINT fk_relationships_target_object_id
    FOREIGN KEY (target_object_id) REFERENCES objects (id),
  ADD CONSTRAINT fk_relationships_field_id
    FOREIGN KEY (field_id) REFERENCES fields (id);

ALTER TABLE fields
  DROP FOREIGN KEY fk_fields_tenant_object;

ALTER TABLE data
  DROP FOREIGN KEY fk_data_tenant_object;

ALTER TABLE indexes
  DROP FOREIGN KEY fk_indexes_tenant_data;

ALTER TABLE relationships
  DROP FOREIGN KEY fk_relationships_tenant_object,
  DROP FOREIGN KEY fk_relationships_tenant_target_object,
  DROP FOREIGN KEY fk_relationships_tenant_object_field;

ALTER TABLE relationships
  DROP INDEX idx_relationships_tenant_object_field_id,
  DROP INDEX idx_relationships_tenant_target_object_id;

ALTER TABLE indexes
  DROP INDEX idx_indexes_tenant_data_id;

ALTER TABLE data
  DROP INDEX idx_data_tenant_object_id,
  DROP INDEX uq_data_tenant_id_id;

ALTER TABLE fields
  DROP INDEX uq_fields_tenant_object_id_id;

ALTER TABLE objects
  DROP INDEX uq_objects_tenant_id_id;
```

成功済みの `_sqlx_migrations` 行や適用済みmigration fileは変更しない。rollback後に再導入する場合は、現状schemaから進む新しいforward migrationを追加する。
