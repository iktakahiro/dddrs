use crate::sqlite::storage;
use chrono::{DateTime, Utc};
use domain::{
    error::RepositoryError,
    todo::{Description, Snapshot, Title, Todo, TodoState},
};

#[derive(sqlx::FromRow)]
pub(super) struct TodoRow {
    id: String,
    title: String,
    description: Option<String>,
    status: String,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>,
    version: i64,
}

fn timestamp_from_millis(millis: i64) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::from_timestamp_millis(millis)
        .ok_or_else(|| RepositoryError::Storage("Invalid stored Todo timestamp".into()))
}

impl TryFrom<TodoRow> for Snapshot {
    type Error = RepositoryError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let id = row.id.parse().map_err(storage)?;
        let title = Title::try_from(row.title).map_err(storage)?;
        let description = Description::from_optional(row.description).map_err(storage)?;
        let created_at = timestamp_from_millis(row.created_at)?;
        let updated_at = timestamp_from_millis(row.updated_at)?;

        let state = match (row.status.as_str(), row.completed_at) {
            ("not_started", None) => TodoState::NotStarted,
            ("in_progress", None) => TodoState::InProgress,
            ("completed", Some(at)) => TodoState::Completed {
                completed_at: timestamp_from_millis(at)?,
            },
            _ => return Err(RepositoryError::Storage("Invalid stored Todo state".into())),
        };

        let todo = Todo::reconstitute(id, title, description, state, created_at, updated_at);
        Ok(Snapshot {
            todo,
            version: row.version,
        })
    }
}
