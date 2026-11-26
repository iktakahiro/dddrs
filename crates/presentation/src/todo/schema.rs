use domain::todo::{Description, Todo, TodoStatus};
use serde::Serialize;

fn status_str(status: TodoStatus) -> &'static str {
    match status {
        TodoStatus::NotStarted => "not_started",
        TodoStatus::InProgress => "in_progress",
        TodoStatus::Completed => "completed",
    }
}

#[derive(Serialize)]
pub(super) struct TodoResponse {
    id: String,
    title: String,
    description: String,
    status: &'static str,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>,
}

impl From<Todo> for TodoResponse {
    fn from(todo: Todo) -> Self {
        Self {
            id: todo.id().to_string(),
            title: todo.title().as_str().into(),
            description: todo
                .description()
                .map(Description::as_str)
                .unwrap_or_default()
                .into(),
            status: status_str(todo.status()),
            created_at: todo.created_at().timestamp_millis(),
            updated_at: todo.updated_at().timestamp_millis(),
            completed_at: todo.completed_at().map(|at| at.timestamp_millis()),
        }
    }
}
