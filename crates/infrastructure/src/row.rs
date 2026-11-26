use crate::storage;
use domain::{
    Completed, Description, Details, InProgress, NotStarted, RepositoryError, Snapshot, StoredTodo,
    Title, Todo,
};

#[derive(sqlx::FromRow)]
pub(crate) struct TodoRow {
    id: String,
    title: String,
    description: Option<String>,
    status: String,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>,
    version: i64,
}

impl TryFrom<TodoRow> for Snapshot {
    type Error = RepositoryError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let details = Details {
            id: row.id.parse().map_err(storage)?,
            title: Title::try_from(row.title).map_err(storage)?,
            description: row
                .description
                .map(Description::try_from)
                .transpose()
                .map_err(storage)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        let todo = match (row.status.as_str(), row.completed_at) {
            ("not_started", None) => StoredTodo::NotStarted(Todo::restore(details, NotStarted)),
            ("in_progress", None) => StoredTodo::InProgress(Todo::restore(details, InProgress)),
            ("completed", Some(at)) => {
                StoredTodo::Completed(Todo::restore(details, Completed { at }))
            }
            _ => return Err(RepositoryError::Storage("Invalid stored Todo state".into())),
        };
        Ok(Snapshot {
            todo,
            version: row.version,
        })
    }
}
