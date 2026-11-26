use super::UseCaseError;
use crate::uow::UnitOfWorkFactory;
use async_trait::async_trait;
use domain::todo::TodoId;
use std::sync::Arc;

/// Use case port for deleting a Todo by its identifier.
#[async_trait]
pub trait DeleteTodoUseCase: Send + Sync {
    async fn execute(&self, id: TodoId) -> Result<(), UseCaseError>;
}

/// Default transactional implementation of [`DeleteTodoUseCase`].
pub struct DeleteTodoUseCaseImpl {
    uow_factory: Arc<dyn UnitOfWorkFactory>,
}

impl DeleteTodoUseCaseImpl {
    pub fn new(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl DeleteTodoUseCase for DeleteTodoUseCaseImpl {
    async fn execute(&self, id: TodoId) -> Result<(), UseCaseError> {
        let mut uow = self.uow_factory.begin().await?;
        let deleted = uow.todos().delete(id).await;
        let deleted = match deleted {
            Ok(deleted) => deleted,
            Err(error) => {
                let _ = uow.rollback().await;
                return Err(error.into());
            }
        };
        if !deleted {
            let _ = uow.rollback().await;
            return Err(UseCaseError::NotFound);
        }
        uow.commit().await?;
        Ok(())
    }
}
