//! Composition root wiring use cases, repositories, and HTTP routers.

use application::{
    todo::{
        CompleteTodoUseCaseImpl, CreateTodoUseCaseImpl, DeleteTodoUseCaseImpl,
        FindTodoByIdUseCaseImpl, FindTodosUseCaseImpl, StartTodoUseCaseImpl, UpdateTodoUseCaseImpl,
    },
    uow::UnitOfWorkFactory,
};
use axum::Router;
use infrastructure::sqlite::{self, SqliteUnitOfWorkFactory};
use presentation::todo::TodoUseCases;
use std::{error::Error, sync::Arc};

/// Build the HTTP router using the provided [`UnitOfWorkFactory`].
pub fn build_router(uow_factory: Arc<dyn UnitOfWorkFactory>) -> Router {
    let todo_usecases = TodoUseCases {
        create: Arc::new(CreateTodoUseCaseImpl::new(Arc::clone(&uow_factory))),
        find_by_id: Arc::new(FindTodoByIdUseCaseImpl::new(Arc::clone(&uow_factory))),
        find_todos: Arc::new(FindTodosUseCaseImpl::new(Arc::clone(&uow_factory))),
        update: Arc::new(UpdateTodoUseCaseImpl::new(Arc::clone(&uow_factory))),
        start: Arc::new(StartTodoUseCaseImpl::new(Arc::clone(&uow_factory))),
        complete: Arc::new(CompleteTodoUseCaseImpl::new(Arc::clone(&uow_factory))),
        delete: Arc::new(DeleteTodoUseCaseImpl::new(uow_factory)),
    };

    Router::new().merge(presentation::todo::router(todo_usecases))
}

/// Open SQLite, apply migrations, and construct the wired application router.
pub async fn build_sqlite_app(database_url: &str) -> Result<Router, Box<dyn Error>> {
    let pool = sqlite::connect(database_url).await?;
    let uow_factory = Arc::new(SqliteUnitOfWorkFactory::new(pool));
    Ok(build_router(uow_factory))
}
