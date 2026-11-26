use super::{row::TodoRow, storage};
use async_trait::async_trait;
use domain::todo::{Description, RepositoryError, Snapshot, StoredTodo, TodoId, TodoRepository};
use sqlx::SqlitePool;

pub struct SqliteTodoRepository {
    pool: SqlitePool,
}

impl SqliteTodoRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TodoRepository for SqliteTodoRepository {
    async fn insert(&self, todo: &StoredTodo) -> Result<(), RepositoryError> {
        let fields = todo.details();
        sqlx::query("INSERT INTO todos (id, title, description, status, created_at, updated_at, completed_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(fields.id.to_string())
            .bind(fields.title.as_str())
            .bind(fields.description.as_ref().map(Description::as_str))
            .bind(todo.status())
            .bind(fields.created_at)
            .bind(fields.updated_at)
            .bind(todo.completed_at())
            .execute(&self.pool).await.map_err(storage)?;
        Ok(())
    }

    async fn find(&self, id: TodoId) -> Result<Option<Snapshot>, RepositoryError> {
        sqlx::query_as::<_, TodoRow>("SELECT * FROM todos WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(Snapshot::try_from)
            .transpose()
    }

    async fn latest(&self) -> Result<Vec<StoredTodo>, RepositoryError> {
        sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todos ORDER BY created_at DESC, rowid DESC LIMIT 20",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?
        .into_iter()
        .map(|row| Ok(Snapshot::try_from(row)?.todo))
        .collect()
    }

    async fn update(
        &self,
        todo: &StoredTodo,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let fields = todo.details();
        let result = sqlx::query("UPDATE todos SET title = ?, description = ?, status = ?, updated_at = ?, completed_at = ?, version = version + 1 WHERE id = ? AND version = ?")
            .bind(fields.title.as_str())
            .bind(fields.description.as_ref().map(Description::as_str))
            .bind(todo.status())
            .bind(fields.updated_at)
            .bind(todo.completed_at())
            .bind(fields.id.to_string())
            .bind(expected_version)
            .execute(&self.pool).await.map_err(storage)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::Conflict);
        }
        Ok(())
    }

    async fn delete(&self, id: TodoId) -> Result<bool, RepositoryError> {
        let result = sqlx::query("DELETE FROM todos WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(storage)?;
        Ok(result.rows_affected() == 1)
    }
}
