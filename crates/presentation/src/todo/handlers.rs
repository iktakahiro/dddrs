use super::{
    extract::{TodoInput, TodoPath},
    schema::TodoResponse,
};
use crate::error::ApiError;
use application::todo::{
    CompleteTodoUseCase, CreateTodoUseCase, DeleteTodoUseCase, FindTodoByIdUseCase,
    FindTodosUseCase, StartTodoUseCase, UpdateTodoUseCase,
};
use axum::{
    Json, Router,
    extract::{FromRef, State},
    http::StatusCode,
    routing::{get, patch},
};
use std::sync::Arc;

/// Container holding the single-responsibility Todo use case ports.
///
/// Each handler extracts only the single use case it depends on via [`FromRef`].
#[derive(Clone)]
pub struct TodoUseCases {
    pub create: Arc<dyn CreateTodoUseCase>,
    pub find_by_id: Arc<dyn FindTodoByIdUseCase>,
    pub find_todos: Arc<dyn FindTodosUseCase>,
    pub update: Arc<dyn UpdateTodoUseCase>,
    pub start: Arc<dyn StartTodoUseCase>,
    pub complete: Arc<dyn CompleteTodoUseCase>,
    pub delete: Arc<dyn DeleteTodoUseCase>,
}

impl FromRef<TodoUseCases> for Arc<dyn CreateTodoUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.create)
    }
}

impl FromRef<TodoUseCases> for Arc<dyn FindTodoByIdUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.find_by_id)
    }
}

impl FromRef<TodoUseCases> for Arc<dyn FindTodosUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.find_todos)
    }
}

impl FromRef<TodoUseCases> for Arc<dyn UpdateTodoUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.update)
    }
}

impl FromRef<TodoUseCases> for Arc<dyn StartTodoUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.start)
    }
}

impl FromRef<TodoUseCases> for Arc<dyn CompleteTodoUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.complete)
    }
}

impl FromRef<TodoUseCases> for Arc<dyn DeleteTodoUseCase> {
    fn from_ref(state: &TodoUseCases) -> Self {
        Arc::clone(&state.delete)
    }
}

pub fn router(usecases: TodoUseCases) -> Router {
    Router::new()
        .route("/todos", get(list).post(create))
        .route("/todos/{id}", get(find).put(update).delete(remove))
        .route("/todos/{id}/start", patch(start))
        .route("/todos/{id}/complete", patch(complete))
        .with_state(usecases)
}

async fn list(
    State(usecase): State<Arc<dyn FindTodosUseCase>>,
) -> Result<Json<Vec<TodoResponse>>, ApiError> {
    Ok(Json(
        usecase
            .execute()
            .await?
            .into_iter()
            .map(TodoResponse::from)
            .collect(),
    ))
}

async fn create(
    State(usecase): State<Arc<dyn CreateTodoUseCase>>,
    TodoInput { title, description }: TodoInput,
) -> Result<(StatusCode, Json<TodoResponse>), ApiError> {
    let todo = usecase.execute(title, description).await?;
    Ok((StatusCode::CREATED, Json(todo.into())))
}

async fn find(
    State(usecase): State<Arc<dyn FindTodoByIdUseCase>>,
    TodoPath(id): TodoPath,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(usecase.execute(id).await?.into()))
}

async fn update(
    State(usecase): State<Arc<dyn UpdateTodoUseCase>>,
    TodoPath(id): TodoPath,
    TodoInput { title, description }: TodoInput,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(usecase.execute(id, title, description).await?.into()))
}

async fn start(
    State(usecase): State<Arc<dyn StartTodoUseCase>>,
    TodoPath(id): TodoPath,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(usecase.execute(id).await?.into()))
}

async fn complete(
    State(usecase): State<Arc<dyn CompleteTodoUseCase>>,
    TodoPath(id): TodoPath,
) -> Result<Json<TodoResponse>, ApiError> {
    Ok(Json(usecase.execute(id).await?.into()))
}

async fn remove(
    State(usecase): State<Arc<dyn DeleteTodoUseCase>>,
    TodoPath(id): TodoPath,
) -> Result<StatusCode, ApiError> {
    usecase.execute(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::todo::UseCaseError;
    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request},
    };
    use domain::todo::{Todo, TodoId, TransitionError};
    use serde_json::Value;
    use tower::ServiceExt;

    struct StubStartUseCase {
        outcome: fn(TodoId) -> Result<Todo, UseCaseError>,
    }

    #[async_trait]
    impl StartTodoUseCase for StubStartUseCase {
        async fn execute(&self, id: TodoId) -> Result<Todo, UseCaseError> {
            (self.outcome)(id)
        }
    }

    async fn call_start(outcome: fn(TodoId) -> Result<Todo, UseCaseError>) -> (StatusCode, Value) {
        let usecase: Arc<dyn StartTodoUseCase> = Arc::new(StubStartUseCase { outcome });
        let app = Router::new()
            .route("/todos/{id}/start", patch(start))
            .with_state(usecase);
        let id = TodoId::generate();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/todos/{id}/start"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn start_handler_maps_use_case_errors_to_http_statuses() {
        let (status, body) = call_start(|_| Err(UseCaseError::NotFound)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["detail"], "Todo not found");

        let (status, body) =
            call_start(|_| Err(UseCaseError::Transition(TransitionError::AlreadyStarted))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["detail"], "Todo has already been started");

        let (status, body) = call_start(|_| Err(UseCaseError::Conflict)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["detail"], "Todo changed since it was read");

        let (status, body) = call_start(|_| Err(UseCaseError::Unexpected("db down".into()))).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["detail"], "Internal server error");
    }
}
