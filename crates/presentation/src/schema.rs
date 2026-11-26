use domain::todo::{Description, StoredTodo};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TodoResponse {
    id: String,
    title: String,
    description: String,
    status: &'static str,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>,
}

impl From<StoredTodo> for TodoResponse {
    fn from(todo: StoredTodo) -> Self {
        let fields = todo.details();
        Self {
            id: fields.id.to_string(),
            title: fields.title.as_str().into(),
            description: fields
                .description
                .as_ref()
                .map(Description::as_str)
                .unwrap_or_default()
                .into(),
            status: todo.status(),
            created_at: fields.created_at,
            updated_at: fields.updated_at,
            completed_at: todo.completed_at(),
        }
    }
}
