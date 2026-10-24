//! Runs only against an explicitly supplied disposable database.
use sqlx::mysql::MySqlPoolOptions;
use strategy_projection::{
    store::{DocumentRelease, Store},
    *,
};
fn reference(kind: &str, id: &str) -> ResourceRef {
    ResourceRef {
        provider: "linear".into(),
        workspace_id: "ws".into(),
        kind: kind.into(),
        external_id: id.into(),
    }
}
fn event(seq: u64, kind: &str, id: &str) -> Change {
    let r = reference(kind, id);
    Change {
        subscription_id: "sub".into(),
        generation: 1,
        sequence: seq,
        event_id: format!("e{seq}"),
        resource_ref: r.clone(),
        observed_revision: seq,
        operation: Operation::Upsert,
        projection: Some(Projection {
            resource_ref: r,
            name: id.into(),
            url: None,
            status: None,
            owner: None,
            target_date: None,
            source_updated_at: None,
            archived: false,
        }),
        relation: None,
        schema_version: 1,
        observed_at: "2026-10-24T00:00:00Z".into(),
    }
}
fn page(changes: Vec<Change>) -> ChangePage {
    ChangePage {
        generation: 1,
        changes,
        next_cursor: None,
    }
}
#[tokio::test]
#[ignore = "requires isolated LIBRARY_STRATEGY_TEST_DATABASE_URL"]
async fn transactional_replay_relations_revocation_and_publication() {
    let url = std::env::var("LIBRARY_STRATEGY_TEST_DATABASE_URL")
        .expect("explicit disposable database required");
    let target = url::Url::parse(&url).unwrap();
    assert!(matches!(target.host_str(),Some("127.0.0.1"|"localhost")) && target.path()=="/strategy_test","test requires an explicitly disposable loopback strategy_test database");
    let pool = MySqlPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    for table in [
        "library_pm_migration_items",
        "library_pm_migration_plans",
        "library_pm_events",
        "library_pm_relations",
        "library_pm_records",
        "library_pm_bindings",
        "library_document_releases",
        "library_pm_writer_fences",
    ] {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    for statement in include_str!("../../../apps/api/migrations/20261024000000_strategy_projection.up.sql").split(';') {if !statement.trim().is_empty(){sqlx::query(statement).execute(&pool).await.unwrap();}}
    for statement in include_str!("../../../apps/api/migrations/20261024010000_pm_reviewed_adoption.up.sql").split(';') {if !statement.trim().is_empty(){sqlx::query(statement).execute(&pool).await.unwrap();}}
    let store = Store::new(pool.clone());
    store
        .connect("repo", "tenant", "sub", "ws", 1)
        .await
        .unwrap();
    assert!(store
        .apply_page("repo", &page(vec![event(1, "project", "p")]))
        .await
        .is_err());
    store.activate_tachyon_writer("repo").await.unwrap();
    let mut writer_guard = pool.begin().await.unwrap();
    sqlx::query("SELECT writer FROM library_pm_writer_fences WHERE repo_id='repo' FOR UPDATE").fetch_one(&mut *writer_guard).await.unwrap();
    let other = store.clone();
    let waiting =
        tokio::spawn(async move { other.pause_projection("repo").await });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(!waiting.is_finished(), "cutover must drain the old writer");
    writer_guard.commit().await.unwrap();
    waiting.await.unwrap().unwrap();
    store.activate_tachyon_writer("repo").await.unwrap();
    let initial = page(vec![event(1, "initiative", "a")]);
    assert_eq!(store.apply_page("repo", &initial).await.unwrap(), 1);
    let original = store
        .list("repo", None, 50)
        .await
        .unwrap()
        .records
        .remove(0)
        .id;
    store
        .record_local_context(
            "repo",
            &original,
            "local analysis",
            &["doc".into()],
        )
        .await
        .unwrap();
    assert_eq!(store.apply_page("repo", &initial).await.unwrap(), 1); // crash before ack
    let mut changed = initial.clone();
    changed.changes[0].projection.as_mut().unwrap().name = "corrupt".into();
    assert!(store.apply_page("repo", &changed).await.is_err());
    let mut edge = event(2, "initiative", "a");
    edge.operation = Operation::RelationChange;
    edge.projection = None;
    edge.relation = Some(Relation {
        source: reference("initiative", "a"),
        target: reference("project", "p"),
        removed: false,
    });
    store.apply_page("repo", &page(vec![edge])).await.unwrap();
    assert_eq!(
        store
            .list("repo", None, 50)
            .await
            .unwrap()
            .pending_relations,
        1
    );
    store
        .apply_page("repo", &page(vec![event(3, "project", "p")]))
        .await
        .unwrap();
    assert_eq!(
        store.list("repo", None, 50).await.unwrap().relations.len(),
        1
    );
    // A whole page rolls back when a later event is invalid.
    assert!(store
        .apply_page(
            "repo",
            &page(vec![event(4, "project", "q"), event(6, "project", "r")])
        )
        .await
        .is_err());
    assert_eq!(store.binding("repo").await.unwrap().checkpoint, 3);
    assert!(store
        .resolve("repo", &reference("project", "q"))
        .await
        .unwrap()
        .is_none());
    let mut stale = event(4, "initiative", "a");
    stale.observed_revision = 1;
    stale.projection.as_mut().unwrap().name = "stale".into();
    store.apply_page("repo", &page(vec![stale])).await.unwrap();
    let record = store
        .list("repo", None, 50)
        .await
        .unwrap()
        .records
        .into_iter()
        .find(|r| r.id == original)
        .unwrap();
    assert_eq!(record.projection.name, "a");
    assert_eq!(record.note, "local analysis");
    // Multiple parents resolve to one stable project object.
    store
        .apply_page("repo", &page(vec![event(5, "initiative", "b")]))
        .await
        .unwrap();
    let mut edge = event(6, "initiative", "b");
    edge.operation = Operation::RelationChange;
    edge.projection = None;
    edge.relation = Some(Relation {
        source: reference("initiative", "b"),
        target: reference("project", "p"),
        removed: false,
    });
    store.apply_page("repo", &page(vec![edge])).await.unwrap();
    let list = store.list("repo", None, 50).await.unwrap();
    assert_eq!(list.relations.len(), 2);
    assert_eq!(list.records.len(), 3);
    let mut archived = event(7, "project", "p");
    archived.operation = Operation::Archived;
    archived.projection.as_mut().unwrap().archived = true;
    store
        .apply_page("repo", &page(vec![archived]))
        .await
        .unwrap();
    assert_eq!(
        store.list("repo", None, 50).await.unwrap().records.len(),
        3
    );
    assert!(
        store
            .replace_snapshot("repo", 1, 6, vec![], vec![])
            .await
            .is_err(),
        "snapshot must not regress concurrent checkpoint"
    );
    store.pause_projection("repo").await.unwrap();
    assert!(store
        .list("repo", None, 50)
        .await
        .unwrap()
        .records
        .is_empty());
    store.begin_resync("repo", 2).await.unwrap();
    store.activate_tachyon_writer("repo").await.unwrap();
    let mut restored = event(1, "initiative", "a");
    restored.generation = 2;
    store
        .apply_page(
            "repo",
            &ChangePage {
                generation: 2,
                changes: vec![restored],
                next_cursor: None,
            },
        )
        .await
        .unwrap();
    let restored = store
        .list("repo", None, 50)
        .await
        .unwrap()
        .records
        .remove(0);
    assert_eq!(restored.id, original);
    assert_eq!(restored.note, "local analysis");
    let release = DocumentRelease {
        id: "rel1".into(),
        data_id: "doc".into(),
        source_version: "1".into(),
        title: "MVV".into(),
        body: "original".into(),
        published_by: "user".into(),
        published_at: "2026-10-24".into(),
        revoked: false,
    };
    store.publish("repo", release.clone()).await.unwrap();
    let mut retry = release.clone();
    retry.id = "rel2".into();
    assert_eq!(
        store.publish("repo", retry.clone()).await.unwrap().id,
        "rel1"
    );
    retry.body = "edited".into();
    assert!(store.publish("repo", retry).await.is_err());
    let mut revised = release;
    revised.id = "rel3".into();
    revised.source_version = "2".into();
    revised.body = "revised".into();
    store.publish("repo", revised).await.unwrap();
    assert_eq!(
        store.release("repo", "rel1").await.unwrap().body,
        "original"
    );
    store.revoke_release("repo", "rel1").await.unwrap();
    assert!(store.release("repo", "rel1").await.unwrap().revoked);
}
