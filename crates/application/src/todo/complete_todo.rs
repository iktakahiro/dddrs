use super::{UseCaseError, update_in_tx};
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::{Todo, TodoId};
use std::sync::Arc;

/// Use case port for transitioning a Todo from `InProgress` to `Completed`.
#[async_trait]
pub trait CompleteTodoUseCase: Send + Sync {
    async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError>;
}

/// Default transactional implementation of [`CompleteTodoUseCase`].
pub struct CompleteTodoUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl CompleteTodoUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl CompleteTodoUseCase for CompleteTodoUseCaseImpl {
    async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError> {
        update_in_tx(self.uow_factory.as_ref(), id, Todo::complete).await
    }
}
