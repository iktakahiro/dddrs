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
    /// Begin a write-capable transaction.
    ///
    /// Implementations backed by SQLite WAL should acquire the write lock up
    /// front (`BEGIN IMMEDIATE`) so concurrent read-then-write updates wait for
    /// `busy_timeout` instead of failing immediately with `SQLITE_BUSY_SNAPSHOT`.
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, RepositoryError>;

    /// Begin a read-only transaction snapshot.
    ///
    /// Defaults to [`begin`](Self::begin); database backends such as SQLite WAL
    /// override this with a deferred read transaction so concurrent readers do
    /// not contend for the writer lock.
    async fn begin_read(&self) -> Result<Box<dyn UnitOfWork>, RepositoryError> {
        self.begin().await
    }
}
