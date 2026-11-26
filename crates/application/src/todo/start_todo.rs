use super::{UseCaseError, update_in_tx};
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::{Todo, TodoId};
use std::sync::Arc;

/// Use case port for transitioning a Todo from `NotStarted` to `InProgress`.
#[async_trait]
pub trait StartTodoUseCase: Send + Sync {
    async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError>;
}

/// Default transactional implementation of [`StartTodoUseCase`].
pub struct StartTodoUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl StartTodoUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl StartTodoUseCase for StartTodoUseCaseImpl {
    async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError> {
        update_in_tx(self.uow_factory.as_ref(), id, Todo::start).await
    }
}
