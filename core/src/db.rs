use crate::task::Task;
use crate::Result;
use log::debug;
use sqlx::{SqliteConnection, SqlitePool};

pub struct Db(SqlitePool);

#[allow(async_fn_in_trait)]
pub trait DbRecord: Sized {
    const TABLE: &'static str;
    async fn upsert_record(&self, conn: &mut SqliteConnection) -> Result<()>;
    async fn delete_record(conn: &mut SqliteConnection, id: &str) -> Result<()>;
    async fn query_dangerous(conn: &mut SqliteConnection, where_clase: &str) -> Result<Vec<Self>>;
}

impl DbRecord for Task {
    const TABLE: &'static str = "tasks";

    async fn upsert_record(&self, conn: &mut SqliteConnection) -> Result<()> {
        let id = self.id().unwrap();
        sqlx::query!(
            "insert or replace into tasks (id, title, status, assignee, description) values (?, ?, ?, ?, ?)",
            id,
            self.title,
            self.status,
            self.assignee,
            self.description,
        )
        .execute(conn)
        .await?;
        Ok(())
    }
    async fn delete_record(conn: &mut SqliteConnection, id: &str) -> Result<()> {
        sqlx::query!("delete from tasks where id = ?", id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
    async fn query_dangerous(conn: &mut SqliteConnection, where_clause: &str) -> Result<Vec<Self>> {
        let sql = format!("select * from tasks where {}", where_clause);
        debug!("query_dangerous: {sql}");
        let records = sqlx::query_as(&sql).fetch_all(&mut *conn).await?;
        Ok(records)
    }
}

impl Db {
    pub async fn connect(db_url: &str) -> Result<Db> {
        debug!("connecting to DB: {db_url}");
        let pool = SqlitePool::connect(&db_url).await?;
        Ok(Db(pool))
    }

    pub async fn upsert_record<D: DbRecord>(&self, record: &D) -> Result<()> {
        let mut conn = self.0.acquire().await?;
        record.upsert_record(&mut *conn).await
    }

    pub async fn delete_record<D: DbRecord>(&self, id: &str) -> Result<()> {
        let mut conn = self.0.acquire().await?;
        D::delete_record(&mut *conn, id).await?;
        Ok(())
    }

    pub async fn query_dangerous<D: DbRecord>(&self, where_clause: &str) -> Result<Vec<D>> {
        let mut conn = self.0.acquire().await?;
        D::query_dangerous(&mut *conn, where_clause).await
    }
}
