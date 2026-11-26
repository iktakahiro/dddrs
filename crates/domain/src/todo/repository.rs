use super::{Todo, TodoId};
use crate::error::RepositoryError;
use async_trait::async_trait;

/// Persisted Todo snapshot paired with its optimistic-locking version.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub todo: Todo,
    pub version: i64,
}

/// Persistence port for the Todo aggregate.
///
/// Methods take `&mut self` so implementations can execute directly against an
/// active transaction inside a Unit of Work without interior locks.
#[async_trait]
pub trait TodoRepository: Send {
    async fn insert(&mut self, todo: &Todo) -> Result<(), RepositoryError>;
    async fn find_by_id(&mut self, id: TodoId) -> Result<Option<Snapshot>, RepositoryError>;
    async fn find_latest(&mut self, limit: u32) -> Result<Vec<Todo>, RepositoryError>;
    async fn update(&mut self, todo: &Todo, expected_version: i64) -> Result<(), RepositoryError>;
    async fn delete(&mut self, id: TodoId) -> Result<bool, RepositoryError>;
}
