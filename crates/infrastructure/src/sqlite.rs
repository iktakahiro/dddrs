//! SQLite connection pool setup and transactional Unit of Work implementation.

use crate::todo::SqliteTodoRepository;
use application::uow::{UnitOfWork, UnitOfWorkFactory};
use async_trait::async_trait;
use domain::{error::RepositoryError, todo::TodoRepository};
use sqlx::{
    Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{error::Error, time::Duration};

/// Open a SQLite pool with foreign keys, WAL mode, and migrations applied.
pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    // In-memory SQLite creates an isolated database per connection unless capped at 1.
    let pool = if url == "sqlite::memory:" {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?
    } else {
        SqlitePool::connect_with(options).await?
    };

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

pub(crate) fn storage(error: impl Error + Send + Sync + 'static) -> RepositoryError {
    RepositoryError::Storage(Box::new(error))
}

/// SQLite-backed implementation of [`UnitOfWorkFactory`].
#[derive(Clone)]
pub struct SqliteUnitOfWorkFactory {
    pool: SqlitePool,
}

impl SqliteUnitOfWorkFactory {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Active SQLite transaction implementing [`UnitOfWork`].
pub struct SqliteUnitOfWork {
    tx: Transaction<'static, Sqlite>,
}

#[async_trait]
impl UnitOfWorkFactory for SqliteUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, RepositoryError> {
        let tx = self.pool.begin().await.map_err(storage)?;
        Ok(Box::new(SqliteUnitOfWork { tx }))
    }
}

#[async_trait]
impl UnitOfWork for SqliteUnitOfWork {
    fn todos(&mut self) -> Box<dyn TodoRepository + '_> {
        Box::new(SqliteTodoRepository::new(&mut self.tx))
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(storage)
    }

    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.rollback().await.map_err(storage)
    }
}
