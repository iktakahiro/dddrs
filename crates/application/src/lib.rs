use domain::{
    Description, RepositoryError, Snapshot, StoredTodo, Title, Todo, TodoId, TodoRepository,
    TransitionError,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error("Todo not found")]
    NotFound,
    #[error(transparent)]
    Transition(#[from] TransitionError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Clone)]
pub struct TodoService {
    repository: Arc<dyn TodoRepository>,
}

impl TodoService {
    pub fn new(repository: Arc<dyn TodoRepository>) -> Self {
        Self { repository }
    }

    pub async fn create(
        &self,
        title: Title,
        description: Option<Description>,
    ) -> Result<StoredTodo, UseCaseError> {
        let todo = StoredTodo::NotStarted(Todo::new(title, description));
        self.repository.insert(&todo).await?;
        Ok(todo)
    }

    async fn load(&self, id: TodoId) -> Result<Snapshot, UseCaseError> {
        self.repository
            .find(id)
            .await?
            .ok_or(UseCaseError::NotFound)
    }

    pub async fn get(&self, id: TodoId) -> Result<StoredTodo, UseCaseError> {
        Ok(self.load(id).await?.todo)
    }

    pub async fn latest(&self) -> Result<Vec<StoredTodo>, UseCaseError> {
        Ok(self.repository.latest().await?)
    }

    pub async fn edit(
        &self,
        id: TodoId,
        title: Title,
        description: Option<Description>,
    ) -> Result<StoredTodo, UseCaseError> {
        self.change(id, |mut todo| {
            todo.edit(title, description);
            Ok(todo)
        })
        .await
    }

    pub async fn start(&self, id: TodoId) -> Result<StoredTodo, UseCaseError> {
        self.change(id, StoredTodo::start).await
    }

    pub async fn complete(&self, id: TodoId) -> Result<StoredTodo, UseCaseError> {
        self.change(id, StoredTodo::complete).await
    }

    async fn change(
        &self,
        id: TodoId,
        apply: impl FnOnce(StoredTodo) -> Result<StoredTodo, TransitionError>,
    ) -> Result<StoredTodo, UseCaseError> {
        let snapshot = self.load(id).await?;
        let todo = apply(snapshot.todo)?;
        self.repository.update(&todo, snapshot.version).await?;
        Ok(todo)
    }

    pub async fn delete(&self, id: TodoId) -> Result<(), UseCaseError> {
        if self.repository.delete(id).await? {
            Ok(())
        } else {
            Err(UseCaseError::NotFound)
        }
    }
}
