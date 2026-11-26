//! Types and operations for the Todo lifecycle.

mod repository;
mod values;

pub use repository::{RepositoryError, Snapshot, TodoRepository};
pub use values::{Description, Title, TodoId, ValidationError};

use chrono::Utc;
use thiserror::Error;

/// The fields shared by all lifecycle states of a Todo.
#[derive(Debug)]
pub struct Details {
    pub id: TodoId,
    pub title: Title,
    pub description: Option<Description>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug)]
pub struct NotStarted;
#[derive(Debug)]
pub struct InProgress;
#[derive(Debug)]
pub struct Completed {
    pub at: i64,
}

#[derive(Debug)]
pub struct Todo<S> {
    details: Details,
    state: S,
}

impl<S> Todo<S> {
    pub fn restore(details: Details, state: S) -> Self {
        Self { details, state }
    }
}

impl Todo<NotStarted> {
    pub fn new(title: Title, description: Option<Description>) -> Self {
        let now = Utc::now().timestamp_millis();
        Self::restore(
            Details {
                id: TodoId::generate(),
                title,
                description,
                created_at: now,
                updated_at: now,
            },
            NotStarted,
        )
    }

    /// Consume the pending Todo so the same value cannot be started twice.
    ///
    /// ```compile_fail,E0382
    /// use domain::todo::{Title, Todo};
    /// let todo = Todo::new(Title::try_from(String::from("Buy groceries")).unwrap(), None);
    /// let started = todo.start();
    /// todo.start();
    /// ```
    pub fn start(mut self) -> Todo<InProgress> {
        self.details.updated_at = Utc::now().timestamp_millis();
        Todo::restore(self.details, InProgress)
    }
}

impl Todo<InProgress> {
    /// Complete an in-progress Todo. A pending Todo has no such method.
    ///
    /// ```compile_fail,E0599
    /// use domain::todo::{Title, Todo};
    /// let todo = Todo::new(Title::try_from(String::from("Buy groceries")).unwrap(), None);
    /// todo.complete();
    /// ```
    pub fn complete(mut self) -> Todo<Completed> {
        let at = Utc::now().timestamp_millis();
        self.details.updated_at = at;
        Todo::restore(self.details, Completed { at })
    }
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum TransitionError {
    #[error("Todo has not been started")]
    NotStarted,
    #[error("Todo has already been started")]
    AlreadyStarted,
    #[error("Todo has already been completed")]
    AlreadyCompleted,
}

/// A Todo whose state is not known until it is loaded.
#[derive(Debug)]
pub enum StoredTodo {
    NotStarted(Todo<NotStarted>),
    InProgress(Todo<InProgress>),
    Completed(Todo<Completed>),
}

impl StoredTodo {
    pub fn is_overdue(&self, deadline_ms: i64, now_ms: i64) -> bool {
        !matches!(self, Self::Completed(_)) && now_ms > deadline_ms
    }

    pub fn details(&self) -> &Details {
        match self {
            Self::NotStarted(todo) => &todo.details,
            Self::InProgress(todo) => &todo.details,
            Self::Completed(todo) => &todo.details,
        }
    }

    pub fn edit(&mut self, title: Title, description: Option<Description>) {
        let details = match self {
            Self::NotStarted(todo) => &mut todo.details,
            Self::InProgress(todo) => &mut todo.details,
            Self::Completed(todo) => &mut todo.details,
        };
        details.title = title;
        details.description = description;
        details.updated_at = Utc::now().timestamp_millis();
    }

    pub fn start(self) -> Result<Self, TransitionError> {
        match self {
            Self::NotStarted(todo) => Ok(Self::InProgress(todo.start())),
            Self::InProgress(_) => Err(TransitionError::AlreadyStarted),
            Self::Completed(_) => Err(TransitionError::AlreadyCompleted),
        }
    }

    pub fn complete(self) -> Result<Self, TransitionError> {
        match self {
            Self::NotStarted(_) => Err(TransitionError::NotStarted),
            Self::InProgress(todo) => Ok(Self::Completed(todo.complete())),
            Self::Completed(_) => Err(TransitionError::AlreadyCompleted),
        }
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::NotStarted(_) => "not_started",
            Self::InProgress(_) => "in_progress",
            Self::Completed(_) => "completed",
        }
    }

    pub fn completed_at(&self) -> Option<i64> {
        match self {
            Self::Completed(todo) => Some(todo.state.at),
            _ => None,
        }
    }
}

impl PartialEq for StoredTodo {
    fn eq(&self, other: &Self) -> bool {
        self.details().id == other.details().id
    }
}

impl Eq for StoredTodo {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unfinished_todos_can_be_overdue() {
        let todo = StoredTodo::NotStarted(Todo::new("read".to_owned().try_into().unwrap(), None));
        assert!(!todo.is_overdue(10, 10));
        assert!(todo.is_overdue(10, 11));
        let active = todo.start().unwrap();
        assert!(active.is_overdue(10, 11));
        assert!(!active.complete().unwrap().is_overdue(10, 11));
    }
}
