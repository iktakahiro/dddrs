use super::UseCaseError;
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::Todo;
use std::sync::Arc;

/// Maximum number of Todos returned by the default listing use case.
pub const DEFAULT_LIST_LIMIT: u32 = 20;

/// Use case port for listing recent Todos.
#[async_trait]
pub trait FindTodosUseCase: Send + Sync {
    async fn execute(&self) -> Result<Vec<Todo>, UseCaseError>;
}

/// Default implementation of [`FindTodosUseCase`].
pub struct FindTodosUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl FindTodosUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl FindTodosUseCase for FindTodosUseCaseImpl {
    async fn execute(&self) -> Result<Vec<Todo>, UseCaseError> {
        let mut uow = self.uow_factory.begin().await?;
        let todos = uow.todos().find_latest(DEFAULT_LIST_LIMIT).await;
        let _ = uow.rollback().await;
        Ok(todos?)
    }
}
