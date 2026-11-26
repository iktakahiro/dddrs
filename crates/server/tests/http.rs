use application::TodoService;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use infrastructure::SqliteTodoRepository;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn app(url: &str) -> Router {
    let repository = SqliteTodoRepository::connect(url).await.unwrap();
    presentation::router(TodoService::new(Arc::new(repository)))
}

async fn request(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    expected: StatusCode,
) -> Value {
    let mut req = Request::builder().method(method).uri(path);
    let body = match body {
        Some(value) => {
            req = req.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(status, expected, "{}", String::from_utf8_lossy(&bytes));
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn create(app: &Router, title: &str) -> Value {
    request(
        app,
        Method::POST,
        "/todos",
        Some(json!({"title": title})),
        StatusCode::CREATED,
    )
    .await
}

#[tokio::test]
async fn edits_starts_completes_and_reopens_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("todos.db").display());
    let first = app(&url).await;
    let todo = create(&first, "Buy groceries").await;
    let path = format!("/todos/{}", todo["id"].as_str().unwrap());
    assert_eq!(todo["status"], "not_started");
    assert_eq!(todo["description"], "");
    assert!(todo["completed_at"].is_null());
    assert!(todo["created_at"].is_i64());

    let updated = request(
        &first,
        Method::PUT,
        &path,
        Some(json!({"title":"Buy groceries for dinner", "description":"Pick up vegetables"})),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated["title"], "Buy groceries for dinner");
    assert_eq!(updated["description"], "Pick up vegetables");
    let start = format!("{path}/start");
    let complete = format!("{path}/complete");
    request(
        &first,
        Method::PATCH,
        &complete,
        None,
        StatusCode::BAD_REQUEST,
    )
    .await;
    let started = request(&first, Method::PATCH, &start, None, StatusCode::OK).await;
    assert_eq!(started["status"], "in_progress");
    request(&first, Method::PATCH, &start, None, StatusCode::BAD_REQUEST).await;
    let done = request(&first, Method::PATCH, &complete, None, StatusCode::OK).await;
    assert_eq!(done["status"], "completed");
    assert!(done["completed_at"].is_i64());
    assert_eq!(done["completed_at"], done["updated_at"]);
    assert_eq!(done["created_at"], todo["created_at"]);
    drop(first);

    let reopened = app(&url).await;
    assert_eq!(
        request(&reopened, Method::GET, &path, None, StatusCode::OK).await,
        done
    );
    assert_eq!(
        request(&reopened, Method::GET, "/todos", None, StatusCode::OK).await,
        json!([done])
    );
    for route in [&start, &complete] {
        request(
            &reopened,
            Method::PATCH,
            route,
            None,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    assert!(
        request(
            &reopened,
            Method::DELETE,
            &path,
            None,
            StatusCode::NO_CONTENT
        )
        .await
        .is_null()
    );
    for method in [Method::GET, Method::DELETE] {
        request(&reopened, method, &path, None, StatusCode::NOT_FOUND).await;
    }
}

#[tokio::test]
async fn list_returns_twenty_newest_first() {
    let app = app("sqlite::memory:").await;
    for i in 0..21 {
        create(&app, &i.to_string()).await;
    }
    let result = request(&app, Method::GET, "/todos", None, StatusCode::OK).await;
    let titles: Vec<_> = result
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap())
        .collect();
    assert_eq!(
        titles,
        (1..=20).rev().map(|i| i.to_string()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn invalid_values_do_not_write_and_missing_ids_return_not_found() {
    let app = app("sqlite::memory:").await;
    for input in [
        json!({}),
        json!({"title": 123}),
        json!({"title":""}),
        json!({"title":"é".repeat(101)}),
        json!({"title":"a", "description":"é".repeat(1001)}),
    ] {
        let error = request(
            &app,
            Method::POST,
            "/todos",
            Some(input),
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
        assert!(error["detail"].is_string());
    }
    assert_eq!(
        request(&app, Method::GET, "/todos", None, StatusCode::OK).await,
        json!([])
    );
    request(
        &app,
        Method::GET,
        "/todos/invalid-id",
        None,
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    let missing = "/todos/00000000-0000-4000-8000-000000000000";
    request(
        &app,
        Method::PUT,
        missing,
        Some(json!({"title": "new"})),
        StatusCode::NOT_FOUND,
    )
    .await;
    request(
        &app,
        Method::PATCH,
        &format!("{missing}/start"),
        None,
        StatusCode::NOT_FOUND,
    )
    .await;
}
