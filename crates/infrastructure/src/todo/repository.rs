use super::row::TodoRow;
use crate::sqlite::storage;
use async_trait::async_trait;
use domain::{
    error::RepositoryError,
    todo::{Description, Snapshot, Todo, TodoId, TodoRepository, TodoStatus},
};
use sqlx::{Sqlite, Transaction};

const SELECT_COLUMNS: &str =
    "id, title, description, status, created_at, updated_at, completed_at, version";

fn status_column(status: TodoStatus) -> &'static str {
    match status {
        TodoStatus::NotStarted => "not_started",
        TodoStatus::InProgress => "in_progress",
        TodoStatus::Completed => "completed",
    }
}

pub(crate) struct SqliteTodoRepository<'a> {
    tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> SqliteTodoRepository<'a> {
    pub(crate) fn new(tx: &'a mut Transaction<'static, Sqlite>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl TodoRepository for SqliteTodoRepository<'_> {
    async fn insert(&mut self, todo: &Todo) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO todos (id, title, description, status, created_at, updated_at, completed_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(todo.id().to_string())
        .bind(todo.title().as_str())
        .bind(todo.description().map(Description::as_str))
        .bind(status_column(todo.status()))
        .bind(todo.created_at().timestamp_millis())
        .bind(todo.updated_at().timestamp_millis())
        .bind(todo.completed_at().map(|at| at.timestamp_millis()))
        .execute(&mut **self.tx)
        .await
        .map_err(storage)?;
        Ok(())
    }

    async fn find_by_id(&mut self, id: TodoId) -> Result<Option<Snapshot>, RepositoryError> {
        let query = format!("SELECT {SELECT_COLUMNS} FROM todos WHERE id = ?");
        sqlx::query_as::<_, TodoRow>(&query)
            .bind(id.to_string())
            .fetch_optional(&mut **self.tx)
            .await
            .map_err(storage)?
            .map(Snapshot::try_from)
            .transpose()
    }

    async fn find_latest(&mut self, limit: u32) -> Result<Vec<Todo>, RepositoryError> {
        let query = format!(
            "SELECT {SELECT_COLUMNS} FROM todos ORDER BY created_at DESC, rowid DESC LIMIT ?"
        );
        sqlx::query_as::<_, TodoRow>(&query)
            .bind(i64::from(limit))
            .fetch_all(&mut **self.tx)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| Ok(Snapshot::try_from(row)?.todo))
            .collect()
    }

    async fn update(&mut self, todo: &Todo, expected_version: i64) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE todos SET title = ?, description = ?, status = ?, updated_at = ?, completed_at = ?, version = version + 1 WHERE id = ? AND version = ?",
        )
        .bind(todo.title().as_str())
        .bind(todo.description().map(Description::as_str))
        .bind(status_column(todo.status()))
        .bind(todo.updated_at().timestamp_millis())
        .bind(todo.completed_at().map(|at| at.timestamp_millis()))
        .bind(todo.id().to_string())
        .bind(expected_version)
        .execute(&mut **self.tx)
        .await
        .map_err(storage)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::Conflict);
        }
        Ok(())
    }

    async fn delete(&mut self, id: TodoId) -> Result<bool, RepositoryError> {
        let result = sqlx::query("DELETE FROM todos WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **self.tx)
            .await
            .map_err(storage)?;
        Ok(result.rows_affected() == 1)
    }
}
