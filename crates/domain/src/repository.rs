use crate::{StoredTodo, TodoId};
use async_trait::async_trait;
use std::error::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Todo changed since it was read")]
    Conflict,
    #[error("Repository operation failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

pub struct Snapshot {
    pub todo: StoredTodo,
    pub version: i64,
}

#[async_trait]
pub trait TodoRepository: Send + Sync {
    async fn insert(&self, todo: &StoredTodo) -> Result<(), RepositoryError>;
    async fn find(&self, id: TodoId) -> Result<Option<Snapshot>, RepositoryError>;
    async fn latest(&self) -> Result<Vec<StoredTodo>, RepositoryError>;
    async fn update(&self, todo: &StoredTodo, expected_version: i64)
    -> Result<(), RepositoryError>;
    async fn delete(&self, id: TodoId) -> Result<bool, RepositoryError>;
}
