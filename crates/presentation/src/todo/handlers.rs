use super::{
    extract::{TodoInput, TodoPath},
    schema::TodoResponse,
};
use crate::error::ApiError;
use application::todo::TodoService;
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};

pub fn router(service: TodoService) -> Router {
    Router::new()
        .route("/todos", get(list).post(create))
        .route("/todos/{id}", get(find).put(edit).delete(remove))
        .route("/todos/{id}/start", axum::routing::patch(start))
        .route("/todos/{id}/complete", axum::routing::patch(complete))
        .with_state(service)
}

async fn list(State(service): State<TodoService>) -> Result<Json<Vec<TodoResponse>>, ApiError> {
    Ok(Json(
        service
            .latest()
            .await?
            .into_iter()
            .map(TodoResponse::from)
            .collect(),
    ))
}

async fn create(
    State(service): State<TodoService>,
    TodoInput { title, description }: TodoInput,
) -> Result<(StatusCode, Json<TodoResponse>), ApiError> {
    let todo = service.create(title, description).await?;
    Ok((StatusCode::CREATED, Json(todo.into())))
}

async fn find(
    State(service): State<TodoService>,
    TodoPath(id): TodoPath,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(service.get(id).await?.into()))
}

async fn edit(
    State(service): State<TodoService>,
    TodoPath(id): TodoPath,
    TodoInput { title, description }: TodoInput,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(service.edit(id, title, description).await?.into()))
}

async fn start(
    State(service): State<TodoService>,
    TodoPath(id): TodoPath,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(service.start(id).await?.into()))
}

async fn complete(
    State(service): State<TodoService>,
    TodoPath(id): TodoPath,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(service.complete(id).await?.into()))
}

async fn remove(
    State(service): State<TodoService>,
    TodoPath(id): TodoPath,
) -> Result<StatusCode, ApiError> {
    service.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
