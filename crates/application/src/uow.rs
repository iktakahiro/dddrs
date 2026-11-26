//! Transactional Unit of Work ports owned by the application layer.

use async_trait::async_trait;
use domain::{error::RepositoryError, todo::TodoRepository};

/// Active transactional boundary coordinating one or more repositories.
///
/// Mutations performed through repositories returned by a `UnitOfWork` become
/// durable only when [`commit`](Self::commit) succeeds. Calling
/// [`rollback`](Self::rollback) or dropping the `UnitOfWork` discards pending
/// changes.
///
/// When adding another entity (for example, `Project`), add its repository
/// accessor here so cross-entity use cases can commit or roll back changes
/// across multiple aggregates atomically.
#[async_trait]
pub trait UnitOfWork: Send {
    fn todos(&mut self) -> Box<dyn TodoRepository + '_>;

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError>;
}

/// Factory that begins a new transactional [`UnitOfWork`].
#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, RepositoryError>;
}
