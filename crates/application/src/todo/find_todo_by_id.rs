use super::UseCaseError;
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::{Todo, TodoId};
use std::sync::Arc;

/// Use case port for fetching a single Todo by its identifier.
#[async_trait]
pub trait FindTodoByIdUseCase: Send + Sync {
    async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError>;
}

/// Default implementation of [`FindTodoByIdUseCase`].
pub struct FindTodoByIdUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl FindTodoByIdUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl FindTodoByIdUseCase for FindTodoByIdUseCaseImpl {
    async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError> {
        let mut uow = self.uow_factory.begin_read().await?;
        let found = uow.todos().find_by_id(id).await;
        let _ = uow.rollback().await;
        found?
            .map(|snapshot| snapshot.todo)
            .ok_or(UseCaseError::NotFound)
    }
}
