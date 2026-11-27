//! Todo application use cases, one struct and one `execute` method per use case.

mod complete_todo;
mod create_todo;
mod delete_todo;
mod error;
mod find_todo_by_id;
mod find_todos;
mod start_todo;
mod update_todo;

pub use complete_todo::{CompleteTodoUseCase, CompleteTodoUseCaseImpl};
pub use create_todo::{CreateTodoUseCase, CreateTodoUseCaseImpl};
pub use delete_todo::{DeleteTodoUseCase, DeleteTodoUseCaseImpl};
pub use error::UseCaseError;
pub use find_todo_by_id::{FindTodoByIdUseCase, FindTodoByIdUseCaseImpl};
pub use find_todos::{DEFAULT_LIST_LIMIT, FindTodosUseCase, FindTodosUseCaseImpl};
pub use start_todo::{StartTodoUseCase, StartTodoUseCaseImpl};
pub use update_todo::{UpdateTodoUseCase, UpdateTodoUseCaseImpl};

use crate::uow::UnitOfWorkFactory;
use domain::todo::{Todo, TodoId, TransitionError};

/// Load a Todo, apply a domain mutation, persist with optimistic locking, and commit.
async fn update_in_tx(
    uow_factory: &dyn UnitOfWorkFactory,
    id: TodoId,
    mutate: impl FnOnce(&mut Todo) -> Result<(), TransitionError>,
) -> Result<Todo, UseCaseError> {
    let mut uow = uow_factory.begin().await?;
    let result: Result<Todo, UseCaseError> = async {
        let snapshot = uow
            .todos()
            .find_by_id(id)
            .await?
            .ok_or(UseCaseError::NotFound)?;
        let mut todo = snapshot.todo;
        mutate(&mut todo)?;
        uow.todos().update(&todo, snapshot.version).await?;
        Ok(todo)
    }
    .await;

    match result {
        Ok(todo) => {
            uow.commit().await?;
            Ok(todo)
        }
        Err(error) => {
            let _ = uow.rollback().await;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests;
