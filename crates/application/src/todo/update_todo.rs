use super::{UseCaseError, update_in_tx};
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::{Description, Title, Todo, TodoId};
use std::sync::Arc;

/// Use case port for replacing a Todo's title and description.
#[async_trait]
pub trait UpdateTodoUseCase: Send + Sync {
    async fn execute(
        &self,
        id: TodoId,
        title: Title,
        description: Option<Description>,
    ) -> Result<Todo, UseCaseError>;
}

/// Default transactional implementation of [`UpdateTodoUseCase`].
pub struct UpdateTodoUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UpdateTodoUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl UpdateTodoUseCase for UpdateTodoUseCaseImpl {
    async fn execute(
        &self,
        id: TodoId,
        title: Title,
        description: Option<Description>,
    ) -> Result<Todo, UseCaseError> {
        update_in_tx(self.uow_factory.as_ref(), id, |todo| {
            todo.update(title, description);
            Ok(())
        })
        .await
    }
}
