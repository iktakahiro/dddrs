use super::UseCaseError;
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::{Description, Title, Todo};
use std::sync::Arc;

/// Use case port for creating a new Todo.
#[async_trait]
pub trait CreateTodoUseCase: Send + Sync {
    async fn execute(
        &self,
        title: Title,
        description: Option<Description>,
    ) -> Result<Todo, UseCaseError>;
}

/// Default transactional implementation of [`CreateTodoUseCase`].
pub struct CreateTodoUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl CreateTodoUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl CreateTodoUseCase for CreateTodoUseCaseImpl {
    async fn execute(
        &self,
        title: Title,
        description: Option<Description>,
    ) -> Result<Todo, UseCaseError> {
        let mut uow = self.uow_factory.begin().await?;
        let todo = Todo::new(title, description);
        let inserted = uow.todos().insert(&todo).await;
        if let Err(error) = inserted {
            let _ = uow.rollback().await;
            return Err(error.into());
        }
        uow.commit().await?;
        Ok(todo)
    }
}
