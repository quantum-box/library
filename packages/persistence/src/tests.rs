//! TODO: add English documentation
//!
//! ## set_up
//!
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! ## clean_up
//!
//! TODO: add English documentation
//!
use std::{fmt::Debug, sync::Arc};

use sqlx::{Executor, MySql, Pool};

#[async_trait::async_trait]
pub trait Migration: Debug + Send + Sync {
    async fn migration(&self);
    fn get_db(&self) -> Arc<Pool<MySql>>;
}

pub async fn set_up(pool: Arc<Pool<MySql>>) -> errors::Result<()> {
    // TODO: add English comment
    pool.execute("SET FOREIGN_KEY_CHECKS = 0;")
        .await
        .expect("外部キー制約の無効化に失敗しました");

    Ok(())
}

/// TODO: add English documentation
pub async fn clean_up(pool: Arc<Pool<MySql>>) -> errors::Result<()> {
    // TODO: add English comment
    pool.execute("SET FOREIGN_KEY_CHECKS = 0;")
        .await
        .expect("外部キー制約の無効化に失敗しました");

    // TODO: add English comment
    let tables: Vec<(Vec<u8>,)> = sqlx::query_as("SHOW TABLES")
        .fetch_all(&*pool)
        .await
        .expect("テーブル一覧の取得に失敗しました");

    for (table,) in tables {
        let table_name = String::from_utf8(table)
            .expect("テーブル名のUTF-8変換に失敗しました");
        sqlx::query(&format!("TRUNCATE TABLE `{table_name}`;"))
            .execute(&*pool)
            .await
            .expect("テーブルのデータ削除に失敗しました");
    }

    pool.execute("SET FOREIGN_KEY_CHECKS = 1;")
        .await
        .expect("外部キー制約の再有効化に失敗しました");

    Ok(())
}

#[cfg(feature = "integration_tests")]
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::Db;
    use std::env;

    async fn inner_test() -> errors::Result<()> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or("mysql://root:@localhost:15000/test".to_string());

        let db = Db::new(&database_url).await;

        clean_up(db.pool()).await?;

        sqlx::migrate!("./migrations")
            .run(db.pool().as_ref())
            .await
            .expect("migration failed");
        set_up(db.pool().clone()).await?;

        // TODO: add English comment
        sqlx::query(
            "insert into posts
                (title, content, user_id)
            values
                ('test', 'test', 1)",
        )
        .execute(db.pool().as_ref())
        .await
        .expect("ユーザーの追加に失敗しました");

        let posts = sqlx::query!("select * from persistence_test.posts")
            .fetch_all(db.pool().as_ref())
            .await
            .expect("ユーザー一覧の取得に失敗しました");
        let users = sqlx::query!("select * from persistence_test.users")
            .fetch_all(db.pool().as_ref())
            .await
            .expect("ユーザー一覧の取得に失敗しました");

        dbg!(&posts);

        assert_eq!(posts.len(), 1);
        assert_eq!(users.len(), 0);
        assert_eq!(posts[0].title, "test".to_string());
        assert_eq!(posts[0].content, "test".to_string());
        assert_eq!(posts[0].user_id, 1);

        Ok(())
    }

    // TODO: add English comment
    // #[tokio::test]
    // async fn test_set_up_and_clean_up() -> errors::Result<()> {
    //     inner_test().await
    // }
}
